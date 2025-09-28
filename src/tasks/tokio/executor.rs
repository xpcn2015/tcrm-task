use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{
    process::ChildStdin,
    sync::{Mutex, mpsc, oneshot},
};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;
use crate::tasks::{
    config::TaskConfig,
    control::{TaskControl, TaskControlAction, TaskStatusInfo},
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    process::{child::terminate_process, process_group::ProcessGroup},
    state::TaskState,
};

#[derive(Debug)]
pub struct TaskExecutor {
    pub(crate) config: TaskConfig,
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) process_id: Arc<AtomicU32>,
    pub(crate) created_at: Arc<AtomicU64>,
    pub(crate) running_at: Arc<AtomicU64>,
    pub(crate) finished_at: Arc<AtomicU64>,
    pub(crate) exit_code: Arc<AtomicI32>,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) terminate_tx: Option<oneshot::Sender<TaskTerminateReason>>,
    pub(crate) internal_terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    pub(crate) stop_reason: Arc<Mutex<Option<TaskStopReason>>>,
    pub(crate) ready_flag: Arc<AtomicBool>,

    #[cfg(feature = "process-group")]
    pub(crate) group: ProcessGroup,
}
impl TaskExecutor {
    pub fn new(config: TaskConfig) -> Self {
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            config,
            state: Arc::new(AtomicU8::new(TaskState::Pending as u8)),
            process_id: Arc::new(AtomicU32::new(0)),
            created_at: Arc::new(AtomicU64::new(now_millis)),
            running_at: Arc::new(AtomicU64::new(0)),
            finished_at: Arc::new(AtomicU64::new(0)),
            exit_code: Arc::new(AtomicI32::new(0)),
            stdin: None,
            terminate_tx: None,
            internal_terminate_tx: Arc::new(Mutex::new(None)),
            stop_reason: Arc::new(Mutex::new(None)),
            group: ProcessGroup::new(),
            ready_flag: Arc::new(AtomicBool::new(false)),
        }
    }
    pub(crate) fn set_state(
        state_store: Arc<AtomicU8>,
        new_state: TaskState,
        time_store: Option<Arc<AtomicU64>>,
    ) -> SystemTime {
        state_store.store(new_state as u8, Ordering::SeqCst);
        let now = SystemTime::now();
        if let Some(time_store) = time_store {
            match new_state {
                TaskState::Running | TaskState::Finished => {
                    Self::set_time(time_store, now);
                }
                _ => {}
            }
        }
        now
    }
    pub(crate) fn set_time(time_store: Arc<AtomicU64>, time: SystemTime) {
        let millis = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        time_store.store(millis, Ordering::SeqCst);
    }

    pub(crate) async fn send_event(event_tx: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
        if (event_tx.send(event.clone()).await).is_err() {
            #[cfg(feature = "tracing")]
            tracing::warn!(event = ?event, "Event channel closed");
        }
    }
    pub(crate) async fn send_error_event_and_stop(
        &mut self,
        error: TaskError,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) {
        let time = Self::set_state(
            self.state.clone(),
            TaskState::Finished,
            Some(self.finished_at.clone()),
        );
        let error_event = TaskEvent::Error {
            error: error.clone(),
        };
        Self::send_event(event_tx, error_event).await;

        let finish_event = TaskEvent::Stopped {
            exit_code: None,
            finished_at: time,
            reason: TaskStopReason::Error(error.clone()),
        };
        Self::send_event(event_tx, finish_event).await;

        *self.stop_reason.lock().await = Some(TaskStopReason::Error(error));
    }
    pub(crate) async fn internal_terminate(
        internal_terminate_tx: &Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
        reason: TaskTerminateReason,
    ) {
        if let Some(tx) = internal_terminate_tx.lock().await.take() {
            if tx.send(reason.clone()).is_err() {
                let msg = "Terminate channel closed while sending signal";
                #[cfg(feature = "tracing")]
                tracing::warn!(terminate_reason=?reason, msg);

                return;
            }
        } else {
            let msg = "Terminate signal already sent or channel missing";
            #[cfg(feature = "tracing")]
            tracing::warn!(msg);
            return;
        }
    }
}

impl TaskControl for TaskExecutor {
    fn terminate_task(&mut self, reason: TaskTerminateReason) -> Result<(), TaskError> {
        let current_state = TaskState::from(self.state.load(Ordering::SeqCst));
        if current_state == TaskState::Finished {
            return Err(TaskError::Control("Task already finished".to_string()));
        }
        if let Some(tx) = self.terminate_tx.take() {
            if tx.send(reason.clone()).is_err() {
                let msg = "Terminate channel closed while sending signal";
                #[cfg(feature = "tracing")]
                tracing::warn!(terminate_reason=?reason, msg);
                return Err(TaskError::Channel(msg.to_string()));
            }
        } else {
            let msg = "Terminate signal already sent or channel missing";
            #[cfg(feature = "tracing")]
            tracing::warn!(msg);
            return Err(TaskError::Channel(msg.to_string()));
        }

        Ok(())
    }

    fn perform_process_action(&mut self, action: TaskControlAction) -> Result<(), TaskError> {
        #[cfg(feature = "process-group")]
        let use_process_group = self.config.use_process_group.unwrap_or_default();
        #[cfg(not(feature = "process-group"))]
        let use_process_group = false;

        #[cfg(feature = "process-group")]
        let active = self.group.is_active();
        #[cfg(not(feature = "process-group"))]
        let active = false;
        let process_id = match self.process_id.load(Ordering::SeqCst) {
            0 => {
                let msg = "No process ID available to perform action";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                return Err(TaskError::Control(msg.to_string()));
            }
            n => n,
        };
        match action {
            TaskControlAction::Terminate => {
                if use_process_group && active {
                    self.group.terminate_group().map_err(|e| {
                        let msg = format!("Failed to terminate process group: {}", e);
                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);
                        TaskError::Control(msg)
                    })?;
                } else {
                    terminate_process(process_id).map_err(|e| {
                        let msg = format!("Failed to terminate process: {}", e);
                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);
                        TaskError::Control(msg)
                    })?;
                }
            }
            TaskControlAction::Pause => todo!(),
            TaskControlAction::Resume => todo!(),
            TaskControlAction::Interrupt => todo!(),
        }
        Ok(())
    }

    #[cfg(feature = "signal")]
    fn send_signal(&self, signal: ProcessSignal) -> Result<(), TaskError> {
        todo!()
    }
}

impl TaskStatusInfo for TaskExecutor {
    fn get_state(&self) -> TaskState {
        self.state.load(Ordering::SeqCst).into()
    }

    fn get_process_id(&self) -> Option<u32> {
        match self.process_id.load(Ordering::SeqCst) {
            0 => None,
            n => Some(n),
        }
    }

    fn get_create_at(&self) -> SystemTime {
        let millis = self.created_at.load(Ordering::SeqCst);
        UNIX_EPOCH + std::time::Duration::from_millis(millis)
    }

    fn get_running_at(&self) -> Option<SystemTime> {
        let millis = self.running_at.load(Ordering::SeqCst);
        match millis {
            0 => None,
            n => Some(UNIX_EPOCH + std::time::Duration::from_millis(n)),
        }
    }

    fn get_finished_at(&self) -> Option<SystemTime> {
        let millis = self.finished_at.load(Ordering::SeqCst);
        match millis {
            0 => None,
            n => Some(UNIX_EPOCH + std::time::Duration::from_millis(n)),
        }
    }
    fn get_exit_code(&self) -> Option<i32> {
        let state = self.get_state();
        if state != TaskState::Finished {
            return None;
        }
        Some(self.exit_code.load(Ordering::SeqCst))
    }
}

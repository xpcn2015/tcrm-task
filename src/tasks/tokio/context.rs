use std::{
    sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicU64},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::{Mutex, oneshot};

#[cfg(feature = "process-group")]
use crate::tasks::process::process_group::ProcessGroup;
use crate::tasks::{
    config::TaskConfig,
    event::{TaskStopReason, TaskTerminateReason},
    state::TaskState,
};

#[derive(Debug)]
pub(crate) struct TaskExecutorContext {
    pub(crate) config: TaskConfig,
    state: AtomicU8,
    process_id: AtomicU32,
    created_at: AtomicU64,
    running_at: AtomicU64,
    finished_at: AtomicU64,
    exit_code: AtomicI32,
    stop_reason: Mutex<Option<TaskStopReason>>,
    ready_flag: AtomicBool,
    internal_terminate_tx: Mutex<Option<oneshot::Sender<TaskTerminateReason>>>,

    #[cfg(feature = "process-group")]
    pub(crate) group: Mutex<ProcessGroup>,
}
impl TaskExecutorContext {
    pub(crate) fn new(config: TaskConfig) -> Self {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self {
            config,
            state: AtomicU8::new(0),
            process_id: AtomicU32::new(0),
            created_at: AtomicU64::new(now_nanos),
            running_at: AtomicU64::new(0),
            finished_at: AtomicU64::new(0),
            exit_code: AtomicI32::new(-1),
            stop_reason: Mutex::new(None),
            ready_flag: AtomicBool::new(false),
            internal_terminate_tx: Mutex::new(None),

            #[cfg(feature = "process-group")]
            group: Mutex::new(ProcessGroup::new()),
        }
    }
    pub(crate) async fn send_terminate_signal(&self, reason: TaskTerminateReason) {
        let mut guard = self.internal_terminate_tx.lock().await;
        if let Some(tx) = guard.take() {
            if tx.send(reason.clone()).is_err() {
                #[cfg(feature = "tracing")]
                tracing::warn!(terminate_reason=?reason, "Internal Terminate channel closed while sending signal");
            }
        } else {
            #[cfg(feature = "tracing")]
            tracing::warn!("Terminate signal already sent or channel missing");
        }
    }
    pub(crate) async fn set_internal_terminate_tx(&self, tx: oneshot::Sender<TaskTerminateReason>) {
        let mut guard = self.internal_terminate_tx.lock().await;
        *guard = Some(tx);
    }

    pub(crate) fn get_ready_flag(&self) -> bool {
        self.ready_flag.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub(crate) fn set_ready_flag(&self, ready: bool) {
        self.ready_flag
            .store(ready, std::sync::atomic::Ordering::SeqCst);
    }

    pub(crate) async fn get_stop_reason(&self) -> Option<TaskStopReason> {
        self.stop_reason.lock().await.clone()
    }
    pub(crate) async fn set_stop_reason(&self, reason: TaskStopReason) {
        let mut guard = self.stop_reason.lock().await;
        *guard = Some(reason);
    }

    pub(crate) fn get_exit_code(&self) -> Option<i32> {
        let state = self.get_state();
        if state != TaskState::Finished {
            return None;
        }
        let code = self.exit_code.load(std::sync::atomic::Ordering::SeqCst);
        if code == -1 { None } else { Some(code) }
    }
    pub(crate) fn set_exit_code(&self, code: Option<i32>) {
        self.exit_code
            .store(code.unwrap_or(-1), std::sync::atomic::Ordering::SeqCst);
    }

    pub(crate) fn get_running_at(&self) -> Option<SystemTime> {
        Self::get_time(&self.running_at)
    }
    pub(crate) fn set_running_at(&self) -> SystemTime {
        Self::set_time(&self.running_at)
    }

    pub(crate) fn get_finished_at(&self) -> Option<SystemTime> {
        Self::get_time(&self.finished_at)
    }
    pub(crate) fn set_finished_at(&self) -> SystemTime {
        Self::set_time(&self.finished_at)
    }

    pub(crate) fn get_create_at(&self) -> SystemTime {
        let nanos = self.created_at.load(std::sync::atomic::Ordering::SeqCst);
        UNIX_EPOCH + std::time::Duration::from_nanos(nanos)
    }

    fn get_time(store: &AtomicU64) -> Option<SystemTime> {
        let nanos = store.load(std::sync::atomic::Ordering::SeqCst);
        if nanos == 0 {
            None
        } else {
            Some(UNIX_EPOCH + std::time::Duration::from_nanos(nanos))
        }
    }
    fn set_time(store: &AtomicU64) -> SystemTime {
        let now = SystemTime::now();
        let nanos = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        store.store(nanos, std::sync::atomic::Ordering::SeqCst);
        now
    }
    pub(crate) fn get_process_id(&self) -> Option<u32> {
        let process_id = self.process_id.load(std::sync::atomic::Ordering::SeqCst);
        if process_id == 0 {
            None
        } else {
            Some(process_id)
        }
    }
    pub(crate) fn set_process_id(&self, pid: u32) {
        self.process_id
            .store(pid, std::sync::atomic::Ordering::SeqCst);
    }

    pub(crate) fn get_state(&self) -> TaskState {
        self.state.load(std::sync::atomic::Ordering::SeqCst).into()
    }
    pub(crate) fn set_state(&self, new_state: TaskState) -> SystemTime {
        self.state
            .store(new_state as u8, std::sync::atomic::Ordering::SeqCst);
        let now = SystemTime::now();
        let nanos = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        match new_state {
            TaskState::Running => {
                self.running_at
                    .store(nanos, std::sync::atomic::Ordering::SeqCst);
            }
            TaskState::Finished => {
                self.finished_at
                    .store(nanos, std::sync::atomic::Ordering::SeqCst);
            }
            _ => {}
        }
        now
    }
}

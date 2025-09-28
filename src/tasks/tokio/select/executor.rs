use std::time::Instant;

use tokio::{process::ChildStdin, sync::oneshot};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;
use crate::tasks::{
    config::TaskConfig,
    control::{TaskControl, TaskControlAction, TaskInformation, TaskInternal},
    error::TaskError,
    event::{TaskStopReason, TaskTerminateReason},
    process::{child::terminate_process, process_group::ProcessGroup},
    state::TaskState,
};

#[derive(Debug)]
pub(crate) struct TaskExecutorFlags {
    pub(crate) stop: bool,
    pub(crate) ready: bool,
}
#[derive(Debug)]
pub struct TaskExecutor {
    pub(crate) config: TaskConfig,
    #[cfg(feature = "process-group")]
    pub(crate) group: ProcessGroup,
    pub(crate) state: TaskState,
    pub(crate) process_id: Option<u32>,
    pub(crate) created_at: Instant,
    pub(crate) running_at: Option<Instant>,
    pub(crate) finished_at: Option<Instant>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stop_reason: Option<TaskStopReason>,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) terminate_tx: Option<oneshot::Sender<TaskTerminateReason>>,
    pub(crate) flags: TaskExecutorFlags,
}
impl TaskExecutor {
    pub fn new(config: TaskConfig) -> Self {
        Self {
            config,
            group: ProcessGroup::new(),
            state: TaskState::Pending,
            process_id: None,
            created_at: Instant::now(),
            running_at: None,
            finished_at: None,
            exit_code: None,
            stop_reason: None,
            stdin: None,
            terminate_tx: None,
            flags: TaskExecutorFlags {
                stop: false,
                ready: false,
            },
        }
    }
}
impl TaskInternal for TaskExecutor {
    fn set_state(&mut self, new_state: TaskState) {
        self.state = new_state;
    }
}

impl TaskControl for TaskExecutor {
    fn terminate_task(&mut self, reason: TaskTerminateReason) -> Result<(), TaskError> {
        if self.state == TaskState::Finished {
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
        let use_process_group = self.config.use_process_group.unwrap_or_default();

        #[cfg(feature = "process-group")]
        let active = self.group.is_active();
        #[cfg(not(feature = "process-group"))]
        let active = false;

        let process_id = match self.process_id {
            Some(pid) => pid,
            None => {
                let msg = "No process ID available to perform action";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                return Err(TaskError::Control(msg.to_string()));
            }
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

impl TaskInformation for TaskExecutor {
    fn get_config(&self) -> &TaskConfig {
        &self.config
    }

    fn get_state(&self) -> &TaskState {
        &self.state
    }

    fn get_process_id(&self) -> &Option<u32> {
        &self.process_id
    }

    fn get_create_at(&self) -> &Instant {
        &self.created_at
    }

    fn get_running_at(&self) -> &Option<Instant> {
        &self.running_at
    }

    fn get_finished_at(&self) -> &Option<Instant> {
        &self.finished_at
    }
    fn get_exit_code(&self) -> &Option<i32> {
        &self.exit_code
    }
    fn get_stop_reason(&self) -> &Option<TaskStopReason> {
        &self.stop_reason
    }
}

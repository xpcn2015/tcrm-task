use std::time::SystemTime;

use crate::tasks::{
    control::{TaskControl, TaskStatusInfo},
    error::TaskError,
    event::{TaskEvent, TaskTerminateReason},
    process::{
        action::stop::stop_process,
        control::{ProcessControl, ProcessControlAction},
    },
    state::{ProcessState, TaskState},
    tokio::executor::TaskExecutor,
};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;

impl TaskControl for TaskExecutor {
    fn terminate_task(&mut self, reason: TaskTerminateReason) -> Result<(), TaskError> {
        let current_state = self.get_task_state();
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

    #[cfg(feature = "signal")]
    fn send_signal(&self, _signal: ProcessSignal) -> Result<(), TaskError> {
        todo!()
    }
}

impl TaskStatusInfo for TaskExecutor {
    fn get_task_state(&self) -> TaskState {
        self.shared_context.get_task_state()
    }

    fn get_process_state(&self) -> ProcessState {
        self.shared_context.get_process_state()
    }

    fn get_process_id(&self) -> Option<u32> {
        self.shared_context.get_process_id()
    }

    fn get_create_at(&self) -> SystemTime {
        self.shared_context.get_create_at()
    }

    fn get_running_at(&self) -> Option<SystemTime> {
        self.shared_context.get_running_at()
    }

    fn get_finished_at(&self) -> Option<SystemTime> {
        self.shared_context.get_finished_at()
    }

    fn get_exit_code(&self) -> Option<i32> {
        self.shared_context.get_exit_code()
    }
}
impl ProcessControl for TaskExecutor {
    async fn perform_process_action(
        &mut self,
        action: ProcessControlAction,
    ) -> Result<(), TaskError> {
        #[cfg(feature = "process-group")]
        let use_process_group = self
            .shared_context
            .config
            .use_process_group
            .unwrap_or_default();
        #[cfg(not(feature = "process-group"))]
        let use_process_group = false;

        #[cfg(feature = "process-group")]
        let active = self.shared_context.group.lock().await.is_active();
        #[cfg(not(feature = "process-group"))]
        let active = false;
        let process_id = match self.shared_context.get_process_id() {
            Some(n) => n,
            None => {
                let msg = "No process ID available to perform action";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                return Err(TaskError::Control(msg.to_string()));
            }
        };
        match action {
            ProcessControlAction::Stop => {
                if use_process_group && active {
                    self.shared_context
                        .group
                        .lock()
                        .await
                        .stop_group()
                        .map_err(|e| {
                            let msg = format!("Failed to terminate process group: {}", e);
                            #[cfg(feature = "tracing")]
                            tracing::error!(error=%e, "{}", msg);
                            TaskError::Control(msg)
                        })?;
                } else {
                    stop_process(process_id).map_err(|e| {
                        let msg = format!("Failed to terminate process: {}", e);
                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);
                        TaskError::Control(msg)
                    })?;
                }
            }
            ProcessControlAction::Pause => {
                if use_process_group && active {
                    self.shared_context
                        .group
                        .lock()
                        .await
                        .pause_group()
                        .map_err(|e| {
                            let msg = format!("Failed to pause process group: {}", e);
                            #[cfg(feature = "tracing")]
                            tracing::error!(error=%e, "{}", msg);
                            TaskError::Control(msg)
                        })?;
                } else {
                    use crate::tasks::process::action::pause::pause_process;
                    pause_process(process_id).map_err(|e| {
                        let msg = format!("Failed to pause process: {}", e);
                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);
                        TaskError::Control(msg)
                    })?;
                }
            }
            ProcessControlAction::Resume => {
                if use_process_group && active {
                    self.shared_context
                        .group
                        .lock()
                        .await
                        .resume_group()
                        .map_err(|e| {
                            let msg = format!("Failed to resume process group: {}", e);
                            #[cfg(feature = "tracing")]
                            tracing::error!(error=%e, "{}", msg);
                            TaskError::Control(msg)
                        })?;
                } else {
                    use crate::tasks::process::action::resume::resume_process;
                    resume_process(process_id).map_err(|e| {
                        let msg = format!("Failed to resume process: {}", e);
                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);
                        TaskError::Control(msg)
                    })?;
                }
            }
        }
        self.event_tx
            .send(TaskEvent::ProcessControl { action })
            .await
            .map_err(|e| {
                let msg = format!("Failed to send ProcessControl event: {}", e);
                #[cfg(feature = "tracing")]
                tracing::error!(error=%e, "{}", msg);
                TaskError::Channel(msg)
            })?;
        Ok(())
    }
}

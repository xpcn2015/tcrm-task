use std::{process::Stdio, sync::Arc};

use tokio::{
    process::{ChildStdin, Command},
    sync::{mpsc, oneshot},
};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;
use crate::tasks::{
    config::TaskConfig,
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    state::TaskState,
    tokio::context::TaskExecutorContext,
};

#[derive(Debug)]
pub struct TaskExecutor {
    pub(crate) shared_context: Arc<TaskExecutorContext>,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) terminate_tx: Option<oneshot::Sender<TaskTerminateReason>>,
}
impl TaskExecutor {
    pub fn new(config: TaskConfig) -> Self {
        Self {
            shared_context: Arc::new(TaskExecutorContext::new(config)),
            stdin: None,
            terminate_tx: None,
        }
    }

    pub(crate) async fn validate_config(
        &mut self,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        match self.shared_context.config.validate() {
            Ok(()) => Ok(()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Invalid task configuration");

                let time = Self::update_state(&self.shared_context, TaskState::Finished);
                let error_event = TaskEvent::Error { error: e.clone() };
                Self::send_event(event_tx, error_event).await;

                let finish_event = TaskEvent::Stopped {
                    exit_code: None,
                    finished_at: time,
                    reason: TaskStopReason::Error(e.clone()),
                };
                Self::send_event(event_tx, finish_event).await;

                return Err(e);
            }
        }
    }
    pub(crate) fn setup_command(&self) -> Command {
        let mut cmd = Command::new(&self.shared_context.config.command);

        cmd.kill_on_drop(true);

        // Setup additional arguments
        if let Some(args) = &self.shared_context.config.args {
            cmd.args(args);
        }

        // Setup working directory with validation
        if let Some(dir) = &self.shared_context.config.working_dir {
            cmd.current_dir(dir);
        }

        // Setup environment variables
        if let Some(envs) = &self.shared_context.config.env {
            cmd.envs(envs);
        }

        // Setup stdio
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(
            if self.shared_context.config.enable_stdin.unwrap_or_default() {
                Stdio::piped()
            } else {
                Stdio::null()
            },
        );
        cmd
    }
}

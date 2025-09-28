use tokio::{io::AsyncWriteExt, process::Child, sync::mpsc};

use crate::tasks::{
    control::TaskStatusInfo, error::TaskError, event::TaskEvent, state::TaskState,
    tokio::executor::TaskExecutor,
};
impl TaskExecutor {
    pub async fn send_stdin(&mut self, input: impl Into<String>) -> Result<(), TaskError> {
        let state = self.get_state();
        if !matches!(state, TaskState::Running | TaskState::Ready) {
            return Err(TaskError::Control(
                "Cannot send stdin, task is not running".to_string(),
            ));
        }
        if let Some(stdin) = &mut self.stdin.as_mut() {
            #[allow(clippy::used_underscore_binding)]
            if let Err(_e) = stdin.write_all(input.into().as_bytes()).await {
                let msg = "Failed to write to child stdin";
                #[cfg(feature = "tracing")]
                tracing::warn!(error=%_e, msg);
                return Err(TaskError::Control(msg.to_string()));
            }
        } else {
            let msg = "Stdin is not available";
            #[cfg(feature = "tracing")]
            tracing::warn!(msg);
            return Err(TaskError::Control(msg.to_string()));
        }

        Ok(())
    }
    pub(crate) async fn store_stdin(
        &mut self,
        child: &mut Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        if self.config.enable_stdin.unwrap_or_default() {
            return Ok(());
        }
        if let Some(stdin) = child.stdin.take() {
            self.stdin = Some(stdin);
            Ok(())
        } else {
            let msg = "Failed to take stdin of child process";
            #[cfg(feature = "tracing")]
            tracing::error!(msg);

            let error = TaskError::IO(msg.to_string());
            self.send_error_event_and_stop(error.clone(), event_tx)
                .await;

            Err(TaskError::IO(msg.to_string()))
        }
    }
}

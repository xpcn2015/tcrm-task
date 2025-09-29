use tokio::sync::{mpsc, oneshot};

use crate::tasks::{
    config::StreamSource,
    error::TaskError,
    event::{TaskEvent, TaskTerminateReason},
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub async fn coordinate_start(
        &mut self,
        event_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        Self::update_state(&self.shared_context, TaskState::Initiating);
        self.validate_config(&event_tx).await?;

        let cmd = self.setup_command();

        #[cfg(feature = "process-group")]
        let cmd = self.setup_process_group(cmd).await?;

        let mut child = self.spawn_child(cmd, &event_tx).await?;
        self.store_stdin(&mut child, &event_tx).await?;

        let (mut stdout, mut stderr) = self.take_std_output_reader(&mut child, &event_tx).await?;
        let (terminate_tx, mut terminate_rx) = oneshot::channel::<TaskTerminateReason>();
        self.terminate_tx = Some(terminate_tx);

        let (internal_terminate_tx, mut internal_terminate_rx) =
            oneshot::channel::<TaskTerminateReason>();
        self.shared_context
            .set_internal_terminate_tx(internal_terminate_tx)
            .await;

        let shared_context = self.shared_context.clone();

        tokio::spawn(async move {
            let mut stop = false;
            loop {
                if stop {
                    break;
                }
                tokio::select! {
                    line = stdout.next_line() => Self::handle_output(shared_context.clone(), line, &event_tx, StreamSource::Stdout).await,
                    line = stderr.next_line() => Self::handle_output(shared_context.clone(), line, &event_tx, StreamSource::Stderr).await,
                    _ = Self::set_timeout_from_config(shared_context.clone()) => Self::handle_timeout(shared_context.clone()).await,
                    reason = &mut terminate_rx => Self::handle_terminate(shared_context.clone(), reason, &mut stop).await,
                    reason = &mut internal_terminate_rx => Self::handle_terminate(shared_context.clone(), reason, &mut stop).await,
                    result = child.wait() => Self::handle_wait_result(shared_context.clone(), result,&mut stop).await,
                }
            }
            Self::handle_result(shared_context.clone(), child, &event_tx).await;
        });
        Ok(())
    }
}

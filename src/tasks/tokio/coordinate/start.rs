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
            let mut process_exited = false;
            let mut termination_requested = false;
            let mut stdout_eof = false;
            let mut stderr_eof = false;
            let mut timeout_triggered = false;
            loop {
                // Exit conditions
                if process_exited && stdout_eof && stderr_eof {
                    break;
                }

                // Force exit if termination was requested and streams are taking too long
                if termination_requested && stdout_eof && stderr_eof {
                    break;
                }
                tokio::select! {
                    line = stdout.next_line(), if !stdout_eof =>
                        stdout_eof = Self::handle_output(shared_context.clone(), line, &event_tx, StreamSource::Stdout).await,
                    line = stderr.next_line(), if !stderr_eof =>
                        stderr_eof = Self::handle_output(shared_context.clone(), line, &event_tx, StreamSource::Stderr).await,

                    _ = Self::set_timeout_from_config(shared_context.clone(), &mut timeout_triggered) => Self::handle_timeout(shared_context.clone()).await,

                    reason = Self::await_oneshot(&mut terminate_rx, termination_requested) =>
                        Self::handle_terminate(shared_context.clone(), &mut child, reason, &mut termination_requested).await,
                    reason = Self::await_oneshot(&mut internal_terminate_rx, termination_requested) =>
                        Self::handle_terminate(shared_context.clone(), &mut child, reason, &mut termination_requested).await,

                    result = child.wait() => Self::handle_wait_result(shared_context.clone(), result,&mut process_exited).await,
                }
            }
            Self::handle_result(shared_context.clone(), &event_tx).await;
        });
        Ok(())
    }
    async fn await_oneshot<T>(
        rx: &mut oneshot::Receiver<T>,
        termination_requested: bool,
    ) -> Result<T, oneshot::error::RecvError> {
        if termination_requested {
            std::future::pending().await
        } else {
            rx.await
        }
    }
}

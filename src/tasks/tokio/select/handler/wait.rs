use std::process::ExitStatus;

use crate::tasks::{event::TaskStopReason, tokio::select::executor::TaskExecutor};

impl TaskExecutor {
    pub(crate) async fn handle_wait_result(&mut self, result: Result<ExitStatus, std::io::Error>) {
        #[cfg(feature = "tracing")]
        tracing::trace!("child process finished");
        match result {
            Ok(status) => {
                let exit_code = status.code();
                self.exit_code = exit_code;
                self.stop_reason = Some(TaskStopReason::Finished);
                tracing::debug!(exit_code = ?exit_code, "Child process finished normally");
            }
            Err(e) => {
                // Expected OS level error
                self.stop_reason = Some(TaskStopReason::Error(e.to_string()));

                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Child process wait failed");
            }
        }

        self.flags.stop = true;
    }
}

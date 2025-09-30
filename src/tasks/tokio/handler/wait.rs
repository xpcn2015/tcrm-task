use std::{process::ExitStatus, sync::Arc};

use crate::tasks::{
    error::TaskError,
    event::TaskStopReason,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

impl TaskExecutor {
    pub(crate) async fn handle_wait_result(
        shared_context: Arc<TaskExecutorContext>,
        result: Result<ExitStatus, std::io::Error>,
        process_exited: &mut bool,
    ) {
        *process_exited = true;

        #[cfg(feature = "tracing")]
        tracing::trace!("child process finished");
        match result {
            Ok(status) => {
                let exit_code = status.code();
                shared_context.set_exit_code(exit_code);
                if shared_context.get_stop_reason().await.is_none() {
                    shared_context
                        .set_stop_reason(TaskStopReason::Finished)
                        .await;
                }
                #[cfg(unix)]
                if let Some(signal) = status.signal() {
                    shared_context.set_terminate_signal_code(Some(signal));
                }
                #[cfg(feature = "tracing")]
                tracing::debug!(exit_code = ?exit_code, "Child process finished normally");
            }
            Err(e) => {
                // Expected OS level error
                shared_context
                    .set_stop_reason(TaskStopReason::Error(TaskError::IO(format!(
                        "Failed to wait for child process: {}",
                        e
                    ))))
                    .await;

                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Child process wait failed");
            }
        }
    }
}

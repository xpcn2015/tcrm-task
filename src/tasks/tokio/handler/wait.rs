use std::{process::ExitStatus, sync::Arc};

use crate::tasks::{
    error::TaskError,
    event::TaskStopReason,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

impl TaskExecutor {
    /// Handles the result of waiting for child process completion.
    ///
    /// Processes the outcome of the child process, extracts exit codes and
    /// signal information, and updates the task context with the final status.
    ///
    /// # Exit Code Behavior
    ///
    /// - **Normal completion**: Sets exit code from process status
    /// - **Terminated processes**: Does not set exit code (remains `None`)
    ///   - This applies to timeout terminations, user-requested terminations, etc.
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context
    /// * `result` - Result from waiting on the child process
    /// * `process_exited` - Mutable reference to mark that the process has exited
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

                // Check if termination was already requested (e.g., by timeout)
                let stop_reason = shared_context.get_stop_reason().await;
                let is_terminated = matches!(stop_reason, Some(TaskStopReason::Terminated(_)));

                if !is_terminated {
                    // Normal completion - set exit code and reason
                    shared_context.set_exit_code(exit_code);
                    shared_context
                        .set_stop_reason(TaskStopReason::Finished)
                        .await;
                } else {
                    // Process was terminated - don't override exit code
                    // (it should remain None for timeout terminations)
                }

                #[cfg(unix)]
                if let Some(signal) = status.signal() {
                    shared_context.set_terminate_signal_code(Some(signal));
                }
                #[cfg(feature = "tracing")]
                tracing::debug!(exit_code = ?exit_code, is_terminated, "Child process finished");
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

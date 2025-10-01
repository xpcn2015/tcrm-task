use std::sync::Arc;

use tokio::{process::Child, sync::oneshot::error::RecvError};

use crate::tasks::{
    event::{TaskStopReason, TaskTerminateReason},
    process::child::terminate_process,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    /// Handles task termination requests.
    ///
    /// Processes termination signals and kills the child process.
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context
    /// * `child` - The child process to terminate
    /// * `reason` - Result containing the termination reason or channel error
    /// * `termination_requested` - Flag to mark that termination was requested
    pub(crate) async fn handle_terminate(
        shared_context: Arc<TaskExecutorContext>,
        child: &mut Child,
        reason: Result<TaskTerminateReason, RecvError>,
        termination_requested: &mut bool,
    ) {
        *termination_requested = true;

        #[cfg(feature = "tracing")]
        tracing::debug!(?reason, "Terminate signal received");

        let reason = match reason {
            Ok(r) => r,
            Err(_) => {
                #[cfg(feature = "tracing")]
                tracing::warn!("Terminate channel closed unexpectedly");
                return;
            }
        };

        shared_context
            .set_stop_reason(TaskStopReason::Terminated(reason))
            .await;

        match child.kill().await {
            Ok(_) => {
                // Successfully
            }
            Err(e) => {
                use std::io::ErrorKind;
                match e.kind() {
                    // Already exited: continue silently
                    ErrorKind::InvalidInput => {
                        // This usually means the process is already dead
                        #[cfg(feature = "tracing")]
                        tracing::debug!("Child process already exited, nothing to kill");
                    }
                    // Permission denied
                    ErrorKind::PermissionDenied => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("Permission denied when killing child process: {:?}", e);
                    }
                    // OS refuses (e.g., ESRCH, EPERM, etc.)
                    ErrorKind::NotFound => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("OS refused to kill child process (NotFound): {:?}", e);
                    }
                    // Process ignores signal (not directly detectable, but log)
                    _ => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("Failed to kill child process: {:?}", e);
                    }
                }
                // Try to terminate by process ID if available
                match shared_context.get_process_id() {
                    Some(process_id) => {
                        #[cfg(feature = "tracing")]
                        tracing::info!("Trying to terminate process ID {}", process_id);
                        if let Err(_e) = terminate_process(process_id) {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(
                                "Failed to terminate process ID {}: {:?}",
                                process_id,
                                _e
                            );
                        };
                    }
                    None => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("No process ID available to terminate");
                    }
                }
            }
        }
    }
}

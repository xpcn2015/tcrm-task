use std::sync::Arc;

use tokio::{
    process::Child,
    sync::{mpsc, oneshot::error::RecvError},
};

use crate::tasks::{
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    process::action::stop::stop_process,
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
        event_tx: &mpsc::Sender<TaskEvent>,
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
            Ok(_) =>
            {
                #[cfg(feature = "process-group")]
                if shared_context.config.use_process_group.unwrap_or_default() {
                    if let Err(e) = shared_context.group.lock().await.stop_group() {
                        use crate::tasks::{error::TaskError, event::TaskEvent};

                        let msg = format!("Failed to terminate process group: {}", e);

                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);

                        let error = TaskError::Control(msg);
                        let event = TaskEvent::Error { error };
                        Self::send_event(&event_tx, event).await;
                    };
                }
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
                        if let Err(_e) = stop_process(process_id) {
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

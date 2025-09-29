use std::sync::Arc;

use tokio::{process::Child, sync::mpsc};

use crate::tasks::{
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    process::child::terminate_process,
    state::TaskState,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    pub(crate) async fn handle_result(
        shared_context: Arc<TaskExecutorContext>,
        mut child: Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) {
        let reason = shared_context.get_stop_reason().await;
        let reason = match reason {
            Some(r) => r,
            None => {
                // This should not happen, but just in case
                let msg = "Task finished without a stop reason";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                TaskStopReason::Error(TaskError::Channel(msg.to_string()))
            }
        };

        if matches!(reason, TaskStopReason::Terminated(_)) {
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
        // If configured to use process group, ensure all child processes are terminated
        #[cfg(feature = "process-group")]
        if shared_context.config.use_process_group.unwrap_or_default() {
            if let Err(e) = shared_context.group.lock().await.terminate_group() {
                let msg = format!("Failed to terminate process group: {}", e);

                #[cfg(feature = "tracing")]
                tracing::error!(error=%e, "{}", msg);

                let error = TaskError::Control(msg);
                let event = TaskEvent::Error { error };
                Self::send_event(&event_tx, event).await;
            };
        }
        let time = shared_context.set_state(TaskState::Finished);
        let exit_code = shared_context.get_exit_code();
        shared_context.set_process_id(0);
        let event = TaskEvent::Stopped {
            exit_code,
            reason: reason,
            finished_at: time,
        };

        Self::send_event(&event_tx, event).await;
    }
}

use std::time::Instant;

use tokio::{process::Child, sync::mpsc};

use crate::tasks::{
    control::{TaskControl, TaskControlAction, TaskInternal},
    event::{TaskEvent, TaskStopReason},
    process::child::terminate_process,
    state::TaskState,
    tokio::select::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn handle_result(
        &mut self,
        mut child: Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) {
        let reason = match self.stop_reason.clone() {
            Some(r) => r,
            None => {
                // This should not happen, but just in case
                let msg = "Task finished without a stop reason";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                let reason = TaskStopReason::Error(msg.to_string());
                self.stop_reason = Some(reason.clone());
                reason
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
                    if let Some(process_id) = self.process_id {
                        #[cfg(feature = "tracing")]
                        tracing::info!("Trying to terminate process ID {}", process_id);
                        if let Err(e) = terminate_process(process_id) {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(
                                "Failed to terminate process ID {}: {:?}",
                                process_id,
                                e
                            );
                        };
                    }
                }
            }
        }
        // If configured to use process group, ensure all child processes are terminated
        #[cfg(feature = "process-group")]
        if self.config.use_process_group.unwrap_or_default() {
            let _ = self.perform_process_action(TaskControlAction::Terminate);
        }

        self.set_state(TaskState::Finished);
        self.finished_at = Some(Instant::now());
        let event = TaskEvent::Stopped {
            exit_code: self.exit_code,
            reason: reason,
        };
        self.send_event(event_tx, event).await;
    }
}

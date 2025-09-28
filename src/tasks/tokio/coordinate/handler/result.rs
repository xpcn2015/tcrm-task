use std::{
    os::windows::process,
    sync::{Arc, atomic::AtomicU32},
};

use tokio::{
    process::Child,
    sync::{Mutex, mpsc},
};

use crate::tasks::{
    control::{TaskControl, TaskControlAction},
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    process::child::terminate_process,
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn handle_result(
        mut child: Child,
        event_tx: &mpsc::Sender<TaskEvent>,
        stop_reason: &Arc<Mutex<Option<TaskStopReason>>>,
        process_id: Arc<AtomicU32>,
    ) {
        let reason = stop_reason.lock().await;
        let reason = match *reason {
            Some(r) => r,
            None => {
                // This should not happen, but just in case
                let msg = "Task finished without a stop reason";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                let r = TaskStopReason::Error(TaskError::Channel(msg.to_string()));
                *reason = Some(r.clone());
                r
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
                    let process_id = process_id.load(std::sync::atomic::Ordering::SeqCst);
                    if process_id != 0 {
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
        let time = self.set_state(TaskState::Finished);
        let event = TaskEvent::Stopped {
            exit_code: self.exit_code,
            reason: reason,
            finished_at: time,
        };
        self.send_event(event_tx, event).await;
    }
}

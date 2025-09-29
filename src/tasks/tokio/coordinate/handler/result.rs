use std::sync::{
    Arc,
    atomic::{AtomicI32, AtomicU8, AtomicU32, AtomicU64},
};

use tokio::{
    process::Child,
    sync::{Mutex, mpsc},
};

#[cfg(feature = "process-group")]
use crate::tasks::process::process_group::ProcessGroup;
use crate::tasks::{
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    process::child::terminate_process,
    state::TaskState,
    tokio::executor::TaskExecutor,
};
pub(crate) struct ResultArgs {
    pub(crate) event_tx: mpsc::Sender<TaskEvent>,
    pub(crate) stop_reason: Arc<Mutex<Option<TaskStopReason>>>,
    pub(crate) process_id: Arc<AtomicU32>,
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) finished_at: Arc<AtomicU64>,
    pub(crate) exit_code: Arc<AtomicI32>,
    #[cfg(feature = "process-group")]
    pub(crate) use_process_group: Option<bool>,
    #[cfg(feature = "process-group")]
    pub(crate) group: Arc<Mutex<ProcessGroup>>,
}
impl TaskExecutor {
    pub(crate) async fn handle_result(mut child: Child, args: ResultArgs) {
        let reason = args.stop_reason.lock().await.clone();
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
                    let process_id = args.process_id.load(std::sync::atomic::Ordering::SeqCst);
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
        if args.use_process_group.unwrap_or_default() {
            if let Err(e) = args.group.lock().await.terminate_group() {
                let msg = format!("Failed to terminate process group: {}", e);
                #[cfg(feature = "tracing")]
                tracing::error!(error=%e, "{}", msg);
                let error = TaskError::Control(msg);
                let event = TaskEvent::Error { error };
                Self::send_event(&args.event_tx, event).await;
            };
        }
        let time = Self::set_state(args.state, TaskState::Finished, Some(args.finished_at));
        let exit_code = match args.exit_code.load(std::sync::atomic::Ordering::SeqCst) {
            -1 => None,
            code => Some(code),
        };
        args.process_id
            .store(0, std::sync::atomic::Ordering::SeqCst);
        let event = TaskEvent::Stopped {
            exit_code,
            reason: reason,
            finished_at: time,
        };

        Self::send_event(&args.event_tx, event).await;
    }
}

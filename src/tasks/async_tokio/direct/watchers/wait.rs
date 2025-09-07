use std::sync::Arc;

use tokio::{
    process::Child,
    sync::{RwLock, oneshot, watch},
    task::JoinHandle,
};
use tracing::{Instrument, debug, error, instrument, warn};

use crate::tasks::{
    event::TaskEventStopReason,
    state::{TaskState, TaskTerminateReason},
};

/// Spawns a watcher that waits for the child process to exit or be terminated
///
/// Sends stop reason and signals other watchers to terminate
#[instrument(skip_all)]
pub(crate) fn spawn_wait_watcher(
    task_name: String,
    state: Arc<RwLock<TaskState>>,
    mut child: Child,
    terminate_rx: oneshot::Receiver<TaskTerminateReason>,
    handle_terminator_tx: watch::Sender<bool>,
    result_tx: oneshot::Sender<(Option<i32>, TaskEventStopReason)>,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            tokio::select! {
                result = child.wait() => {
                    match result {
                        Ok(status) => {
                            let exit_code = status.code();
                            if let Err(_) = result_tx.send((
                                exit_code,
                                TaskEventStopReason::Finished,
                            )) {
                                warn!(exit_code, "Result channel closed while sending TaskEventStopReason::Finished");
                            };
                            debug!(exit_code = ?exit_code, "Child process finished normally");
                        }
                        Err(e) => {
                            // Expected OS level error
                            if let Err(_) = result_tx.send((
                                None,
                                TaskEventStopReason::Error(e.to_string()),
                            )) {
                                warn!(error = %e, "Result channel closed while sending TaskEventStopReason::Error");
                            };
                            error!(error = %e, "Child process wait failed");
                        }
                    }
                }
                reason = terminate_rx => {
                    if let Err(e) = child.kill().await {
                        // Expected OS level error
                        if let Err(_) = result_tx.send((
                            None,
                            TaskEventStopReason::Error(format!(
                                "Failed to terminate task {}: {}",
                                task_name, e
                            )),
                        )) {
                            warn!(error = %e, "Result channel closed while sending TaskEventStopReason::Error");
                        };
                        error!(error = %e, "Failed to kill child process");
                        return;
                    }

                    *state.write().await = TaskState::Finished;
                    let reason = reason.unwrap_or(TaskTerminateReason::Custom(
                            "Terminate rx channel closed".to_string(),
                    ));
                    if let Err(_) = result_tx.send((
                        None,
                        TaskEventStopReason::Terminated(reason.clone()),
                    )) {
                        warn!(reason = ?reason, "Result channel closed while sending TaskEventStopReason::Terminated");
                    };
                    debug!(reason = ?reason, "Child process terminated via watcher");
                }
            }
            // Task finished, send handle terminate signal
            if let Err(_) = handle_terminator_tx.send(true){
                warn!("Handle terminate channels closed while sending signal");
            };
            debug!("Watcher finished");
        }
        .instrument(tracing::debug_span!("spawn")),
    );
    debug!(
        handle_id = %handle.id(),
        "Spawned wait watcher handle"
    );

    handle
}

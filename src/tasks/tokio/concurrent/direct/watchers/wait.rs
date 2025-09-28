use std::sync::Arc;

use tokio::{
    process::Child,
    sync::{RwLock, oneshot, watch},
    task::JoinHandle,
};

use crate::{
    helper::tracing::MaybeInstrument,
    tasks::{
         event::{TaskStopReason, TaskTerminateReason}, process::process_group::{ProcessGroup, ProcessGroupError}, state::TaskState
    },
};

/// Spawns a watcher that waits for the child process to exit or be terminated.
///
/// Sends stop reason and signals other watchers to terminate.
/// Uses cross-platform process group termination to kill entire process trees.
///
/// # Arguments
///
/// * `task_name` - Name of the task.
/// * `state` - Shared state of the task.
/// * `child` - The child process to monitor.
/// * `process_group` - Process group for killing entire process trees.
/// * `terminate_rx` - Receiver for termination signals.
/// * `handle_terminator_tx` - Sender to signal other watchers to terminate.
/// * `result_tx` - Sender for the process exit code and stop reason.
/// * `process_id` - Shared process ID.
///
/// # Returns
///
/// A `JoinHandle` for the spawned watcher task.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub(crate) fn spawn_wait_watcher(
    state: Arc<RwLock<TaskState>>,
    mut child: Child,
    process_group: Option<ProcessGroup>,
    terminate_rx: oneshot::Receiver<TaskTerminateReason>,
    handle_terminator_tx: watch::Sender<bool>,
    result_tx: oneshot::Sender<(Option<i32>, TaskStopReason)>,
    process_id: Arc<RwLock<Option<u32>>>,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            tokio::select! {
                result = child.wait() => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("child process finished");
                    
                    // When main process exits, it must terminate any remaining child processes
                    // to prevent orphaned processes from continuing to run
                    if let Some(ref pg) = process_group {
                        #[cfg(feature = "tracing")]
                        tracing::debug!("Main process finished, terminating remaining child processes in group");
                        if let Err(_e) = pg.terminate_group() {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(error = %_e, "Failed to terminate remaining child processes after main process exit");
                        }
                    }
                    
                    match result {
                        Ok(status) => {
                            let exit_code = status.code();
                            if result_tx.send((
                                exit_code,
                                TaskStopReason::Finished,
                            )).is_err() {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!(exit_code, "Result channel closed while sending TaskEventStopReason::Finished");
                            }
                                #[cfg(feature = "tracing")]
                                tracing::debug!(exit_code = ?exit_code, "Child process finished normally, child processes terminated");
                        }
                        Err(e) => {
                            // Expected OS level error
                            if result_tx.send((
                                None,
                                TaskStopReason::Error(e.to_string()),
                            )).is_err() {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!(error = %e, "Result channel closed while sending TaskEventStopReason::Error");
                            }
                                #[cfg(feature = "tracing")]
                                tracing::error!(error = %e, "Child process wait failed");
                        }
                    }
                }
                reason = terminate_rx => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("Termination signal received");

                    // Try to terminate the entire process group if enabled, otherwise just the individual process
                    let termination_result = if let Some(ref pg) = process_group {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Terminating process group");
                        pg.terminate_group()
                    } else {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Process group disabled, terminating individual process");
                        child.kill().await.map_err(|e| ProcessGroupError::SignalFailed(e.to_string()))
                    };

                    if let Err(e) = termination_result {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(error = %e, "Process termination failed");
                        
                        // If process group termination failed and we're using process groups, fallback to individual kill
                        if process_group.is_some() {
                            if let Err(e2) = child.kill().await {
                                // Expected OS level error
                                if result_tx.send((
                                    None,
                                    TaskStopReason::Error(format!(
                                        "Failed to terminate task : process group: {e}, individual: {e2}"
                                    )),
                                )).is_err() {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!(error = %e2, "Result channel closed while sending TaskEventStopReason::Error");
                                }
                                #[cfg(feature = "tracing")]
                                tracing::error!(error = %e2, "Failed to kill child process after process group failure");
                                return;
                            }
                        } else {
                            // Process group not available and individual termination failed
                            if result_tx.send((
                                None,
                                TaskStopReason::Error(format!(
                                    "Failed to terminate task : {e}"
                                )),
                            )).is_err() {
                                #[cfg(feature = "tracing")]
                                tracing::warn!(error = %e, "Result channel closed while sending TaskEventStopReason::Error");
                            }
                            #[cfg(feature = "tracing")]
                            tracing::error!(error = %e, "Failed to kill child process");
                            return;
                        }
                    }

                    *state.write().await = TaskState::Finished;
                    let reason = reason.unwrap_or(TaskTerminateReason::Cleanup);
                    if result_tx.send((
                        None,
                        TaskStopReason::Terminated(reason.clone()),
                    )).is_err() {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(reason = ?reason, "Result channel closed while sending TaskEventStopReason::Terminated");
                    }
                        #[cfg(feature = "tracing")]
                        tracing::debug!(reason = ?reason, "Process group terminated via watcher");
                }
            }
            // Task finished, send handle terminate signal
            if handle_terminator_tx.send(true).is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Handle terminate channels closed while sending signal");
            }
                process_id.write().await.take();

                #[cfg(feature = "tracing")]
                tracing::debug!("Watcher finished");
        }
        .maybe_instrument("spawn"),
    );
    #[cfg(feature = "tracing")]
    tracing::debug!(
        handle_id = %handle.id(),
        "Spawned wait watcher handle"
    );

    handle
}

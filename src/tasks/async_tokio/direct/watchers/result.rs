use std::sync::Arc;

use tokio::{
    sync::{RwLock, mpsc, oneshot},
    task::JoinHandle,
    time::Instant,
};
use tracing::{Instrument, debug, info, instrument, warn};

use crate::tasks::{
    async_tokio::spawner::join_all_handles,
    event::{TaskEvent, TaskEventStopReason},
    state::TaskState,
};

/// Spawns a watcher that waits for the task result and updates state
///
/// Joins all watcher handles and sends a `TaskEvent::Stopped` event
#[instrument(skip_all)]
pub(crate) fn spawn_result_watcher(
    task_name: String,
    state: Arc<RwLock<TaskState>>,
    finished_arc: Arc<RwLock<Option<Instant>>>,
    event_tx: mpsc::Sender<TaskEvent>,
    result_rx: oneshot::Receiver<(Option<i32>, TaskEventStopReason)>,
    mut task_handles: Vec<JoinHandle<()>>,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            let (exit_code, stop_reason) = match result_rx.await {
                Ok(result) => result,
                Err(_) => {
                    // Somehow, all tx has been dropped, this is unexpected
                    let msg = "All result senders dropped unexpectedly";
                    warn!(msg);
                    (None, TaskEventStopReason::Error(msg.to_string()))
                }
            };
            info!(
                exit_code = ?exit_code,
                stop_reason = ?stop_reason,
                "Task stopped"
            );
            if let Err(e) = join_all_handles(&mut task_handles).await {
                warn!(
                    error = %e,
                    "One or more task handles failed to join cleanly"
                );
            }

            if let Err(_) = event_tx
                .send(TaskEvent::Stopped {
                    task_name: task_name.clone(),
                    exit_code,
                    reason: stop_reason.clone(),
                })
                .await
            {
                warn!("Event channel closed while sending TaskEvent::Stopped");
            }

            *state.write().await = TaskState::Finished;
            *finished_arc.write().await = Some(Instant::now());

            debug!("Watcher finished");
        }
        .instrument(tracing::debug_span!("spawn")),
    );
    debug!(
        handle_id = %handle.id(),
        "Spawned result watcher handle");
    handle
}

use std::sync::Arc;

use tokio::{
    sync::{RwLock, mpsc, oneshot},
    task::JoinHandle,
    time::Instant,
};

use crate::{
    helper::tracing::MaybeInstrument,
    tasks::{
        async_tokio::spawner::join_all_handles,
        event::{TaskEvent, TaskEventStopReason},
        state::TaskState,
    },
};

/// Spawns a watcher that waits for the task result and updates state
///
/// Joins all watcher handles and sends a `TaskEvent::Stopped` event.
///
/// # Arguments
///
/// * `task_name` - Name of the task.
/// * `state` - Shared state of the task.
/// * `finished_arc` - Shared reference to the task's finished time.
/// * `event_tx` - Sender for task events.
/// * `result_rx` - Receiver for the process exit code and stop reason.
/// * `task_handles` - Vector of watcher task handles to join.
///
/// # Returns
///
/// A `JoinHandle` for the spawned result watcher task.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
            let (exit_code, stop_reason) = if let Ok(result) = result_rx.await { result } else {
                // Somehow, all tx has been dropped, this is unexpected
                let msg = "All result senders dropped unexpectedly";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                (None, TaskEventStopReason::Error(msg.to_string()))
            };
            #[cfg(feature = "tracing")]
            tracing::info!(
                exit_code = ?exit_code,
                stop_reason = ?stop_reason,
                "Task stopped"
            );
            if let Err(_e) = join_all_handles(&mut task_handles).await {
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_e, "One or more task handles failed to join cleanly");
            }

            if (event_tx
                .send(TaskEvent::Stopped {
                    task_name: task_name.clone(),
                    exit_code,
                    reason: stop_reason.clone(),
                })
                .await)
                .is_err()
            {
                #[cfg(feature = "tracing")]
                tracing::warn!("Event channel closed while sending TaskEvent::Stopped");
            }

            *state.write().await = TaskState::Finished;
            *finished_arc.write().await = Some(Instant::now());

            #[cfg(feature = "tracing")]
            tracing::debug!("Watcher finished");
        }
        .maybe_instrument("spawn"),
    );
    #[cfg(feature = "tracing")]
    tracing::debug!(
        handle_id = %handle.id(),
        "Spawned result watcher handle");
    handle
}

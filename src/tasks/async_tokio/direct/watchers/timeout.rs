use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, oneshot, watch},
    task::JoinHandle,
};

use crate::{helper::tracing::MaybeInstrument, tasks::state::TaskTerminateReason};

/// Spawns a watcher that triggers a timeout after the specified duration.
///
/// Sends a termination signal if the timeout elapses.
///
/// # Arguments
///
/// * `terminate_tx` - Sender for termination signals.
/// * `timeout_ms` - Timeout duration in milliseconds.
/// * `handle_terminator_rx` - Receiver to listen for termination signals.
///
/// # Returns
///
/// A `JoinHandle` for the spawned timeout watcher task.
#[cfg_attr(feature = "tracing", tracing::instrument(skip(terminate_tx)))]
pub(crate) fn spawn_timeout_watcher(
    terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    timeout_ms: u64,
    mut handle_terminator_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            #[cfg(feature = "tracing")]
            tracing::trace!(timeout_ms, "Starting timeout watcher");
            let sleep = tokio::time::sleep(Duration::from_millis(timeout_ms));
            tokio::pin!(sleep);
            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        #[cfg(feature = "tracing")]
                        tracing::info!("Task timeout reached, sending termination signal");
                        match terminate_tx.lock().await.take() {
                            Some(tx) => {
                                if let Err(_) = tx.send(TaskTerminateReason::Timeout) {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!("Event channel closed while sending TaskEvent::Timeout");
                                }
                            }
                            None => {
                                #[cfg(feature = "tracing")]
                                tracing::warn!("Terminate signal already sent or channel missing");
                            }
                        }
                        break;
                    }
                    _ = handle_terminator_rx.changed() => {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Task handle termination signal received");

                        if *handle_terminator_rx.borrow() {
                            #[cfg(feature = "tracing")]
                            tracing::debug!("Termination signal received, closing timeout watcher");
                            break;
                        }
                    }
                }
            }
                #[cfg(feature = "tracing")]
                tracing::debug!("Watcher finished");
        }
        .maybe_instrument("spawn"),
    );
    #[cfg(feature = "tracing")]
    tracing::debug!(
        handle_id = %handle.id(),
        "Spawned timeout watcher handle"
    );
    handle
}

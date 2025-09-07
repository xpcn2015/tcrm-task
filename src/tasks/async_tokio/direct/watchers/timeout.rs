use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, oneshot, watch},
    task::JoinHandle,
};
use tracing::{Instrument, debug, instrument, warn};

use crate::tasks::state::TaskTerminateReason;

/// Spawns a watcher that triggers a timeout after the specified duration
///
/// Sends a termination signal if the timeout elapses
#[instrument(skip(terminate_tx))]
pub(crate) fn spawn_timeout_watcher(
    terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    timeout_ms: u64,
    mut handle_terminator_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            let sleep = tokio::time::sleep(Duration::from_millis(timeout_ms));
            tokio::pin!(sleep);
            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        match terminate_tx.lock().await.take() {
                            Some(tx) => {
                                if let Err(_) = tx.send(TaskTerminateReason::Timeout) {
                                    warn!("Event channel closed while sending TaskEvent::Timeout");
                                }
                            }
                            None => {
                                warn!("Terminate signal already sent or channel missing");
                            }
                        }
                        break;
                    }
                    _ = handle_terminator_rx.changed() => {
                        if *handle_terminator_rx.borrow() {
                            debug!("Termination signal received, closing timeout watcher");
                            break;
                        }
                    }
                }
            }
            debug!("Watcher finished");
        }
        .instrument(tracing::debug_span!("spawn")),
    );
    debug!(
        handle_id = %handle.id(),
        "Spawned timeout watcher handle"
    );
    handle
}

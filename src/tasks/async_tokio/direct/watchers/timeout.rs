use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, oneshot},
    task::JoinHandle,
};
use tracing::{Instrument, debug, instrument, warn};

use crate::tasks::state::TaskTerminateReason;

#[instrument(skip(terminate_tx))]
pub fn spawn_timeout_watcher(
    terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    timeout_ms: u64,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
            match terminate_tx.lock().await.take() {
                Some(tx) => {
                    if let Err(_) = tx.send(TaskTerminateReason::Timeout) {
                        warn!("Event channel closed while sending TaskEvent::Timeout");
                    }
                }
                None => {
                    warn!("Terminate signal already sent or channel missing");
                }
            };

            debug!("Watcher finished");
        }
        .instrument(tracing::debug_span!("tokio::spawn(timeout_watcher)")),
    );
    debug!(
        handle_id = %handle.id(),
        "Spawned timeout watcher handle"
    );

    handle
}

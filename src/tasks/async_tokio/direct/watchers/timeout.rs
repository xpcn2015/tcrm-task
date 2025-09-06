use std::time::Duration;

use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{Instrument, debug, instrument, warn};

use crate::tasks::state::TaskTerminateReason;

#[instrument(skip(terminate_tx))]
pub fn spawn_timeout_watcher(
    terminate_tx: mpsc::UnboundedSender<TaskTerminateReason>,
    timeout_ms: u64,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
            if let Err(_) = terminate_tx.send(TaskTerminateReason::Timeout) {
                warn!("Event channel closed while sending TaskEvent::Timeout");
            }
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

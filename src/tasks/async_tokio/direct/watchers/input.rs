use tokio::{
    io::AsyncWriteExt,
    process::ChildStdin,
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{Instrument, debug, instrument, warn};
/// Spawns an asynchronous watcher for task stdin
///
/// Listens for lines from a channel and writes them to the child process's stdin
///
/// Terminates when the channel is closed or a termination signal is received
#[instrument(skip_all)]
pub(crate) fn spawn_stdin_watcher(
    mut stdin: ChildStdin,
    mut stdin_rx: mpsc::Receiver<String>,
    mut handle_terminator_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let handle = tokio::spawn(
        async move {
            loop {
                tokio::select! {
                    // New line from stdin channel
                    maybe_line = stdin_rx.recv() => {
                        match maybe_line {
                            Some(mut line) => {
                                if !line.ends_with('\n') {
                                    line.push('\n');
                                }
                                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                                    warn!(error=%e, "Failed to write to child stdin");
                                    break;
                                }
                            }
                            None => {
                                // Channel closed, stop watcher
                                break;
                            }
                        }
                    }

                    // Termination signal
                    _ = handle_terminator_rx.changed() => {
                        if *handle_terminator_rx.borrow() {
                            debug!("Termination signal received, closing stdin watcher");
                            break;
                        }
                    }
                }
            }

            // Close stdin when channel is closed
            if let Err(e) = stdin.shutdown().await {
                warn!(error=%e, "Failed to shutdown child stdin");
            }

            debug!("Watcher finished");
        }
        .instrument(tracing::debug_span!("spawn")),
    );

    debug!(
        handle_id = %handle.id(),
        "Spawned stdin watcher handle"
    );

    handle
}

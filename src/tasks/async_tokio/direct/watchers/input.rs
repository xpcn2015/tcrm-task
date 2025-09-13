use tokio::{
    io::AsyncWriteExt,
    process::ChildStdin,
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::helper::tracing::MaybeInstrument;
/// Spawns an asynchronous watcher for task stdin
///
/// Listens for lines from a channel and writes them to the child process's stdin
///
/// Terminates when the channel is closed or a termination signal is received
///
/// # Arguments
///
/// * `stdin` - The stdin handle of the child process.
/// * `stdin_rx` - Receiver channel for stdin input strings.
/// * `handle_terminator_rx` - Receiver to listen for termination signals.
///
/// # Returns
///
/// A `JoinHandle` for the spawned stdin watcher task.
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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

                                #[cfg(feature = "tracing")]
                                tracing::trace!(line, "Received line for stdin");

                                if !line.ends_with('\n') {
                                    line.push('\n');
                                }
                                if let Err(_e) = stdin.write_all(line.as_bytes()).await {
                                        #[cfg(feature = "tracing")]
                                        tracing::warn!(error=%_e, "Failed to write to child stdin");
                                    break;
                                }
                            }
                            None => {
                                #[cfg(feature = "tracing")]
                                tracing::trace!("Stdin channel closed");
                                // Channel closed, stop watcher
                                break;
                            }
                        }
                    }

                    // Termination signal
                    _ = handle_terminator_rx.changed() => {
                            #[cfg(feature = "tracing")]
                            tracing::trace!("Task handle termination signal received");

                        if *handle_terminator_rx.borrow() {
                            #[cfg(feature = "tracing")]
                            tracing::debug!("Termination signal received, closing stdin watcher");
                            break;
                        }
                    }
                }
            }

            // Close stdin when channel is closed
            if let Err(_e) = stdin.shutdown().await {
                #[cfg(feature = "tracing")]
                tracing::warn!(error=%_e, "Failed to shutdown child stdin");
            }
            #[cfg(feature = "tracing")]
            tracing::debug!("Watcher finished");
        }
        .maybe_instrument("spawn"),
    );
    #[cfg(feature = "tracing")]
    tracing::debug!(
        handle_id = %handle.id(),
        "Spawned stdin watcher handle"
    );

    handle
}

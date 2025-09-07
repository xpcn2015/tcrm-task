use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    helper::tracing::MaybeInstrument,
    tasks::{config::StreamSource, event::TaskEvent},
};

/// Spawns watchers for stdout and stderr of a child process
///
/// Sends output lines as `TaskEvent::Output` events
pub(crate) fn spawn_output_watchers(
    task_name: String,
    event_tx: mpsc::Sender<TaskEvent>,
    child: &mut Child,
    handle_terminator_rx: tokio::sync::watch::Receiver<bool>,
) -> Vec<JoinHandle<()>> {
    let mut handles: Vec<JoinHandle<()>> = vec![];
    // Spawn stdout watcher
    if let Some(stdout) = child.stdout.take() {
        let handle = spawn_std_watcher(
            stdout,
            task_name.clone(),
            event_tx.clone(),
            StreamSource::Stdout,
            handle_terminator_rx.clone(),
        );
        handles.push(handle);
    }

    // Spawn stderr watcher
    if let Some(stderr) = child.stderr.take() {
        let handle = spawn_std_watcher(
            stderr,
            task_name,
            event_tx,
            StreamSource::Stderr,
            handle_terminator_rx,
        );
        handles.push(handle);
    }

    handles
}

/// Spawns a watcher for a single output stream (stdout or stderr)
///
/// Each line is sent as a `TaskEvent::Output`
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(stream = ?src)))]
fn spawn_std_watcher<T>(
    std: T,
    task_name: String,
    event_tx: mpsc::Sender<TaskEvent>,
    src: StreamSource,
    mut handle_terminator_rx: tokio::sync::watch::Receiver<bool>,
) -> JoinHandle<()>
where
    T: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let handle = tokio::spawn(
        async move {
            let reader = BufReader::new(std);
            let mut lines = reader.lines();
            loop {
                tokio::select! {
                    line_result = lines.next_line() => {
                        match line_result {
                            Ok(Some(line)) => {
                                    #[cfg(feature = "tracing")]
                                    tracing::trace!(line = %line);
                                if let Err(_) = event_tx
                                    .send(TaskEvent::Output {
                                        task_name: task_name.clone(),
                                        line,
                                        src: src.clone(),
                                    })
                                    .await
                                {
                                        #[cfg(feature = "tracing")]
                                        tracing::warn!("Event channel closed while sending TaskEvent::Output");
                                    break;
                                }
                            }
                            Ok(None) => {
                                // EOF
                                break;
                            }
                            Err(_e) => {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!(error=%_e, "Error reading line from output stream");
                                break;
                            }
                        }
                    }
                    _ = handle_terminator_rx.changed() => {
                        if *handle_terminator_rx.borrow() {
                                #[cfg(feature = "tracing")]
                                tracing::debug!("Termination signal received, closing output watcher");
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
        "Spawned std output watcher handle"
    );
    handle
}

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
    sync::mpsc,
    task::JoinHandle,
};
use tracing::{Instrument, debug, instrument, trace, warn};

use crate::tasks::{config::StreamSource, event::TaskEvent};
pub fn spawn_output_watchers(
    task_name: String,
    event_tx: mpsc::Sender<TaskEvent>,
    child: &mut Child,
) -> Vec<JoinHandle<()>> {
    let mut handles: Vec<JoinHandle<()>> = vec![];
    // Spawn stdout watcher
    if let Some(stdout) = child.stdout.take() {
        let handle = spawn_std_watcher(
            stdout,
            task_name.clone(),
            event_tx.clone(),
            StreamSource::Stdout,
        );

        handles.push(handle);
    }

    // Spawn stderr watcher
    if let Some(stderr) = child.stderr.take() {
        let handle = spawn_std_watcher(stderr, task_name, event_tx, StreamSource::Stderr);

        handles.push(handle);
    }

    handles
}
#[instrument(skip_all, fields(stream = ?src))]
pub fn spawn_std_watcher<T>(
    std: T,
    task_name: String,
    event_tx: mpsc::Sender<TaskEvent>,
    src: StreamSource,
) -> JoinHandle<()>
where
    T: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let src_tracing = src.clone();
    let handle = tokio::spawn(
        async move {
            let reader = BufReader::new(std);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                trace!(line = %line);
                if let Err(_) = event_tx
                    .send(TaskEvent::Output {
                        task_name: task_name.clone(),
                        line,
                        src: src.clone(),
                    })
                    .await
                {
                    warn!("Event channel closed while sending TaskEvent::Output");
                    break;
                }
            }

            debug!("Watcher finished");
        }
        .instrument(tracing::debug_span!("tokio::spawn(std_watcher)", stream = ?src_tracing)),
    );
    debug!(
        handle_id = %handle.id(),
        "Spawned std output watcher handle"
    );

    handle
}

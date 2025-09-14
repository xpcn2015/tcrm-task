use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
    sync::{RwLock, mpsc},
    task::JoinHandle,
};

use crate::{
    helper::tracing::MaybeInstrument,
    tasks::{config::StreamSource, event::TaskEvent, state::TaskState},
};

/// Configuration for spawning output watchers
#[derive(Debug)]
struct OutputWatcherConfig {
    task_name: String,
    state: Arc<RwLock<TaskState>>,
    event_tx: mpsc::Sender<TaskEvent>,
    src: StreamSource,
    handle_terminator_rx: tokio::sync::watch::Receiver<bool>,
    ready_indicator: Option<String>,
    ready_indicator_source: StreamSource,
}

/// Spawns watchers for stdout and stderr of a child process.
///
/// Sends output lines as `TaskEvent::Output` events.
///
/// # Arguments
///
/// * `task_name` - Name of the task.
/// * `state` - Shared state of the task.
/// * `event_tx` - Sender for task events.
/// * `child` - The child process to monitor.
/// * `handle_terminator_rx` - Receiver to listen for termination signals.
/// * `ready_indicator` - Optional string indicating readiness.
/// * `ready_indicator_source` - Optional source for the ready indicator (stdout or stderr).
///
/// # Returns
///
/// A vector of `JoinHandle` for the spawned watcher tasks.
pub(crate) fn spawn_output_watchers(
    task_name: String,
    state: Arc<RwLock<TaskState>>,
    event_tx: mpsc::Sender<TaskEvent>,
    child: &mut Child,
    handle_terminator_rx: tokio::sync::watch::Receiver<bool>,
    ready_indicator: Option<String>,
    ready_indicator_source: Option<StreamSource>,
) -> Vec<JoinHandle<()>> {
    let mut handles: Vec<JoinHandle<()>> = vec![];
    // Spawn stdout watcher
    if let Some(stdout) = child.stdout.take() {
        let config = OutputWatcherConfig {
            task_name: task_name.clone(),
            state: state.clone(),
            event_tx: event_tx.clone(),
            src: StreamSource::Stdout,
            handle_terminator_rx: handle_terminator_rx.clone(),
            ready_indicator: ready_indicator.clone(),
            ready_indicator_source: ready_indicator_source.clone().unwrap_or_default(),
        };
        let handle = spawn_std_watcher(stdout, config);
        handles.push(handle);
    }

    // Spawn stderr watcher
    if let Some(stderr) = child.stderr.take() {
        let config = OutputWatcherConfig {
            task_name,
            state,
            event_tx,
            src: StreamSource::Stderr,
            handle_terminator_rx,
            ready_indicator,
            ready_indicator_source: ready_indicator_source.unwrap_or_default(),
        };
        let handle = spawn_std_watcher(stderr, config);
        handles.push(handle);
    }

    handles
}

/// Spawns a watcher for a single output stream (stdout or stderr).
///
/// Monitors the specified stream for output lines and ready indicators.
/// Each line is sent as a `TaskEvent::Output` event. If a ready indicator
/// is configured and matches the stream source, the task state is updated
/// to `Ready` when the indicator text is found.
///
/// # Arguments
///
/// * `std` - The async readable stream to monitor.
/// * `task_name` - Name of the task for event identification.
/// * `state` - Shared task state for ready indicator updates.
/// * `event_tx` - Channel sender for emitting task events.
/// * `src` - Source stream type (stdout or stderr).
/// * `handle_terminator_rx` - Receiver for termination signals.
/// * `ready_indicator` - Optional text to watch for readiness.
/// * `ready_indicator_source` - Stream source where ready indicator is expected.
///
/// # Returns
///
/// A `JoinHandle` for the spawned watcher task.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(stream = ?config.src)))]
fn spawn_std_watcher<T>(std: T, config: OutputWatcherConfig) -> JoinHandle<()>
where
    T: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let OutputWatcherConfig {
        task_name,
        state,
        event_tx,
        src,
        mut handle_terminator_rx,
        ready_indicator,
        ready_indicator_source,
    } = config;
    let handle = tokio::spawn(
        async move {
            let reader = BufReader::new(std);
            let mut lines = reader.lines();
            let mut ready_found = false;
            loop {
                tokio::select! {
                    line_result = lines.next_line() => {
                        match line_result {
                            Ok(Some(line)) => {
                                #[cfg(feature = "tracing")]
                                tracing::trace!(line = %line);

                                let line_for_ready = if ready_indicator_source == src && !ready_found {
                                    line.clone()
                                } else {
                                    "".to_string()
                                };

                                if (event_tx
                                    .send(TaskEvent::Output {
                                        task_name: task_name.clone(),
                                        line,
                                        src: src.clone(),
                                    })
                                    .await).is_err()
                                {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!("Event channel closed while sending TaskEvent::Output");
                                    break;
                                }

                                // Check for ready indicator
                                if ready_indicator_source != src || ready_found {
                                    continue;
                                }
                                let ready_indicator = match &ready_indicator {
                                    Some(i) => i,
                                    None => continue,
                                };

                                if line_for_ready.contains(ready_indicator) {
                                    ready_found = true;
                                    #[cfg(feature = "tracing")]
                                    tracing::debug!(stream=?src, "Ready indicator found in output stream");
                                }

                                #[cfg(feature = "tracing")]
                                tracing::debug!("Updating task state to Ready");
                                *state.write().await = TaskState::Ready;
                                if (event_tx
                                    .send(TaskEvent::Ready {
                                        task_name: task_name.clone(),
                                    })
                                    .await).is_err()
                                {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!("Event channel closed while sending TaskEvent::Ready");
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
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Task handle termination signal received");
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
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn does_not_emit_ready_if_source_mismatch() {
        let data = b"foo\nREADY_INDICATOR\nbar\n";
        let cursor = std::io::Cursor::new(&data[..]);
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(10);
        let (_term_tx, term_rx) = watch::channel(false);
        let ready_indicator = Some("READY_INDICATOR".to_string());
        let task_name = "test_task_mismatch".to_string();
        let state = Arc::new(RwLock::new(TaskState::Running));

        // src is Stdout, ready_indicator_source is Stderr (should NOT emit Ready)
        let config = OutputWatcherConfig {
            task_name: task_name.clone(),
            state: state.clone(),
            event_tx: tx,
            src: StreamSource::Stdout,
            handle_terminator_rx: term_rx,
            ready_indicator: ready_indicator.clone(),
            ready_indicator_source: StreamSource::Stderr,
        };
        let handle = spawn_std_watcher(cursor, config);

        let mut ready_event = false;
        while let Some(event) = rx.recv().await {
            if let TaskEvent::Ready { .. } = event {
                ready_event = true;
            }
        }
        handle.await.unwrap();
        assert_eq!(
            ready_event, false,
            "Should NOT emit Ready event if ready_indicator_source does not match src"
        );
        let state_val = state.read().await.clone();
        assert_eq!(
            state_val,
            TaskState::Running,
            "State should not be set to Ready"
        );
    }

    use super::*;
    use crate::tasks::config::StreamSource;
    use crate::tasks::event::TaskEvent;
    use std::io::Cursor;
    use tokio::sync::{mpsc, watch};

    #[tokio::test]
    async fn emits_ready_and_output_events() {
        let data = b"first line\nREADY_INDICATOR\nlast line\n";
        let cursor = Cursor::new(&data[..]);
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(10);
        let (_term_tx, term_rx) = watch::channel(false);
        let ready_indicator = Some("READY_INDICATOR".to_string());
        let task_name = "test_task".to_string();
        let state = Arc::new(RwLock::new(TaskState::Running));

        let config = OutputWatcherConfig {
            task_name: task_name.clone(),
            state: state.clone(),
            event_tx: tx,
            src: StreamSource::Stdout,
            handle_terminator_rx: term_rx,
            ready_indicator: ready_indicator.clone(),
            ready_indicator_source: StreamSource::Stdout,
        };
        let handle = spawn_std_watcher(cursor, config);

        let mut output_lines = vec![];
        let mut ready_event = false;
        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Output {
                    task_name: tn,
                    line,
                    src,
                } => {
                    assert_eq!(tn, task_name);
                    assert_eq!(src, StreamSource::Stdout);
                    output_lines.push(line);
                }
                TaskEvent::Ready { task_name: tn } => {
                    assert_eq!(tn, task_name);
                    ready_event = true;
                }
                _ => {}
            }
        }
        handle.await.unwrap();
        assert_eq!(
            output_lines,
            vec!["first line", "READY_INDICATOR", "last line"]
        );
        assert!(
            ready_event,
            "Should emit Ready event when ready_indicator is present"
        );
        let state_val = state.read().await.clone();
        assert_eq!(
            state_val,
            TaskState::Ready,
            "State should be set to Ready when ready indicator is found"
        );
    }
}

use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout},
    sync::mpsc,
};

use crate::tasks::{
    config::StreamSource,
    control::TaskInternal,
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::select::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn take_std_output_reader(
        &mut self,
        child: &mut Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(Lines<BufReader<ChildStdout>>, Lines<BufReader<ChildStderr>>), TaskError> {
        let stdout = match child.stdout.take() {
            Some(out) => BufReader::new(out).lines(),
            None => {
                let msg = "Failed to take stdout of child process";
                #[cfg(feature = "tracing")]
                tracing::error!(msg);

                self.set_state(TaskState::Finished);
                let error_event = TaskEvent::Error {
                    error: TaskError::IO(msg.to_string()),
                };
                self.send_event(event_tx, error_event).await;

                return Err(TaskError::IO(msg.to_string()));
            }
        };

        let stderr = match child.stderr.take() {
            Some(err) => BufReader::new(err).lines(),
            None => {
                let msg = "Failed to take stderr of child process";
                #[cfg(feature = "tracing")]
                tracing::error!(msg);

                self.set_state(TaskState::Finished);
                let error_event = TaskEvent::Error {
                    error: TaskError::IO(msg.to_string()),
                };
                self.send_event(event_tx, error_event).await;

                return Err(TaskError::IO(msg.to_string()));
            }
        };

        Ok((stdout, stderr))
    }

    pub(crate) async fn handle_output(
        &mut self,
        src: StreamSource,
        line: Result<Option<String>, std::io::Error>,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) {
        let line = match line {
            Ok(Some(l)) => l,
            Ok(None) => {
                // EOF reached
                return;
            }
            Err(e) => {
                let msg = format!("Error reading stdout: {}", e);
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Error reading stdout");
                self.set_state(TaskState::Finished);

                let error_event = TaskEvent::Error {
                    error: TaskError::IO(msg.clone()),
                };
                self.flags.stop = true;
                self.stop_reason = Some(TaskStopReason::Error(msg.clone()));
                self.send_event(event_tx, error_event).await;
                return;
            }
        };
        let event = TaskEvent::Output {
            line: line.clone(),
            src: src.clone(),
        };
        self.send_event(event_tx, event).await;

        if self.flags.ready {
            return;
        }
        let ready_indicator_source = match &self.config.ready_indicator_source {
            Some(ind) => ind,
            None => &StreamSource::default(),
        };
        if ready_indicator_source != &src {
            return;
        }
        let ready_indicator = match &self.config.ready_indicator {
            Some(text) => text,
            None => return,
        };

        if line.contains(ready_indicator) {
            self.flags.ready = true;
            self.set_state(TaskState::Ready);
            let ready_event = TaskEvent::Ready;
            self.send_event(event_tx, ready_event).await;
        }
    }
}

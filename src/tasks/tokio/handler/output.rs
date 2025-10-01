use crate::tasks::{
    config::StreamSource,
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    state::TaskState,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};
use std::sync::Arc;
use tokio::sync::mpsc;

impl TaskExecutor {
    /// Handles output from stdout/stderr streams.
    ///
    /// Processes each line of output from the child process, emits output events,
    /// and checks for ready indicators. Returns true if the stream should be closed.
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context
    /// * `line` - Result containing the line read from the stream
    /// * `event_tx` - Channel for sending task events
    /// * `src` - Source of the output (stdout/stderr)
    ///
    /// # Returns
    ///
    /// * `true` - If the stream should be closed (EOF or error)
    /// * `false` - If the stream should continue reading
    pub(crate) async fn handle_output(
        shared_context: Arc<TaskExecutorContext>,
        line: Result<Option<String>, std::io::Error>,
        event_tx: &mpsc::Sender<TaskEvent>,
        src: StreamSource,
    ) -> bool {
        let line = match line {
            Ok(Some(l)) => l,
            Ok(None) => {
                // EOF reached
                return true;
            }
            Err(e) => {
                let msg = format!("Error reading stdout: {}", e);

                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Error reading stdout");

                let error = TaskError::IO(msg);
                shared_context
                    .set_stop_reason(TaskStopReason::Error(error.clone()))
                    .await;

                let error_event = TaskEvent::Error { error };
                Self::send_event(event_tx, error_event).await;
                shared_context
                    .send_terminate_oneshot(TaskTerminateReason::InternalError)
                    .await;
                return true;
            }
        };
        let event = TaskEvent::Output {
            line: line.clone(),
            src: src.clone(),
        };
        Self::send_event(event_tx, event).await;

        if shared_context.get_ready_flag() {
            return false;
        }

        if shared_context.config.ready_indicator_source != Some(src) {
            return false;
        }
        let ready_indicator = match &shared_context.config.ready_indicator {
            Some(text) => text,
            None => return false,
        };

        if line.contains(ready_indicator) {
            shared_context.set_ready_flag(true);
            shared_context.set_state(TaskState::Ready);
            Self::send_event(event_tx, TaskEvent::Ready).await;
        }
        return false;
    }
}

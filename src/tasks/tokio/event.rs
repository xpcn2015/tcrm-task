use tokio::sync::mpsc;

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;
use crate::tasks::{
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn send_event(event_tx: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
        if (event_tx.send(event.clone()).await).is_err() {
            #[cfg(feature = "tracing")]
            tracing::warn!(event = ?event, "Event channel closed");
        }
    }
    pub(crate) async fn send_error_event_and_stop(
        &mut self,
        error: TaskError,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) {
        let time = Self::update_state(&self.shared_context, TaskState::Finished);
        let error_event = TaskEvent::Error {
            error: error.clone(),
        };
        Self::send_event(event_tx, error_event).await;

        let finish_event = TaskEvent::Stopped {
            exit_code: None,
            finished_at: time,
            reason: TaskStopReason::Error(error.clone()),
        };
        Self::send_event(event_tx, finish_event).await;
        self.shared_context
            .set_stop_reason(TaskStopReason::Error(error))
            .await;
    }
}

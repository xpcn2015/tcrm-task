use std::time::Instant;

use tokio::sync::mpsc;

use crate::tasks::{
    control::TaskInternal,
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn handle_result(&mut self, event_tx: &mpsc::Sender<TaskEvent>) {
        let reason = match self.stop_reason.clone() {
            Some(r) => r,
            None => {
                // This should not happen, but just in case
                let msg = "Task finished without a stop reason";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                let reason = TaskStopReason::Error(msg.to_string());
                self.stop_reason = Some(reason.clone());
                reason
            }
        };
        self.set_state(TaskState::Finished);
        self.finished_at = Some(Instant::now());
        let event = TaskEvent::Stopped {
            exit_code: self.exit_code,
            reason: reason,
        };
        self.send_event(event_tx, event).await;
    }
}

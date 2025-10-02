use std::sync::Arc;

use tokio::sync::mpsc;

use crate::tasks::{
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    /// Handles the final result of task execution.
    ///
    /// Processes the task completion, ensures proper cleanup (including
    /// process group termination if configured), and sends the final
    /// stopped event with the appropriate reason and exit code.
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context
    /// * `event_tx` - Channel for sending task events
    pub(crate) async fn handle_result(
        shared_context: Arc<TaskExecutorContext>,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) {
        let reason = shared_context.get_stop_reason().await;
        let reason = match reason {
            Some(r) => r,
            None => {
                // This should not happen, but just in case
                let msg = "Task finished without a stop reason";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                TaskStopReason::Error(TaskError::Channel(msg.to_string()))
            }
        };

        // If configured to use process group, ensure all child processes are terminated
        #[cfg(feature = "process-group")]
        if shared_context.config.use_process_group.unwrap_or_default() {
            if let Err(e) = shared_context.group.lock().await.stop_group() {
                let msg = format!("Failed to terminate process group: {}", e);

                #[cfg(feature = "tracing")]
                tracing::error!(error=%e, "{}", msg);

                let error = TaskError::Control(msg);
                let event = TaskEvent::Error { error };
                Self::send_event(&event_tx, event).await;
            };
        }
        let time = shared_context.set_task_state(TaskState::Finished);
        let exit_code = shared_context.get_exit_code();
        shared_context.set_process_id(0);
        let event = TaskEvent::Stopped {
            exit_code,
            reason: reason,
            finished_at: time,
            #[cfg(unix)]
            signal: shared_context.get_terminate_signal_code(),
        };

        Self::send_event(&event_tx, event).await;
    }
}

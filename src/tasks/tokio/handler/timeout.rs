use std::{sync::Arc, time::Duration};

use futures::future::{BoxFuture, pending};

use crate::tasks::{
    event::TaskTerminateReason,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    /// Creates a timeout future based on the task configuration.
    ///
    /// Returns a pinned future that completes when the configured timeout
    /// duration elapses. If no timeout is configured, returns a future
    /// that never completes (pending forever).
    ///
    /// This future should be created once and reused across select! iterations
    /// to prevent the timeout from being reset on each loop iteration.
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context containing timeout configuration
    pub(crate) fn create_timeout_future(
        shared_context: Arc<TaskExecutorContext>,
    ) -> BoxFuture<'static, ()> {
        if let Some(ms) = shared_context.config.timeout_ms {
            Box::pin(tokio::time::sleep(Duration::from_millis(ms)))
        } else {
            Box::pin(pending::<()>())
        }
    }

    /// Handles timeout expiration by sending a termination signal.
    ///
    /// Called when the configured timeout duration has elapsed. Sends
    /// a timeout termination reason to trigger task cleanup.
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context
    pub(crate) async fn handle_timeout(shared_context: Arc<TaskExecutorContext>) {
        shared_context
            .send_terminate_oneshot(TaskTerminateReason::Timeout)
            .await;
    }
}

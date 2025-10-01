use std::{sync::Arc, time::Duration};

use futures::future::pending;

use crate::tasks::{
    event::TaskTerminateReason,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    /// Sets up a timeout based on the task configuration.
    ///
    /// Creates a sleep future that completes when the configured timeout
    /// duration elapses. If no timeout is configured, this future will
    /// never complete (pending forever).
    ///
    /// # Arguments
    ///
    /// * `shared_context` - Shared task execution context containing timeout configuration
    /// * `timeout_triggered` - Mutable reference to track if timeout has been triggered
    pub(crate) async fn set_timeout_from_config(
        shared_context: Arc<TaskExecutorContext>,
        timeout_triggered: &mut bool,
    ) {
        if let Some(ms) = shared_context.config.timeout_ms
            && !*timeout_triggered
        {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            *timeout_triggered = true;
        } else {
            pending::<()>().await;
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

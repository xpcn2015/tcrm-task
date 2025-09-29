use std::{sync::Arc, time::Duration};

use futures::future::pending;

use crate::tasks::{
    event::TaskTerminateReason,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    pub(crate) async fn set_timeout_from_config(shared_context: Arc<TaskExecutorContext>) {
        if let Some(ms) = shared_context.config.timeout_ms {
            tokio::time::sleep(Duration::from_millis(ms)).await;
        } else {
            pending::<()>().await;
        }
    }
    pub(crate) async fn handle_timeout(shared_context: Arc<TaskExecutorContext>) {
        shared_context
            .send_terminate_signal(TaskTerminateReason::Timeout)
            .await;
    }
}

use std::sync::Arc;

use tokio::sync::oneshot::error::RecvError;

use crate::tasks::{
    event::{TaskStopReason, TaskTerminateReason},
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    pub(crate) async fn handle_terminate(
        shared_context: Arc<TaskExecutorContext>,
        reason: Result<TaskTerminateReason, RecvError>,
        stop: &mut bool,
    ) {
        #[cfg(feature = "tracing")]
        tracing::debug!(?reason, "Terminate signal received");

        let reason = match reason {
            Ok(r) => r,
            Err(_) => {
                *stop = true;
                #[cfg(feature = "tracing")]
                tracing::warn!("Terminate channel closed unexpectedly");
                return;
            }
        };

        *stop = true;
        let stop_reason = shared_context.get_stop_reason().await;
        if stop_reason.is_some() {
            return;
        }
        shared_context
            .set_stop_reason(TaskStopReason::Terminated(reason))
            .await;
    }
}

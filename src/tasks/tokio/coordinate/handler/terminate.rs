use std::sync::Arc;

use tokio::sync::{Mutex, oneshot::error::RecvError};

use crate::tasks::{
    event::{TaskStopReason, TaskTerminateReason},
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn handle_terminate(
        reason: Result<TaskTerminateReason, RecvError>,
        stop_reason: &Arc<Mutex<Option<TaskStopReason>>>,
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
        let mut stop_reason = stop_reason.lock().await;
        if stop_reason.is_some() {
            return;
        }
        *stop_reason = Some(TaskStopReason::Terminated(reason));
    }
}

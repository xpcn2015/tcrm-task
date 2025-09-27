use tokio::sync::oneshot::error::RecvError;

use crate::tasks::{
    event::{TaskStopReason, TaskTerminateReason},
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn handle_terminate(
        &mut self,
        reason: Result<TaskTerminateReason, RecvError>,
    ) {
        #[cfg(feature = "tracing")]
        tracing::debug!(?reason, "Terminate signal received");

        let reason = match reason {
            Ok(r) => r,
            Err(_) => {
                self.flags.stop = true;
                #[cfg(feature = "tracing")]
                tracing::warn!("Terminate channel closed unexpectedly");
                return;
            }
        };

        self.flags.stop = true;
        self.stop_reason = Some(TaskStopReason::Terminated(reason));
    }
}

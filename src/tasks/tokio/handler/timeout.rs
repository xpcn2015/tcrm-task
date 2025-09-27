use std::time::Duration;

use futures::future::pending;

use crate::tasks::{
    control::TaskControl, event::TaskTerminateReason, tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    pub(crate) async fn set_timeout_from_config(&self) {
        if let Some(ms) = self.config.timeout_ms {
            tokio::time::sleep(Duration::from_millis(ms)).await;
        } else {
            pending::<()>().await;
        }
    }
    pub(crate) fn handle_timeout(&mut self) {
        let _ = self.terminate(TaskTerminateReason::Timeout);
    }
}

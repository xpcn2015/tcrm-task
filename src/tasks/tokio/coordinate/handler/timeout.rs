use std::{sync::Arc, time::Duration};

use futures::future::pending;
use tokio::sync::{Mutex, oneshot};

use crate::tasks::{event::TaskTerminateReason, tokio::executor::TaskExecutor};

impl TaskExecutor {
    pub(crate) async fn set_timeout_from_config(timeout_ms: &Option<u64>) {
        if let Some(ms) = timeout_ms {
            tokio::time::sleep(Duration::from_millis(*ms)).await;
        } else {
            pending::<()>().await;
        }
    }
    pub(crate) async fn handle_timeout(
        internal_terminate_tx: &Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    ) {
        Self::internal_terminate(internal_terminate_tx, TaskTerminateReason::Timeout).await;
    }
}

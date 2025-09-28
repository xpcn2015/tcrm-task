use std::{
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
};

use tokio::sync::Mutex;

use crate::tasks::{error::TaskError, event::TaskStopReason, tokio::executor::TaskExecutor};

impl TaskExecutor {
    pub(crate) async fn handle_wait_result(
        result: Result<ExitStatus, std::io::Error>,
        stop_reason: &Arc<Mutex<Option<TaskStopReason>>>,
        exit_code_store: &Arc<AtomicI32>,
        stop: &mut bool,
    ) {
        #[cfg(feature = "tracing")]
        tracing::trace!("child process finished");
        match result {
            Ok(status) => {
                let exit_code = status.code();
                exit_code_store.store(exit_code.unwrap_or(0), Ordering::SeqCst);
                *stop_reason.lock().await = Some(TaskStopReason::Finished);
                #[cfg(feature = "tracing")]
                tracing::debug!(exit_code = ?exit_code, "Child process finished normally");
            }
            Err(e) => {
                // Expected OS level error
                *stop_reason.lock().await = Some(TaskStopReason::Error(TaskError::IO(format!(
                    "Failed to wait for child process: {}",
                    e
                ))));

                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Child process wait failed");
            }
        }

        *stop = true;
    }
}

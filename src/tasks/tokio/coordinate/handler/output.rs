use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout},
    sync::{Mutex, mpsc, oneshot},
};

use crate::tasks::{
    config::StreamSource,
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    state::TaskState,
    tokio::executor::TaskExecutor,
};
pub(crate) struct OutputArgs {
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) stop_reason: Arc<Mutex<Option<TaskStopReason>>>,
    pub(crate) ready_flag: Arc<AtomicBool>,
    pub(crate) ready_indicator_source: StreamSource,
    pub(crate) ready_indicator: Option<String>,
    pub(crate) src: StreamSource,
    pub(crate) event_tx: mpsc::Sender<TaskEvent>,
    pub(crate) internal_terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
}
impl TaskExecutor {
    pub(crate) async fn take_std_output_reader(
        &mut self,
        child: &mut Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(Lines<BufReader<ChildStdout>>, Lines<BufReader<ChildStderr>>), TaskError> {
        let stdout = match child.stdout.take() {
            Some(out) => BufReader::new(out).lines(),
            None => {
                let msg = "Failed to take stdout of child process";
                #[cfg(feature = "tracing")]
                tracing::error!(msg);

                let error = TaskError::IO(msg.to_string());
                self.send_error_event_and_stop(error.clone(), event_tx)
                    .await;

                return Err(TaskError::IO(msg.to_string()));
            }
        };

        let stderr = match child.stderr.take() {
            Some(err) => BufReader::new(err).lines(),
            None => {
                let msg = "Failed to take stderr of child process";
                #[cfg(feature = "tracing")]
                tracing::error!(msg);

                let error = TaskError::IO(msg.to_string());
                self.send_error_event_and_stop(error.clone(), event_tx)
                    .await;

                return Err(TaskError::IO(msg.to_string()));
            }
        };

        Ok((stdout, stderr))
    }

    pub(crate) async fn handle_output(
        args: &OutputArgs,
        line: Result<Option<String>, std::io::Error>,
    ) {
        let line = match line {
            Ok(Some(l)) => l,
            Ok(None) => {
                // EOF reached
                return;
            }
            Err(e) => {
                let msg = format!("Error reading stdout: {}", e);
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Error reading stdout");
                let error = TaskError::IO(msg);
                *args.stop_reason.lock().await = Some(TaskStopReason::Error(error.clone()));
                let error_event = TaskEvent::Error { error };
                Self::send_event(&args.event_tx, error_event).await;
                Self::internal_terminate(
                    &args.internal_terminate_tx,
                    TaskTerminateReason::InternalError,
                )
                .await;
                return;
            }
        };
        let event = TaskEvent::Output {
            line: line.clone(),
            src: args.src.clone(),
        };
        Self::send_event(&args.event_tx, event).await;

        if args.ready_flag.load(Ordering::SeqCst) {
            return;
        }

        if args.ready_indicator_source != args.src {
            return;
        }
        let ready_indicator = match &args.ready_indicator {
            Some(text) => text,
            None => return,
        };

        if line.contains(ready_indicator) {
            args.ready_flag.store(true, Ordering::SeqCst);
            Self::set_state(args.state.clone(), TaskState::Ready, None);
            Self::send_event(&args.event_tx, TaskEvent::Ready).await;
        }
    }
}

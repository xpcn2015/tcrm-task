use std::sync::atomic::Ordering;

use tokio::{
    process::{Child, Command},
    sync::{mpsc, oneshot},
};

use crate::tasks::{
    config::StreamSource,
    control::TaskStatusInfo,
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    state::TaskState,
    tokio::{
        coordinate::handler::{output::OutputArgs, result::ResultArgs},
        executor::TaskExecutor,
    },
};

impl TaskExecutor {
    pub async fn coordinate_start(
        &mut self,
        event_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        Self::set_state(
            self.state.clone(),
            TaskState::Initiating,
            Some(self.created_at.clone()),
        );
        self.validate_config(&event_tx).await?;

        let cmd = self.setup_command();
        #[cfg(feature = "process-group")]
        let cmd = self.setup_process_group(cmd).await?;

        let mut child = self.spawn_child(cmd, &event_tx).await?;
        self.store_stdin(&mut child, &event_tx).await?;

        // Assign the child process to the process group if enabled
        //       #[cfg(feature = "process-group")]

        let (mut stdout, mut stderr) = self.take_std_output_reader(&mut child, &event_tx).await?;
        let (terminate_tx, mut terminate_rx) = oneshot::channel::<TaskTerminateReason>();
        self.terminate_tx = Some(terminate_tx);
        let (internal_terminate_tx, mut internal_terminate_rx) =
            oneshot::channel::<TaskTerminateReason>();
        *self.internal_terminate_tx.lock().await = Some(internal_terminate_tx);

        let (stdout_args, stderr_args) = self.get_std_output_args(&event_tx);
        let result_args = self.get_result_args(&event_tx);
        let timeout_ms = self.config.timeout_ms.clone();
        let internal_terminate_tx = self.internal_terminate_tx.clone();
        let stop_reason = self.stop_reason.clone();
        let exit_code = self.exit_code.clone();

        tokio::spawn(async move {
            let mut stop = false;
            loop {
                if stop {
                    break;
                }
                tokio::select! {
                    line = stdout.next_line() => Self::handle_output(&stdout_args, line).await,
                    line = stderr.next_line() => Self::handle_output(&stderr_args, line).await,
                    _ = Self::set_timeout_from_config(&timeout_ms) => Self::handle_timeout(&internal_terminate_tx).await,
                    reason = &mut terminate_rx => Self::handle_terminate(reason, &stop_reason, &mut stop).await,
                    reason = &mut internal_terminate_rx => Self::handle_terminate(reason, &stop_reason, &mut stop).await,
                    result = child.wait() => Self::handle_wait_result(result, &stop_reason, &exit_code, &mut stop).await,
                }
            }
            Self::handle_result(child, result_args).await;
        });
        Ok(())
    }

    fn get_std_output_args(&self, event_tx: &mpsc::Sender<TaskEvent>) -> (OutputArgs, OutputArgs) {
        let stdout_args = OutputArgs {
            state: self.state.clone(),
            stop_reason: self.stop_reason.clone(),
            ready_flag: self.ready_flag.clone(),
            ready_indicator_source: self
                .config
                .ready_indicator_source
                .clone()
                .unwrap_or_default(),
            ready_indicator: self.config.ready_indicator.clone(),
            src: StreamSource::Stdout,
            event_tx: event_tx.clone(),
            internal_terminate_tx: self.internal_terminate_tx.clone(),
        };
        let stderr_args = OutputArgs {
            state: self.state.clone(),
            stop_reason: self.stop_reason.clone(),
            ready_flag: self.ready_flag.clone(),
            ready_indicator_source: self
                .config
                .ready_indicator_source
                .clone()
                .unwrap_or_default(),
            ready_indicator: self.config.ready_indicator.clone(),
            src: StreamSource::Stderr,
            event_tx: event_tx.clone(),
            internal_terminate_tx: self.internal_terminate_tx.clone(),
        };
        (stdout_args, stderr_args)
    }
    fn get_result_args(&self, event_tx: &mpsc::Sender<TaskEvent>) -> ResultArgs {
        ResultArgs {
            event_tx: event_tx.clone(),
            stop_reason: self.stop_reason.clone(),
            process_id: self.process_id.clone(),
            state: self.state.clone(),
            finished_at: self.finished_at.clone(),
            exit_code: self.exit_code.clone(),

            #[cfg(feature = "process-group")]
            use_process_group: self.config.use_process_group.clone(),
            #[cfg(feature = "process-group")]
            group: self.group.clone(),
        }
    }
    async fn spawn_child(
        &mut self,
        mut cmd: Command,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<Child, TaskError> {
        match cmd.spawn() {
            Ok(child) => {
                self.update_state_after_spawn(&child, event_tx).await?;

                Ok(child)
            }
            Err(e) => {
                let msg = format!("Failed to spawn child process: {}", e);
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Failed to spawn child process");
                let error = TaskError::IO(msg);
                let time = Self::set_state(
                    self.state.clone(),
                    TaskState::Finished,
                    Some(self.finished_at.clone()),
                );
                let error_event = TaskEvent::Error {
                    error: error.clone(),
                };

                Self::send_event(event_tx, error_event).await;

                let finish_event = TaskEvent::Stopped {
                    exit_code: None,
                    finished_at: time,
                    reason: TaskStopReason::Error(error.clone()),
                };
                Self::send_event(event_tx, finish_event).await;

                Err(error)
            }
        }
    }
    async fn update_state_after_spawn(
        &mut self,
        child: &Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        let Some(pid) = child.id() else {
            let msg = "Failed to get process id";

            #[cfg(feature = "tracing")]
            tracing::error!(msg);
            let error = TaskError::Handle(msg.to_string());

            self.send_error_event_and_stop(error.clone(), event_tx)
                .await;

            return Err(error);
        };
        self.process_id.store(pid, Ordering::SeqCst);

        let time = Self::set_state(
            self.state.clone(),
            TaskState::Running,
            Some(self.running_at.clone()),
        );

        Self::send_event(
            event_tx,
            TaskEvent::Started {
                process_id: pid,
                created_at: self.get_create_at(),
                running_at: time,
            },
        )
        .await;
        Ok(())
    }

    #[cfg(feature = "process-group")]
    async fn setup_process_group(&self, cmd: Command) -> Result<Command, TaskError> {
        if !self.config.use_process_group.unwrap_or_default() {
            return Ok(cmd);
        }
        Ok(cmd)
    }
}

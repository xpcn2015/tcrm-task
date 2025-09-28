use std::process::Stdio;

use tokio::{
    process::{Child, Command},
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::tasks::{
    config::StreamSource,
    control::{TaskInternal, TaskStatusInfo},
    error::TaskError,
    event::{TaskEvent, TaskTerminateReason},
    state::TaskState,
    tokio::select::executor::TaskExecutor,
};

impl TaskExecutor {
    pub async fn spawn_start(
        mut self,
        event_tx: mpsc::Sender<TaskEvent>,
    ) -> JoinHandle<Result<(), TaskError>> {
        tokio::spawn(async move { self.start(event_tx).await })
    }
    pub async fn start(&mut self, event_tx: mpsc::Sender<TaskEvent>) -> Result<(), TaskError> {
        self.set_state(TaskState::Initiating);
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

        loop {
            if self.flags.stop {
                break;
            }
            tokio::select! {
                line = stdout.next_line() => self.handle_output(StreamSource::Stdout, line, &event_tx).await,
                line = stderr.next_line() => self.handle_output(StreamSource::Stderr, line, &event_tx).await,
                _ = self.set_timeout_from_config() => self.handle_timeout(),
                result = child.wait() => self.handle_wait_result(result).await,
                reason = &mut terminate_rx => self.handle_terminate(reason).await,
            }
        }
        self.handle_result(child, &event_tx).await;
        Ok(())
    }
    async fn validate_config(
        &mut self,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        match self.config.validate() {
            Ok(()) => Ok(()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Invalid task configuration");

                let time = self.set_state(TaskState::Finished);
                let error_event = TaskEvent::Error {
                    error: e.clone(),
                    finished_at: time,
                };
                self.send_event(event_tx, error_event).await;

                return Err(e);
            }
        }
    }
    fn setup_command(&self) -> Command {
        let mut cmd = Command::new(&self.config.command);

        cmd.kill_on_drop(true);

        // Setup additional arguments
        if let Some(args) = &self.config.args {
            cmd.args(args);
        }

        // Setup working directory with validation
        if let Some(dir) = &self.config.working_dir {
            cmd.current_dir(dir);
        }

        // Setup environment variables
        if let Some(envs) = &self.config.env {
            cmd.envs(envs);
        }

        // Setup stdio
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(
            if self.config.enable_stdin.unwrap_or_default() {
                Stdio::piped()
            } else {
                Stdio::null()
            },
        );
        cmd
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

                let time = self.set_state(TaskState::Finished);
                let error_event = TaskEvent::Error {
                    error: TaskError::IO(msg.clone()),
                    finished_at: time,
                };

                self.send_event(event_tx, error_event).await;
                Err(TaskError::IO(msg))
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

            let time = self.set_state(TaskState::Finished);
            let error_event = TaskEvent::Error {
                error: TaskError::Handle(msg.to_string()),
                finished_at: time,
            };

            self.send_event(event_tx, error_event).await;

            return Err(TaskError::Handle(msg.to_string()));
        };
        self.process_id = Some(pid);
        let time = self.set_state(TaskState::Running);

        self.send_event(
            event_tx,
            TaskEvent::Started {
                process_id: pid,
                created_at: *self.get_create_at(),
                running_at: time,
            },
        )
        .await;
        Ok(())
    }

    pub(crate) async fn send_event(&self, event_tx: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
        if (event_tx.send(event.clone()).await).is_err() {
            #[cfg(feature = "tracing")]
            tracing::warn!(event = ?event, "Event channel closed");
        }
    }

    #[cfg(feature = "process-group")]
    async fn setup_process_group(&self, cmd: Command) -> Result<Command, TaskError> {
        if !self.config.use_process_group.unwrap_or_default() {
            return Ok(cmd);
        }
        Ok(cmd)
    }
}

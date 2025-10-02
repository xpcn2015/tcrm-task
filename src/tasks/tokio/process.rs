use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::mpsc,
};

use crate::tasks::{
    control::TaskStatusInfo,
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    /// Tries to store the process ID from a spawned child process.
    ///
    /// Validates that a process ID was successfully obtained and stores it
    /// in the shared context for later use in process management.
    ///
    /// # Arguments
    ///
    /// * `pid` - Optional process ID from the spawned child
    ///
    /// # Returns
    ///
    /// * `Ok(u32)` - The validated process ID
    /// * `Err(TaskError)` - If no process ID was provided
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Handle`] if the process ID is None
    pub(crate) fn try_store_process_id(&self, pid: Option<u32>) -> Result<u32, TaskError> {
        let Some(pid) = pid else {
            let msg = "Failed to get process id";

            #[cfg(feature = "tracing")]
            tracing::error!(msg);

            let error = TaskError::Handle(msg.to_string());
            return Err(error);
        };
        self.shared_context.set_process_id(pid);
        Ok(pid)
    }
    /// Spawns a child process and handles the result.
    ///
    /// Attempts to spawn the configured command and stores the process ID.
    /// If spawning fails, appropriate error events are sent.
    ///
    /// On Windows with process groups enabled, spawns the process in a suspended state,
    /// assigns it to the job object, then resumes it to avoid the race condition where
    /// child processes could escape the job object.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The configured command to spawn
    /// * `event_tx` - Channel for sending task events
    ///
    /// # Returns
    ///
    /// * `Ok(Child)` - The spawned child process
    /// * `Err(TaskError)` - If spawning fails
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::IO`] if process spawning fails
    pub(crate) async fn spawn_child(
        &mut self,
        mut cmd: Command,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<Child, TaskError> {
        let use_pg = self
            .shared_context
            .config
            .use_process_group
            .unwrap_or_default();

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = match self.try_store_process_id(child.id()) {
                    Ok(pid) => pid,
                    Err(e) => {
                        let _ = child.kill().await;

                        self.send_error_event_and_stop(e.clone(), event_tx).await;
                        return Err(e);
                    }
                };

                // Assign the child process to the process group if enabled
                #[cfg(feature = "process-group")]
                if use_pg {
                    let result = self.shared_context.group.lock().await.assign_child(pid);
                    if let Err(e) = result {
                        let msg = format!("Failed to add process to group: {}", e);

                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "Failed to add process to group");

                        let _ = child.kill().await;

                        let error = TaskError::Handle(msg);
                        self.send_error_event_and_stop(error.clone(), event_tx)
                            .await;
                        return Err(error);
                    }
                    // Resume the process on Windows if it was suspended
                    #[cfg(all(windows, feature = "process-group"))]
                    {
                        let mut error = None;
                        {
                            let group = self.shared_context.group.lock().await;
                            let result = group.resume_process(pid);
                            if let Err(e) = result {
                                let msg = format!("Failed to resume process: {}", e);

                                #[cfg(feature = "tracing")]
                                tracing::error!(error=%e, "Failed to resume process");

                                let _ = child.kill().await;

                                error = Some(TaskError::Handle(msg));
                            }
                        }

                        if let Some(error) = error {
                            self.send_error_event_and_stop(error.clone(), event_tx)
                                .await;
                            return Err(error);
                        }
                    }
                }

                let time = Self::update_state(&self.shared_context, TaskState::Running);
                Self::send_event(
                    event_tx,
                    TaskEvent::Started {
                        process_id: pid,
                        created_at: self.get_create_at(),
                        running_at: time,
                    },
                )
                .await;
                Ok(child)
            }
            Err(e) => {
                let msg = format!("Failed to spawn child process: {}", e);
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Failed to spawn child process");
                let error = TaskError::IO(msg);
                let time = Self::update_state(&self.shared_context, TaskState::Finished);
                let error_event = TaskEvent::Error {
                    error: error.clone(),
                };

                Self::send_event(event_tx, error_event).await;

                let finish_event = TaskEvent::Stopped {
                    exit_code: None,
                    finished_at: time,
                    reason: TaskStopReason::Error(error.clone()),
                    #[cfg(unix)]
                    signal: None,
                };
                Self::send_event(event_tx, finish_event).await;

                Err(error)
            }
        }
    }
    /// Takes stdout and stderr readers from a child process.
    ///
    /// Extracts the stdout and stderr streams from the child process and
    /// converts them into line readers for processing output.
    ///
    /// # Arguments
    ///
    /// * `child` - The child process to extract streams from
    /// * `event_tx` - Channel for sending error events if extraction fails
    ///
    /// # Returns
    ///
    /// * `Ok((Lines<BufReader<ChildStdout>>, Lines<BufReader<ChildStderr>>))` - The stdout and stderr line readers
    /// * `Err(TaskError)` - If stream extraction fails
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Handle`] if stdout or stderr streams cannot be taken
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

    /// Stores the stdin handle from a child process for later use.
    ///
    /// Extracts and stores the stdin stream from the child process if stdin
    /// is enabled in the task configuration. This allows sending input to
    /// the process later via the send_stdin method.
    ///
    /// # Arguments
    ///
    /// * `child` - The child process to extract stdin from
    /// * `event_tx` - Channel for sending error events if extraction fails
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Stdin stored successfully or not required
    /// * `Err(TaskError)` - If stdin extraction fails when required
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Handle`] if stdin cannot be taken from the child process
    /// when stdin is enabled in the configuration
    pub(crate) async fn store_stdin(
        &mut self,
        child: &mut Child,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        if !self.shared_context.config.enable_stdin.unwrap_or_default() {
            return Ok(());
        }

        if let Some(stdin) = child.stdin.take() {
            self.stdin = Some(stdin);
            Ok(())
        } else {
            let msg = "Failed to take stdin out of child process";
            #[cfg(feature = "tracing")]
            tracing::error!(msg);

            let error = TaskError::IO(msg.to_string());
            self.send_error_event_and_stop(error.clone(), event_tx)
                .await;

            Err(TaskError::IO(msg.to_string()))
        }
    }
    /// Sends input to the process's stdin.
    ///
    /// Writes the provided input to the process's stdin stream. The input
    /// will be automatically terminated with a newline if it doesn't already end with one.
    ///
    /// # Arguments
    ///
    /// * `input` - The input string to send to stdin
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the input was sent successfully
    /// * `Err(TaskError)` - If sending fails or the task is not running
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Control`] if the task is not in a running state,
    /// or [`TaskError::IO`] if writing to stdin fails
    pub async fn send_stdin(&mut self, input: impl Into<String>) -> Result<(), TaskError> {
        let state = self.get_task_state();
        if !matches!(state, TaskState::Running | TaskState::Ready) {
            return Err(TaskError::Control(
                "Cannot send stdin, task is not running".to_string(),
            ));
        }
        let mut input: String = input.into();
        if !input.ends_with('\n') {
            input.push('\n');
        }
        if let Some(stdin) = &mut self.stdin.as_mut() {
            #[allow(clippy::used_underscore_binding)]
            if let Err(_e) = stdin.write_all(input.as_bytes()).await {
                let msg = "Failed to write to child stdin";
                #[cfg(feature = "tracing")]
                tracing::warn!(error=%_e, msg);
                return Err(TaskError::Control(msg.to_string()));
            }
        } else {
            let msg = "Stdin is not available";
            #[cfg(feature = "tracing")]
            tracing::warn!(msg);
            return Err(TaskError::Control(msg.to_string()));
        }

        Ok(())
    }

    /// Sets up process group configuration for the command.
    ///
    /// Configures the command to run in a process group if process group
    /// support is enabled in the task configuration. This allows for
    /// coordinated termination of process trees.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to configure for process group execution
    ///
    /// # Returns
    ///
    /// * `Ok(Command)` - The configured command ready for spawning
    /// * `Err(TaskError)` - If process group setup fails
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Control`] if process group creation fails
    #[cfg(feature = "process-group")]
    pub(crate) async fn setup_process_group(&self, cmd: Command) -> Result<Command, TaskError> {
        if !self
            .shared_context
            .config
            .use_process_group
            .unwrap_or_default()
        {
            return Ok(cmd);
        }
        let mut group = self.shared_context.group.lock().await;
        let cmd = group.create_with_command(cmd).map_err(|e| {
            let msg = format!("Failed to create process group: {}", e);
            #[cfg(feature = "tracing")]
            tracing::error!(error=%e, "{}", msg);

            TaskError::Control(msg)
        })?;
        Ok(cmd)
    }
}

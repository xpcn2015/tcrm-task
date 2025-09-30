use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::mpsc,
};

use crate::tasks::{
    control::{TaskControlAction, TaskStatusInfo},
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    process::child::terminate_process,
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
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
    pub(crate) async fn spawn_child(
        &mut self,
        mut cmd: Command,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<Child, TaskError> {
        match cmd.spawn() {
            Ok(child) => {
                let pid = match self.try_store_process_id(child.id()) {
                    Ok(pid) => pid,
                    Err(e) => {
                        self.send_error_event_and_stop(e.clone(), event_tx).await;
                        return Err(e);
                    }
                };

                // Assign the child process to the process group if enabled
                #[cfg(feature = "process-group")]
                if self
                    .shared_context
                    .config
                    .use_process_group
                    .unwrap_or_default()
                {
                    let result = self.shared_context.group.lock().await.assign_child(pid);
                    if let Err(e) = result {
                        let msg = format!("Failed to add process to group: {}", e);

                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "Failed to add process to group");

                        let error = TaskError::Handle(msg);
                        self.send_error_event_and_stop(error.clone(), event_tx)
                            .await;
                        return Err(error);
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
    pub async fn send_stdin(&mut self, input: impl Into<String>) -> Result<(), TaskError> {
        let state = self.get_state();
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

    pub async fn perform_process_action(
        &mut self,
        action: TaskControlAction,
    ) -> Result<(), TaskError> {
        #[cfg(feature = "process-group")]
        let use_process_group = self
            .shared_context
            .config
            .use_process_group
            .unwrap_or_default();
        #[cfg(not(feature = "process-group"))]
        let use_process_group = false;

        #[cfg(feature = "process-group")]
        let active = self.shared_context.group.lock().await.is_active();
        #[cfg(not(feature = "process-group"))]
        let active = false;
        let process_id = match self.shared_context.get_process_id() {
            Some(n) => n,
            None => {
                let msg = "No process ID available to perform action";
                #[cfg(feature = "tracing")]
                tracing::warn!(msg);
                return Err(TaskError::Control(msg.to_string()));
            }
        };
        match action {
            TaskControlAction::Terminate => {
                if use_process_group && active {
                    self.shared_context
                        .group
                        .lock()
                        .await
                        .terminate_group()
                        .map_err(|e| {
                            let msg = format!("Failed to terminate process group: {}", e);
                            #[cfg(feature = "tracing")]
                            tracing::error!(error=%e, "{}", msg);
                            TaskError::Control(msg)
                        })?;
                } else {
                    terminate_process(process_id).map_err(|e| {
                        let msg = format!("Failed to terminate process: {}", e);
                        #[cfg(feature = "tracing")]
                        tracing::error!(error=%e, "{}", msg);
                        TaskError::Control(msg)
                    })?;
                }
            }
            TaskControlAction::Pause => todo!(),
            TaskControlAction::Resume => todo!(),
            TaskControlAction::Interrupt => todo!(),
        }
        Ok(())
    }

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

use tokio::sync::{mpsc, oneshot};

use crate::tasks::{
    config::StreamSource,
    error::TaskError,
    event::{TaskEvent, TaskTerminateReason},
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskExecutor {
    /// Start coordinated process execution with event monitoring
    ///
    /// This is the main execution method that spawns the process, sets up event monitoring,
    /// and manages the complete process lifecycle. It handles stdout/stderr streaming,
    /// timeout management, termination signals, and process cleanup in a coordinated
    /// async event loop.
    ///
    /// # Arguments
    ///
    /// * `event_tx` - Channel sender for emitting [`TaskEvent`]s during process execution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Process coordination started successfully
    /// * `Err(TaskError)` - Configuration validation or process spawning failed
    ///
    /// # Errors
    ///
    /// Returns [`TaskError`] for:
    /// - [`TaskError::InvalidConfiguration`] - Configuration validation failed
    /// - [`TaskError::IO`] - Process spawning failed
    /// - [`TaskError::Handle`] - Process handle or watcher setup failed
    ///
    /// # Events Emitted
    ///
    /// During execution, the following events are sent via `event_tx`:
    /// - [`TaskEvent::Started`] - Process spawned successfully
    /// - [`TaskEvent::Output`] - Lines from stdout/stderr
    /// - [`TaskEvent::Ready`] - Ready indicator detected (if configured)
    /// - [`TaskEvent::Stopped`] - Process completed with exit code
    /// - [`TaskEvent::Error`] - Errors during execution
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, tokio::executor::TaskExecutor};
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     #[cfg(windows)]
    ///     let config = TaskConfig::new("cmd").args(["/C", "echo", "test"]);
    ///     #[cfg(unix)]
    ///     let config = TaskConfig::new("echo").args(["test"]);
    ///     
    ///     config.validate()?;
    ///     
    ///     let mut executor = TaskExecutor::new(config);
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///     
    ///     // Start coordination - returns immediately, process runs in background
    ///     executor.coordinate_start(tx).await?;
    ///     
    ///     // Process events until completion
    ///     while let Some(event) = rx.recv().await {
    ///         println!("Event: {:?}", event);
    ///         if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
    ///             break;
    ///         }
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## With Ready Indicator
    /// ```rust
    /// use tcrm_task::tasks::{
    ///     config::{TaskConfig, StreamSource},
    ///     tokio::executor::TaskExecutor,
    ///     event::TaskEvent
    /// };
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     #[cfg(windows)]
    ///     let config = TaskConfig::new("cmd")
    ///         .args(["/C", "echo", "Server ready"])
    ///         .ready_indicator("Server ready")
    ///         .ready_indicator_source(StreamSource::Stdout);
    ///     
    ///     #[cfg(unix)]
    ///     let config = TaskConfig::new("echo")
    ///         .args(["Server ready"])
    ///         .ready_indicator("Server ready")
    ///         .ready_indicator_source(StreamSource::Stdout);
    ///     
    ///     let mut executor = TaskExecutor::new(config);
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///     
    ///     executor.coordinate_start(tx).await?;
    ///     
    ///     while let Some(event) = rx.recv().await {
    ///         match event {
    ///             TaskEvent::Ready => {
    ///                 println!("Process is ready!");
    ///                 // Can now interact with the running process
    ///             }
    ///             TaskEvent::Stopped { .. } => break,
    ///             _ => {}
    ///         }
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn coordinate_start(
        &mut self,
        event_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        Self::update_state(&self.shared_context, TaskState::Initiating);
        self.validate_config(&event_tx).await?;

        let cmd = self.setup_command();

        #[cfg(feature = "process-group")]
        let cmd = self.setup_process_group(cmd).await?;

        let mut child = self.spawn_child(cmd, &event_tx).await?;
        self.store_stdin(&mut child, &event_tx).await?;

        let (mut stdout, mut stderr) = self.take_std_output_reader(&mut child, &event_tx).await?;
        let (terminate_tx, mut terminate_rx) = oneshot::channel::<TaskTerminateReason>();
        self.terminate_tx = Some(terminate_tx);

        let (internal_terminate_tx, mut internal_terminate_rx) =
            oneshot::channel::<TaskTerminateReason>();
        self.shared_context
            .set_internal_terminate_tx(internal_terminate_tx)
            .await;

        let shared_context = self.shared_context.clone();

        tokio::spawn(async move {
            let mut process_exited = false;
            let mut termination_requested = false;
            let mut stdout_eof = false;
            let mut stderr_eof = false;
            let mut timeout_triggered = false;
            loop {
                // Exit conditions
                if process_exited && stdout_eof && stderr_eof {
                    break;
                }

                // Force exit if termination was requested and streams are taking too long
                if termination_requested && stdout_eof && stderr_eof {
                    break;
                }
                tokio::select! {
                    line = stdout.next_line(), if !stdout_eof =>
                        stdout_eof = Self::handle_output(shared_context.clone(), line, &event_tx, StreamSource::Stdout).await,
                    line = stderr.next_line(), if !stderr_eof =>
                        stderr_eof = Self::handle_output(shared_context.clone(), line, &event_tx, StreamSource::Stderr).await,

                    _ = Self::set_timeout_from_config(shared_context.clone(), &mut timeout_triggered) => Self::handle_timeout(shared_context.clone()).await,

                    reason = Self::await_oneshot(&mut terminate_rx, termination_requested) =>
                        Self::handle_terminate(shared_context.clone(), &mut child, reason, &mut termination_requested).await,
                    reason = Self::await_oneshot(&mut internal_terminate_rx, termination_requested) =>
                        Self::handle_terminate(shared_context.clone(), &mut child, reason, &mut termination_requested).await,

                    result = child.wait() => Self::handle_wait_result(shared_context.clone(), result,&mut process_exited).await,
                }
            }
            Self::handle_result(shared_context.clone(), &event_tx).await;
        });
        Ok(())
    }
    async fn await_oneshot<T>(
        rx: &mut oneshot::Receiver<T>,
        termination_requested: bool,
    ) -> Result<T, oneshot::error::RecvError> {
        if termination_requested {
            std::future::pending().await
        } else {
            rx.await
        }
    }
}

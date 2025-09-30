use std::{process::Stdio, sync::Arc};

use tokio::{
    process::{ChildStdin, Command},
    sync::{mpsc, oneshot},
};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;
use crate::tasks::{
    config::TaskConfig,
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
    state::TaskState,
    tokio::context::TaskExecutorContext,
};

/// Task executor for managing process lifecycle
///
/// `TaskExecutor` is the main entry point for executing system processes with real-time
/// event monitoring, timeout management, and cross-platform process control.
/// It coordinates process spawning, I/O handling, and termination through an event-driven
/// architecture built on tokio.
///
/// # Examples
///
/// ## Basic Process Execution
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, tokio::executor::TaskExecutor};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "Hello, World!"]);
///     #[cfg(unix)]
///     let config = TaskConfig::new("echo").args(["Hello, World!"]);
///     
///     config.validate()?;
///     
///     let mut executor = TaskExecutor::new(config);
///     let (tx, mut rx) = mpsc::channel(100);
///     
///     executor.coordinate_start(tx).await?;
///     
///     while let Some(event) = rx.recv().await {
///         match event {
///             tcrm_task::tasks::event::TaskEvent::Output { line, .. } => {
///                 println!("Output: {}", line);
///             }
///             tcrm_task::tasks::event::TaskEvent::Stopped { .. } => break,
///             _ => {}
///         }
///     }
///     
///     Ok(())
/// }
/// ```
///
/// ## Process with Timeout and Termination
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     tokio::executor::TaskExecutor,
///     control::TaskControl,
///     event::TaskTerminateReason
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd")
///         .args(["/C", "timeout", "/t", "10"])
///         .timeout_ms(5000);
///     #[cfg(unix)]
///     let config = TaskConfig::new("sleep")
///         .args(["10"])
///         .timeout_ms(5000);
///     
///     let mut executor = TaskExecutor::new(config);
///     let (tx, mut rx) = mpsc::channel(100);
///     
///     executor.coordinate_start(tx).await?;
///     
///     // Terminate after 2 seconds
///     tokio::spawn(async move {
///         tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
///         let _ = executor.terminate_task(TaskTerminateReason::UserRequested);
///     });
///     
///     while let Some(event) = rx.recv().await {
///         if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
///             break;
///         }
///     }
///     
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct TaskExecutor {
    pub(crate) shared_context: Arc<TaskExecutorContext>,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) terminate_tx: Option<oneshot::Sender<TaskTerminateReason>>,
}

impl TaskExecutor {
    /// Create a new task executor with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Validated task configuration containing command, arguments, and options
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, tokio::executor::TaskExecutor};
    ///
    /// #[cfg(windows)]
    /// let config = TaskConfig::new("cmd").args(["/C", "dir"]);
    /// #[cfg(unix)]
    /// let config = TaskConfig::new("ls").args(["-la"]);
    ///
    /// let executor = TaskExecutor::new(config);
    /// ```
    pub fn new(config: TaskConfig) -> Self {
        Self {
            shared_context: Arc::new(TaskExecutorContext::new(config)),
            stdin: None,
            terminate_tx: None,
        }
    }

    /// Validates the task configuration before execution.
    ///
    /// Checks if the task configuration is valid and sends appropriate
    /// events if validation fails.
    ///
    /// # Arguments
    ///
    /// * `event_tx` - Channel for sending task events
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If configuration is valid
    /// * `Err(TaskError)` - If configuration validation fails
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::InvalidConfiguration`] if configuration is invalid
    pub(crate) async fn validate_config(
        &mut self,
        event_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(), TaskError> {
        match self.shared_context.config.validate() {
            Ok(()) => Ok(()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Invalid task configuration");

                let time = Self::update_state(&self.shared_context, TaskState::Finished);
                let error_event = TaskEvent::Error { error: e.clone() };
                Self::send_event(event_tx, error_event).await;

                let finish_event = TaskEvent::Stopped {
                    exit_code: None,
                    finished_at: time,
                    reason: TaskStopReason::Error(e.clone()),
                    #[cfg(unix)]
                    signal: None,
                };
                Self::send_event(event_tx, finish_event).await;

                return Err(e);
            }
        }
    }
    /// Sets up a command for execution based on the task configuration.
    ///
    /// Creates a tokio Command with all the configured parameters including
    /// arguments, environment variables, working directory, and I/O redirection.
    ///
    /// # Returns
    ///
    /// A configured `tokio::process::Command` ready for spawning
    pub(crate) fn setup_command(&self) -> Command {
        let mut cmd = Command::new(&self.shared_context.config.command);

        cmd.kill_on_drop(true);

        // Setup additional arguments
        if let Some(args) = &self.shared_context.config.args {
            cmd.args(args);
        }

        // Setup working directory with validation
        if let Some(dir) = &self.shared_context.config.working_dir {
            cmd.current_dir(dir);
        }

        // Setup environment variables
        if let Some(envs) = &self.shared_context.config.env {
            cmd.envs(envs);
        }

        // Setup stdio
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(
            if self.shared_context.config.enable_stdin.unwrap_or_default() {
                Stdio::piped()
            } else {
                Stdio::null()
            },
        );
        cmd
    }
}

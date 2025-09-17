use crate::tasks::{config::StreamSource, error::TaskError};

/// Events emitted during task execution lifecycle
///
/// `TaskEvent` represents all significant events that occur during task execution,
/// from process start to completion. These events enable real-time monitoring
/// and reactive programming patterns.
///
/// # Event Flow
///
/// A typical task execution emits events in this order:
/// 1. `Started` - Process has been spawned
/// 2. `Output` - Output lines from stdout/stderr (ongoing)
/// 3. `Ready` - Ready indicator detected (optional, for long-running processes)
/// 4. `Stopped` - Process has completed
/// 5. `Error` - If any error occurs during execution
///
/// # Examples
///
/// ## Basic Event Processing
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner, event::TaskEvent};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello", "world"]);
///     let mut spawner = TaskSpawner::new("demo".to_string(), config);
///     
///     let (tx, mut rx) = mpsc::channel(100);
///     spawner.start_direct(tx).await?;
///
///     while let Some(event) = rx.recv().await {
///         match event {
///             TaskEvent::Started { task_name } => {
///                 println!("Task '{}' started", task_name);
///             }
///             TaskEvent::Output { task_name, line, src } => {
///                 println!("Task '{}' output: {}", task_name, line);
///             }
///             TaskEvent::Stopped { task_name, exit_code, reason } => {
///                 println!("Task '{}' stopped with code {:?}", task_name, exit_code);
///                 break;
///             }
///             TaskEvent::Error { task_name, error } => {
///                 eprintln!("Task '{}' error: {}", task_name, error);
///                 break;
///             }
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Server Ready Detection
/// ```rust
/// use tcrm_task::tasks::{
///     config::{TaskConfig, StreamSource},
///     async_tokio::spawner::TaskSpawner,
///     event::TaskEvent
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd")
///         .args(["/C", "echo", "Server listening"])
///         .ready_indicator("Server listening")
///         .ready_indicator_source(StreamSource::Stdout);
///
///     let mut spawner = TaskSpawner::new("server".to_string(), config);
///     let (tx, mut rx) = mpsc::channel(100);
///     spawner.start_direct(tx).await?;
///
///     while let Some(event) = rx.recv().await {
///         match event {
///             TaskEvent::Ready { task_name } => {
///                 println!("Server '{}' is ready for requests!", task_name);
///                 // Server is now ready - can start sending requests
///                 break;
///             }
///             TaskEvent::Output { line, .. } => {
///                 println!("Server log: {}", line);
///             }
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskEvent {
    /// Process has been successfully spawned and is running
    ///
    /// This is the first event emitted after successful process creation.
    /// The process is now running and other events will follow.
    Started {
        /// Name of the task that started
        task_name: String,
    },

    /// Output line received from the process
    ///
    /// Emitted for each line of output from stdout or stderr.
    /// Lines are buffered and emitted when complete (on newline).
    Output {
        /// Name of the task that produced the output
        task_name: String,
        /// The output line (without trailing newline)
        line: String,
        /// Source stream (stdout or stderr)
        src: StreamSource,
    },

    /// Process has signaled it's ready to accept requests
    ///
    /// Only emitted for long-running processes that have a ready indicator configured.
    /// Indicates the process has completed initialization and is ready for work.
    Ready {
        /// Name of the task that became ready
        task_name: String,
    },

    /// Process has completed execution
    ///
    /// This is the final event for successful task execution.
    /// The process has exited and all resources have been cleaned up.
    Stopped {
        /// Name of the task that stopped
        task_name: String,
        /// Exit code from the process (None if terminated)
        exit_code: Option<i32>,
        /// Reason the process stopped
        reason: TaskEventStopReason,
    },

    /// An error occurred during task execution
    ///
    /// Emitted when errors occur during configuration validation,
    /// process spawning, or execution monitoring.
    Error {
        /// Name of the task that encountered an error
        task_name: String,
        /// The specific error that occurred
        error: TaskError,
    },
}

/// Reason why a task stopped executing
///
/// Provides detailed information about why a process completed,
/// whether due to natural completion, termination, or error.
///
/// # Examples
///
/// ```rust
/// use tcrm_task::tasks::{event::TaskEventStopReason, event::TaskTerminateReason};
///
/// // Natural completion
/// let reason = TaskEventStopReason::Finished;
///
/// // Terminated due to timeout
/// let reason = TaskEventStopReason::Terminated(TaskTerminateReason::Timeout);
///
/// // Terminated due to error
/// let reason = TaskEventStopReason::Error("Process crashed".to_string());
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskEventStopReason {
    /// Process completed normally with an exit code
    Finished,

    /// Process was terminated for a specific reason
    Terminated(TaskTerminateReason),

    /// Process stopped due to an error
    Error(String),
}

/// Reason for terminating a running task
///
/// Provides context about why a task termination was requested,
/// enabling appropriate cleanup and response handling.
///
/// # Examples
///
/// ## Timeout Termination
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     async_tokio::spawner::TaskSpawner,
///     event::TaskTerminateReason
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd").args(["/C", "ping", "127.0.0.1", "-n", "5"]); // 5 second sleep
///     let mut spawner = TaskSpawner::new("long-task".to_string(), config);
///     
///     let (tx, _rx) = mpsc::channel(100);
///     spawner.start_direct(tx).await?;
///     
///     // Terminate after 1 second
///     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
///     spawner.send_terminate_signal(TaskTerminateReason::Timeout).await?;
///     
///     Ok(())
/// }
/// ```
///
/// ## Cleanup Termination
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     async_tokio::spawner::TaskSpawner,
///     event::TaskTerminateReason
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "running"]);
///     let mut spawner = TaskSpawner::new("daemon".to_string(), config);
///     
///     let (tx, _rx) = mpsc::channel(100);
///     spawner.start_direct(tx).await?;
///     
///     // Cleanup shutdown reason
///     let reason = TaskTerminateReason::Cleanup;
///     spawner.send_terminate_signal(reason).await?;
///     
///     Ok(())
/// }
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskTerminateReason {
    /// Task exceeded its configured timeout
    ///
    /// The process ran longer than the `timeout_ms` specified in `TaskConfig`
    /// and was terminated to prevent runaway processes.
    Timeout,

    /// Task was terminated during cleanup operations
    ///
    /// Used when terminating tasks as part of application shutdown,
    /// resource cleanup, or dependency management.
    Cleanup,

    /// Task was terminated because its dependencies finished
    ///
    /// Used in task orchestration scenarios where tasks depend on
    /// other tasks and should be terminated when dependencies complete.
    DependenciesFinished,
}

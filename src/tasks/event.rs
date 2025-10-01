use std::time::SystemTime;

use crate::tasks::{config::StreamSource, error::TaskError};

/// Events emitted during task execution lifecycle
///
/// `TaskEvent` represents all events that occur during task execution,
/// from process start to completion. These events enable real-time monitoring
/// and event-driven programming patterns.
///
/// # Event Flow
///
/// A typical task execution emits events in this order:
/// 1. `Started` - Process has been spawned
/// 2. `Output` - Output lines from stdout/stderr (ongoing)
/// 3. `Ready` - Ready indicator detected (optional, for long-running processes)
/// 4. `Stopped` - Process has completed, with exit code and reason
///    - Exit code is `Some(code)` for natural completion
///    - Exit code is `None` for terminated processes (timeout, manual termination)
/// 5. `Error` - Error related to task execution
///
/// # Examples
///
/// ## Basic Event Processing
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, tokio::executor::TaskExecutor, event::TaskEvent};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello", "world"]);
///     #[cfg(unix)]
///     let config = TaskConfig::new("echo").args(["hello", "world"]);
///     
///     let mut executor = TaskExecutor::new(config);
///     
///     let (tx, mut rx) = mpsc::channel(100);
///     executor.coordinate_start(tx).await?;
///
///     while let Some(event) = rx.recv().await {
///         match event {
///             TaskEvent::Started { process_id, .. } => {
///                 println!("Process started with ID: {}", process_id);
///             }
///             TaskEvent::Output { line, .. } => {
///                 println!("Output: {}", line);
///             }
///             TaskEvent::Stopped { exit_code, .. } => {
///                 match exit_code {
///                     Some(code) => println!("Process completed with code {}", code),
///                     None => println!("Process was terminated"),
///                 }
///                 break;
///             }
///             TaskEvent::Error { error } => {
///                 eprintln!("Error: {}", error);
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
///     tokio::executor::TaskExecutor,
///     event::TaskEvent
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd")
///         .args(["/C", "echo", "Server listening"])
///         .ready_indicator("Server listening")
///         .ready_indicator_source(StreamSource::Stdout);
///     
///     #[cfg(unix)]
///     let config = TaskConfig::new("echo")
///         .args(["Server listening"])
///         .ready_indicator("Server listening")
///         .ready_indicator_source(StreamSource::Stdout);
///
///     let mut executor = TaskExecutor::new(config);
///     let (tx, mut rx) = mpsc::channel(100);
///     executor.coordinate_start(tx).await?;
///
///     while let Some(event) = rx.recv().await {
///         match event {
///             TaskEvent::Ready => {
///                 println!("Server is ready for requests!");
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
    /// This is the first event emitted after successful process spawning.
    /// The process is now running and other events will follow.
    Started {
        /// Operating system process ID
        process_id: u32,
        /// Timestamp when the process was created
        created_at: SystemTime,
        /// Timestamp when the process started running
        running_at: SystemTime,
    },

    /// Output line received from the process
    ///
    /// Emitted for each line of output from stdout or stderr.
    /// Lines are buffered and emitted when complete (on newline).
    Output {
        /// The output line (without trailing newline)
        line: String,
        /// Source stream (stdout or stderr)
        src: StreamSource,
    },

    /// Process has signaled it's ready to work
    ///
    /// Only emitted for long-running processes that have a ready indicator configured.
    /// Indicates the process has completed initialization and is ready for work.
    Ready,

    /// Process has completed execution
    ///
    /// The process has exited and all resources have been cleaned up.
    Stopped {
        /// Exit code from the process
        ///
        /// - `Some(code)` - Process completed naturally with exit code
        /// - `None` - Process was terminated (timeout, user request, etc.)
        ///
        /// Note: Terminated processes do not provide exit codes to avoid
        /// race conditions between termination and natural completion.
        exit_code: Option<i32>,
        /// Reason the process stopped
        reason: TaskStopReason,
        /// Timestamp when the process finished
        finished_at: SystemTime,

        #[cfg(unix)]
        /// Termination signal if the process was killed by a signal
        signal: Option<i32>,
    },

    /// An error occurred during task execution
    ///
    /// Emitted when errors occur during configuration validation,
    /// process spawning, sending input/output
    ///  
    Error {
        /// The specific error that occurred
        error: TaskError,
    },
}

/// Reason why a task stopped executing
///
/// Provides detailed information about why a process completed,
/// whether due to natural completion, termination, or error.
///
/// # Exit Code Relationship
///
/// - `Finished`: Process completed naturally - exit code is `Some(code)`
/// - `Terminated(_)`: Process was killed - exit code is `None`
/// - `Error(_)`: Process encountered an error - exit code behavior varies
///
/// # Examples
///
/// ```rust
/// use tcrm_task::tasks::{event::TaskStopReason, event::TaskTerminateReason};
///
/// // Natural completion
/// let reason = TaskStopReason::Finished;
///
/// // Terminated due to timeout
/// let reason = TaskStopReason::Terminated(TaskTerminateReason::Timeout);
///
/// // Terminated due to error
/// let reason = TaskStopReason::Error(tcrm_task::tasks::error::TaskError::IO("Process crashed".to_string()));
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStopReason {
    /// Process completed normally with an exit code
    ///
    /// The process ran to completion and exited naturally.
    /// Exit code will be `Some(code)` in the `TaskEvent::Stopped` event.
    Finished,

    /// Process was terminated for a specific reason
    ///
    /// The process was forcefully killed before natural completion.
    /// Exit code will be `None` in the `TaskEvent::Stopped` event.
    Terminated(TaskTerminateReason),

    /// Process stopped due to an error
    ///
    /// An error occurred during execution or process management.
    /// Exit code behavior varies depending on the type of error.
    Error(TaskError),
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
///     tokio::executor::TaskExecutor,
///     event::TaskTerminateReason
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd").args(["/C", "timeout", "/t", "5"]); // 5 second sleep
///     #[cfg(unix)]
///     let config = TaskConfig::new("sleep").args(["5"]); // 5 second sleep
///     
///     let mut executor = TaskExecutor::new(config);
///     
///     let (tx, _rx) = mpsc::channel(100);
///     executor.coordinate_start(tx).await?;
///     
///     // Terminate after 1 second
///     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
///     
///     Ok(())
/// }
/// ```
///
/// ## Cleanup Termination
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     tokio::executor::TaskExecutor,
///     event::TaskTerminateReason
/// };
/// use tokio::sync::mpsc;
/// use crate::tcrm_task::tasks::control::TaskControl;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "running"]);
///     #[cfg(unix)]
///     let config = TaskConfig::new("echo").args(["running"]);
///     
///     let mut executor = TaskExecutor::new(config);
///     
///     let (tx, _rx) = mpsc::channel(100);
///     executor.coordinate_start(tx).await?;
///     
///     let reason = TaskTerminateReason::UserRequested;
///     executor.terminate_task(reason)?;
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

    /// Task was terminated by explicit user request
    ///
    /// Used when user or external library requests the task to stop.
    UserRequested,

    /// Task was terminated due to internal error condition
    ///
    /// Indicates that the task encountered an unexpected error
    /// that caused it to be terminated.
    InternalError,
}

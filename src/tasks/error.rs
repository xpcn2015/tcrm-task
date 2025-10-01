use thiserror::Error;

/// Errors that can occur during task configuration and execution
///
/// `TaskError` represents all error conditions that can arise when configuring,
/// validating, or executing tasks. Each variant provides specific context
/// about the failure to enable proper error handling and debugging.
///
/// # Examples
///
/// ## Error Handling
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, error::TaskError};
///
/// fn validate_config(config: &TaskConfig) -> Result<(), String> {
///     match config.validate() {
///         Ok(()) => Ok(()),
///         Err(TaskError::InvalidConfiguration(msg)) => {
///             Err(format!("Configuration error: {}", msg))
///         }
///         Err(TaskError::IO(msg)) => {
///             Err(format!("IO error: {}", msg))
///         }
///         Err(other) => {
///             Err(format!("Other error: {}", other))
///         }
///     }
/// }
/// ```
///
/// ## Pattern Matching on Events
/// ```rust
/// use tcrm_task::tasks::{event::TaskEvent, error::TaskError};
///
/// fn handle_event(event: TaskEvent) {
///     match event {
///         TaskEvent::Error { error } => {
///             match error {
///                 TaskError::IO(msg) => {
///                     eprintln!("IO error: {}", msg);
///                 }
///                 TaskError::InvalidConfiguration(msg) => {
///                     eprintln!("Config error: {}", msg);
///                 }
///                 TaskError::Channel(msg) => {
///                     eprintln!("Channel error: {}", msg);
///                 }
///                 _ => {
///                     eprintln!("Other error: {}", error);
///                 }
///             }
///         }
///         _ => {}
///     }
/// }
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TaskError {
    /// Input/Output operation failed
    ///
    /// Covers file system operations, process spawning failures,
    /// and other system-level IO errors.
    ///
    /// # Common Causes
    /// - Command not found in PATH
    /// - Permission denied when spawning process
    /// - Working directory doesn't exist or isn't accessible
    /// - File descriptor or pipe creation failures
    #[error("IO error: {0}")]
    IO(String),

    /// Process handle or watcher error
    ///
    /// Errors related to process handle management or watcher failures.
    ///
    /// # Common Causes
    /// - Failed to obtain process ID after spawning
    /// - Watcher task join or abort errors
    #[error("Handle error: {0}")]
    Handle(String),

    /// Inter-task communication channel error
    ///
    /// Failures in the async channel system used for event delivery,
    /// stdin input, or process coordination.
    ///
    /// # Common Causes
    /// - Event channel closed unexpectedly
    /// - Stdin channel disconnected
    /// - Termination signal channel closed
    /// - Receiver dropped before sender finished
    #[error("Channel error: {0}")]
    Channel(String),

    /// Task configuration validation failed
    ///
    /// The task configuration contains invalid parameters that
    /// prevent safe execution. Always check these before starting tasks.
    ///
    /// # Common Causes
    /// - Empty command string
    /// - Invalid characters in command or arguments
    /// - Working directory doesn't exist
    /// - Environment variables with invalid keys
    /// - Zero or negative timeout values
    /// - Security validation failures (command injection, etc.)
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Control operation failed
    ///
    /// Errors related to controlling or managing the task's execution.
    /// This can include failures to pause, resume, or stop the task.
    ///
    /// # Common Causes
    /// - Failed to send control signal to process
    #[error("Control error: {0}")]
    Control(String),
}

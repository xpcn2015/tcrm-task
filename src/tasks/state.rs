/// Execution state of a task throughout its lifecycle
///
/// `TaskState` tracks the progression of a task from creation through completion.
/// States transition in a defined order, enabling predictable state management
/// and event-driven programming.
///
/// # State Transitions
///
/// ```text
/// Pending → Initiating → Running → [Ready] → Finished
///                   ↘             ↗
///                    → Finished ←
/// ```
///
/// The Ready state is optional and only occurs for long-running processes
/// with a configured ready indicator.
///
/// # Examples
///
/// ## State Monitoring
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner, state::TaskState};
///
/// #[tokio::main]
/// async fn main() {
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello"]);
///     let spawner = TaskSpawner::new("test".to_string(), config);
///     
///     // Initially pending
///     assert_eq!(spawner.get_state().await, TaskState::Pending);
///     
///     // After calling start_direct(), state will progress through:
///     // Pending → Initiating → Running → Finished
/// }
/// ```
///
/// ## Basic State Checking
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     async_tokio::spawner::TaskSpawner,
///     state::TaskState
/// };
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello"]);
///     let spawner = TaskSpawner::new("demo".to_string(), config);
///
///     // Check initial state
///     let state = spawner.get_state().await;
///     assert_eq!(state, TaskState::Pending);
///     println!("Task is in {:?} state", state);
///
///     Ok(())
/// }
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    /// Task has been created but not yet started
    ///
    /// Initial state when `TaskSpawner` is created. The task configuration
    /// exists but no process has been spawned yet.
    Pending,

    /// Task is being initialized and validated
    ///
    /// Transitional state during `start_direct()` when configuration is being
    /// validated and the process is being prepared for spawning.
    Initiating,

    /// Process is running and executing
    ///
    /// The system process has been spawned and is actively executing.
    /// Output events may be emitted during this state.
    Running,

    /// Process is running and ready to accept requests
    ///
    /// Only reached by long-running processes that have a ready indicator
    /// configured. Indicates the process has completed initialization
    /// and is ready for work (e.g., web server listening on port).
    Ready,

    /// Task execution has completed
    ///
    /// Final state reached when the process exits normally, is terminated,
    /// or encounters an error. No further state transitions occur.
    Finished,
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
///     state::TaskTerminateReason
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
/// ## Custom Termination
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     async_tokio::spawner::TaskSpawner,
///     state::TaskTerminateReason
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
///     // Custom shutdown reason
///     let reason = TaskTerminateReason::Custom("User requested shutdown".to_string());
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

    /// Task was terminated for a custom application-specific reason
    ///
    /// Allows applications to provide specific context about why
    /// a task was terminated beyond the standard reasons.
    Custom(String),
}

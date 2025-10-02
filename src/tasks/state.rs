/// Execution state of a task throughout its lifecycle
///
/// `TaskState` tracks the progression of a task from creation through completion.
/// States transition in a defined order, enabling event-driven tasks execution.
///
/// # State Transitions
///
/// ```text
/// Pending → Initiating → Running → [Ready] → Finished
///                           ↘
///                               →  Finished
/// ```
///
/// The Ready state is optional and only occurs for long-running processes
/// with a configured ready indicator.
///
/// # Examples
///
/// ## State Monitoring
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, tokio::executor::TaskExecutor, state::TaskState, control::TaskStatusInfo};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello"]);
///     #[cfg(unix)]
///     let config = TaskConfig::new("echo").args(["hello"]);
///     
///     let (tx, _rx) = mpsc::channel(100);
///     let executor = TaskExecutor::new(config, tx);
///     
///     // Initially pending
///     assert_eq!(executor.get_state(), TaskState::Pending);
///     
///     // After calling coordinate_start(), state will progress through:
///     // Pending → Initiating → Running → Finished
/// }
/// ```
///
/// ## Basic State Checking
/// ```rust
/// use tcrm_task::tasks::{
///     config::TaskConfig,
///     tokio::executor::TaskExecutor,
///     state::TaskState,
///     control::TaskStatusInfo
/// };
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     #[cfg(windows)]
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello"]);
///     #[cfg(unix)]
///     let config = TaskConfig::new("echo").args(["hello"]);
///     
///     let (tx, _rx) = mpsc::channel(100);
///     let executor = TaskExecutor::new(config, tx);
///
///     // Check initial state
///     let state = executor.get_state();
///     assert_eq!(state, TaskState::Pending);
///     println!("Task is in {:?} state", state);
///
///     Ok(())
/// }
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task has been created but not yet started
    ///
    /// Initial state when `TaskSpawner` is created. The task configuration
    /// exists but no process has been spawned yet.
    Pending = 0,

    /// Task is being initialized and validated
    ///
    /// Transitional state during `start_direct()` when configuration is being
    /// validated and the process is being prepared for spawning.
    Initiating = 1,

    /// Process is running and executing
    ///
    /// The system process has been spawned and is actively executing.
    /// Output events may be emitted during this state.
    Running = 2,

    /// Process is running and executing
    ///
    /// Only reached by long-running processes that have a ready indicator
    /// configured. Indicates the process has completed initialization
    /// and is ready for work (e.g., web server listening on port).
    /// Useful when orchestrating dependent tasks.
    Ready = 3,

    /// Task execution has completed
    ///
    /// Final state reached when the process exits normally, is terminated,
    /// or encounters an error. No further state transitions occur.
    Finished = 4,

    /// Invalid state (should not occur)
    ///
    /// This state indicates an error in state management. It should not be
    /// possible to reach this state during normal operation.
    ///
    /// Internal use only.
    Invalid = 255,
}

impl From<u8> for TaskState {
    /// Converts a `u8` value to a `TaskState` enum.
    ///
    /// Returns `TaskState::Invalid` for unknown values.
    fn from(value: u8) -> Self {
        match value {
            0 => TaskState::Pending,
            1 => TaskState::Initiating,
            2 => TaskState::Running,
            3 => TaskState::Ready,
            4 => TaskState::Finished,
            _ => TaskState::Invalid,
        }
    }
}

impl From<TaskState> for u8 {
    /// Converts a `TaskState` enum to its corresponding `u8` value.
    fn from(state: TaskState) -> Self {
        state as u8
    }
}

/// Represents the state of a spawned process during its lifecycle.
///
/// `ProcessState` is used to track whether a process is running, paused, or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// The process is not running.
    Stopped = 0,

    /// The process is running.
    Running = 1,

    /// The process is paused.
    Pause = 2,

    /// Invalid state (should not occur).
    Invalid = 255,
}

impl From<u8> for ProcessState {
    /// Converts a `u8` value to a `ProcessState` enum.
    ///
    /// Returns `ProcessState::Stopped` for unknown values.
    fn from(value: u8) -> Self {
        match value {
            0 => ProcessState::Stopped,
            1 => ProcessState::Running,
            2 => ProcessState::Pause,
            _ => ProcessState::Invalid,
        }
    }
}

impl From<ProcessState> for u8 {
    /// Converts a `ProcessState` enum to its corresponding `u8` value.
    fn from(state: ProcessState) -> Self {
        state as u8
    }
}

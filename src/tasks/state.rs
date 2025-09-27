/// Execution state of a task throughout its lifecycle
///
/// `TaskState` tracks the progression of a task from creation through completion.
/// States transition in a defined order, enabling event-driven tasks execution.
///
/// # State Transitions
///
/// ```text
/// Pending → Initiating → Running → [Ready] → Finished
///                             ↘
///                               → Finished
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

    Invalid = 255, // Internal use only
}
impl From<u8> for TaskState {
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
    fn from(state: TaskState) -> Self {
        state as u8
    }
}

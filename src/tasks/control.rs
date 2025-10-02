use std::time::SystemTime;

use crate::tasks::{
    error::TaskError,
    event::TaskTerminateReason,
    state::{ProcessState, TaskState},
};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;

/// Trait for controlling process execution.
///
/// This trait provides methods to control the lifecycle of a running process,
/// including termination and signal handling.
pub trait TaskControl {
    /// Terminates the task with the specified reason.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for termination
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the termination signal was sent successfully
    /// * `Err(TaskError)` - If the task is already finished or termination fails
    ///
    /// # Example
    ///
    /// ```rust
    /// use tcrm_task::tasks::control::TaskControl;
    /// use tcrm_task::tasks::event::TaskTerminateReason;
    /// use tcrm_task::tasks::config::TaskConfig;
    /// use tcrm_task::tasks::tokio::executor::TaskExecutor;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = TaskConfig::new("echo".to_string());
    /// let mut task = TaskExecutor::new(config);
    /// task.terminate_task(TaskTerminateReason::UserRequested)?;
    /// # Ok(())
    /// # }
    /// ```
    fn terminate_task(&mut self, reason: TaskTerminateReason) -> Result<(), TaskError>;

    /// Sends a signal to the task process.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to send to the process
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the signal was sent successfully
    /// * `Err(TaskError)` - If signal sending fails
    #[cfg(feature = "signal")]
    fn send_signal(&self, signal: ProcessSignal) -> Result<(), TaskError>;
}
/// Trait for retrieving task status information.
///
/// This trait provides methods to query the current state and runtime
/// information of a task.
pub trait TaskStatusInfo {
    /// Gets the current state of the task.
    ///
    /// # Returns
    ///
    /// The current `TaskState` of the task
    fn get_task_state(&self) -> TaskState;

    /// Gets the current state of the process.
    ///
    /// # Returns
    ///
    /// The current `ProcessState` of the process
    #[cfg(feature = "process-control")]
    fn get_process_state(&self) -> ProcessState;

    /// Gets the process ID of the running task.
    ///
    /// # Returns
    ///
    /// * `Some(u32)` - The process ID if the task is running
    /// * `None` - If process hasn't started yet or has finished
    fn get_process_id(&self) -> Option<u32>;

    /// Gets the creation timestamp of the task.
    ///
    /// # Returns
    ///
    /// The `SystemTime` when the task was created
    fn get_create_at(&self) -> SystemTime;

    /// Gets the timestamp when the task started running.
    ///
    /// # Returns
    ///
    /// * `Some(SystemTime)` - When the task started running
    /// * `None` - If the task hasn't started yet
    fn get_running_at(&self) -> Option<SystemTime>;

    /// Gets the timestamp when the task finished.
    ///
    /// # Returns
    ///
    /// * `Some(SystemTime)` - When the task finished
    /// * `None` - If the task is still running or hasn't started
    fn get_finished_at(&self) -> Option<SystemTime>;

    /// Gets the exit code of the finished task.
    ///
    /// # Returns
    ///
    /// * `Some(i32)` - The exit code if the task has finished
    /// * `None` - If the task is still running or hasn't started
    fn get_exit_code(&self) -> Option<i32>;

    /// Gets all information about the task.
    ///
    /// This is a convenience method that collects all status information
    /// into a single structure.
    ///
    /// # Returns
    ///
    /// A `TaskInformation` struct containing all task status data
    ///
    /// # Example
    ///
    /// ```rust
    /// use tcrm_task::tasks::control::TaskStatusInfo;
    /// use tcrm_task::tasks::config::TaskConfig;
    /// use tcrm_task::tasks::tokio::executor::TaskExecutor;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = TaskConfig::new("echo".to_string());
    /// let task = TaskExecutor::new(config);
    /// let info = task.get_information();
    /// println!("Task state: {:?}", info.state);
    /// if let Some(pid) = info.process_id {
    ///     println!("Process ID: {}", pid);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_information(&self) -> TaskInformation {
        TaskInformation {
            task_state: self.get_task_state(),
            #[cfg(feature = "process-control")]
            process_state: self.get_process_state(),
            process_id: self.get_process_id(),
            created_at: self.get_create_at(),
            running_at: self.get_running_at(),
            finished_at: self.get_finished_at(),
            exit_code: self.get_exit_code(),
        }
    }
}

/// Task status information.
///
/// This structure contains all available information about a task's
/// current state.
#[derive(Debug, PartialEq)]
pub struct TaskInformation {
    /// Current state of the task
    pub task_state: TaskState,
    /// Current state of the process
    #[cfg(feature = "process-control")]
    pub process_state: ProcessState,
    /// Process ID if the task is running
    pub process_id: Option<u32>,
    /// When the task was created
    pub created_at: SystemTime,
    /// When the task started running (if it has started)
    pub running_at: Option<SystemTime>,
    /// When the task finished (if it has finished)
    pub finished_at: Option<SystemTime>,
    /// Exit code of the task (if it has finished)
    pub exit_code: Option<i32>,

    #[cfg(unix)]
    /// Last received signal (if any) from process::ExitStatus
    pub last_signal: Option<i32>,
}

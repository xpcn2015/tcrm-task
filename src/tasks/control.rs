use std::time::SystemTime;

use crate::tasks::{
    config::TaskConfig,
    error::TaskError,
    event::{TaskStopReason, TaskTerminateReason},
    state::TaskState,
};

#[cfg(feature = "signal")]
use crate::tasks::signal::ProcessSignal;

pub trait TaskControl {
    fn terminate_task(&mut self, reason: TaskTerminateReason) -> Result<(), TaskError>;

    /// Perform a control action on the child process.
    fn perform_process_action(&mut self, action: TaskControlAction) -> Result<(), TaskError>;

    #[cfg(feature = "signal")]
    fn send_signal(&self, signal: ProcessSignal) -> Result<(), TaskError>;
}
pub trait TaskStatusInfo {
    fn get_config(&self) -> &TaskConfig;
    fn get_state(&self) -> &TaskState;
    fn get_process_id(&self) -> &Option<u32>;
    fn get_create_at(&self) -> &SystemTime;
    fn get_running_at(&self) -> &Option<SystemTime>;
    fn get_finished_at(&self) -> &Option<SystemTime>;
    fn get_exit_code(&self) -> &Option<i32>;
    fn get_stop_reason(&self) -> &Option<TaskStopReason>;
    fn get_information(&self) -> TaskInformation {
        TaskInformation {
            config: self.get_config().clone(),
            state: *self.get_state(),
            process_id: *self.get_process_id(),
            created_at: *self.get_create_at(),
            running_at: *self.get_running_at(),
            finished_at: *self.get_finished_at(),
            exit_code: *self.get_exit_code(),
            stop_reason: self.get_stop_reason().clone(),
        }
    }
}
pub(crate) trait TaskInternal {
    fn set_state(&mut self, new_state: TaskState) -> SystemTime;
}
#[derive(Debug, PartialEq)]
pub struct TaskInformation {
    pub config: TaskConfig,
    pub state: TaskState,
    pub process_id: Option<u32>,
    pub created_at: SystemTime,
    pub running_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub stop_reason: Option<TaskStopReason>,
}
pub enum TaskControlAction {
    Terminate,
    Pause,
    Resume,
    Interrupt,
}

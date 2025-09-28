use std::time::Instant;

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
pub trait TaskInformation {
    fn get_config(&self) -> &TaskConfig;
    fn get_state(&self) -> &TaskState;
    fn get_process_id(&self) -> &Option<u32>;
    fn get_create_at(&self) -> &Instant;
    fn get_running_at(&self) -> &Option<Instant>;
    fn get_finished_at(&self) -> &Option<Instant>;
    fn get_exit_code(&self) -> &Option<i32>;
    fn get_stop_reason(&self) -> &Option<TaskStopReason>;
}

pub(crate) trait TaskInternal {
    fn set_state(&mut self, new_state: TaskState);
}

pub enum TaskControlAction {
    Terminate,
    Pause,
    Resume,
    Interrupt,
}

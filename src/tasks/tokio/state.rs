use std::{sync::Arc, time::SystemTime};

use crate::tasks::{
    state::TaskState,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
    /// Updates the task state and records appropriate timestamps.
    ///
    /// Sets the new state in the shared context and updates the corresponding
    /// timestamp (running_at for Running state, finished_at for Finished state).
    ///
    /// # Arguments
    ///
    /// * `shared_context` - The shared task execution context
    /// * `new_state` - The new state to set
    ///
    /// # Returns
    ///
    /// The timestamp when the state change occurred
    pub(crate) fn update_state(
        shared_context: &Arc<TaskExecutorContext>,
        new_state: TaskState,
    ) -> SystemTime {
        shared_context.set_state(new_state);

        let time = match new_state {
            TaskState::Running => shared_context.set_running_at(),
            TaskState::Finished => shared_context.set_finished_at(),
            _ => SystemTime::now(),
        };
        time
    }
}

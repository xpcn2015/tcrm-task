use std::{sync::Arc, time::SystemTime};

use crate::tasks::{
    state::TaskState,
    tokio::{context::TaskExecutorContext, executor::TaskExecutor},
};

impl TaskExecutor {
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

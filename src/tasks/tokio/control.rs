use std::time::SystemTime;

use crate::tasks::{
    control::{TaskControl, TaskStatusInfo},
    error::TaskError,
    event::TaskTerminateReason,
    state::TaskState,
    tokio::executor::TaskExecutor,
};

impl TaskControl for TaskExecutor {
    fn terminate_task(&mut self, reason: TaskTerminateReason) -> Result<(), TaskError> {
        let current_state = self.get_state();
        if current_state == TaskState::Finished {
            return Err(TaskError::Control("Task already finished".to_string()));
        }
        if let Some(tx) = self.terminate_tx.take() {
            if tx.send(reason.clone()).is_err() {
                let msg = "Terminate channel closed while sending signal";
                #[cfg(feature = "tracing")]
                tracing::warn!(terminate_reason=?reason, msg);
                return Err(TaskError::Channel(msg.to_string()));
            }
        } else {
            let msg = "Terminate signal already sent or channel missing";
            #[cfg(feature = "tracing")]
            tracing::warn!(msg);
            return Err(TaskError::Channel(msg.to_string()));
        }

        Ok(())
    }

    #[cfg(feature = "signal")]
    fn send_signal(&self, signal: ProcessSignal) -> Result<(), TaskError> {
        todo!()
    }
}

impl TaskStatusInfo for TaskExecutor {
    fn get_state(&self) -> TaskState {
        self.shared_context.get_state()
    }

    fn get_process_id(&self) -> Option<u32> {
        self.shared_context.get_process_id()
    }

    fn get_create_at(&self) -> SystemTime {
        self.shared_context.get_create_at()
    }

    fn get_running_at(&self) -> Option<SystemTime> {
        self.shared_context.get_running_at()
    }

    fn get_finished_at(&self) -> Option<SystemTime> {
        self.shared_context.get_finished_at()
    }
    fn get_exit_code(&self) -> Option<i32> {
        self.shared_context.get_exit_code()
    }
}

#[cfg(feature = "process-control")]
use crate::tasks::error::TaskError;

#[cfg(feature = "process-control")]
pub trait ProcessControl {
    /// Requests the process to stop execution.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the stop signal was sent successfully
    /// * `Err(TaskError)` if stopping fails
    #[cfg(feature = "tokio")]
    fn stop_process(&mut self) -> impl futures::Future<Output = Result<(), TaskError>> {
        self.perform_process_action(ProcessControlAction::Stop)
    }

    /// Requests the process to pause execution.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the pause signal was sent successfully
    /// * `Err(TaskError)` if pausing fails
    fn pause_process(&mut self) -> impl futures::Future<Output = Result<(), TaskError>> {
        self.perform_process_action(ProcessControlAction::Pause)
    }

    /// Requests the process to resume execution if it is paused.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the resume signal was sent successfully
    /// * `Err(TaskError)` if resuming fails
    fn resume_process(&mut self) -> impl futures::Future<Output = Result<(), TaskError>> {
        self.perform_process_action(ProcessControlAction::Resume)
    }

    /// Performs a control action on the process or process group.
    ///
    /// # Arguments
    ///
    /// * `action` - The control action to perform (stop, pause, resume)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the action was performed successfully
    /// * `Err(TaskError)` if the action fails
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Control`] if the process is not in a controllable state
    /// or if the requested action is not supported
    fn perform_process_action(
        &mut self,
        action: ProcessControlAction,
    ) -> impl futures::Future<Output = Result<(), TaskError>>;
}

/// Actions that can be performed on a process.
///
/// Defines the possible control actions that can be
/// applied to a running process, such as stopping, pausing, or resuming.
#[cfg(feature = "process-control")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessControlAction {
    /// Stop process execution.
    Stop,
    /// Pause process execution.
    Pause,
    /// Resume paused process execution.
    Resume,
}

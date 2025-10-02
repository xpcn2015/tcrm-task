use crate::tasks::process::group::builder::ProcessGroup;
use crate::tasks::process::group::error::ProcessGroupError;

impl ProcessGroup {
    /// Terminates all processes in the group (Unix).
    ///
    /// Sends SIGTERM to the process group using killpg().
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the signal was sent successfully or processes were already terminated
    /// * `Err(ProcessGroupError)` - If termination fails due to permissions or other errors
    ///
    /// # Example
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::group::builder::ProcessGroup;
    /// let mut group = ProcessGroup::new();
    /// // ... spawn processes in the group ...
    /// group.terminate_group().unwrap();
    /// ```
    #[cfg(unix)]
    pub fn terminate_group(&self) -> Result<(), ProcessGroupError> {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        if let Some(pgid) = self.inner.process_group_id {
            use nix::errno::Errno;
            match killpg(Pid::from_raw(pgid), Signal::SIGTERM) {
                Ok(_) => Ok(()),
                Err(e) => match e {
                    Errno::ESRCH => Ok(()), // Already terminated
                    Errno::EPERM => Err(ProcessGroupError::SignalFailed(format!(
                        "Permission denied to terminate process group {}",
                        pgid
                    ))),
                    _ => Err(ProcessGroupError::SignalFailed(format!(
                        "Failed to send SIGTERM to process group {}: {}",
                        pgid, e
                    ))),
                },
            }
        } else {
            Err(ProcessGroupError::SignalFailed(
                "No process group ID available".to_string(),
            ))
        }
    }

    /// Terminates all processes in the job object (Windows).
    ///
    /// Terminates all processes in the job object using TerminateJobObject.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the job was terminated successfully or was already terminated
    /// * `Err(ProcessGroupError)` - If termination fails due to permissions or other errors
    ///
    /// # Example
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::group::builder::ProcessGroup;
    /// let mut group = ProcessGroup::new();
    /// // ... spawn processes in the group ...
    /// group.terminate_group().unwrap();
    /// ```
    #[cfg(windows)]
    pub fn stop_group(&self) -> Result<(), ProcessGroupError> {
        use crate::tasks::process::group::builder::SendHandle;
        if let Some(SendHandle(job_handle)) = &self.inner.job_handle {
            unsafe {
                use windows::Win32::System::JobObjects::TerminateJobObject;
                // Terminate all processes in the job object
                // Note: Do NOT call CloseHandle here - the Drop implementation will handle it
                // Calling CloseHandle here would cause a double-free when Drop is called
                TerminateJobObject(*job_handle, 1).map_err(|e| {
                    ProcessGroupError::SignalFailed(format!(
                        "Failed to terminate job object: {}",
                        e
                    ))
                })?;
            }
            Ok(())
        } else {
            Err(ProcessGroupError::SignalFailed(
                "No Job Object handle available".to_string(),
            ))
        }
    }

    /// Process group termination is not available on this platform.
    #[cfg(not(any(unix, windows)))]
    pub fn terminate_group(&self) -> Result<(), ProcessGroupError> {
        Err(ProcessGroupError::UnsupportedPlatform(
            "Process group termination not available on this platform".to_string(),
        ))
    }
}

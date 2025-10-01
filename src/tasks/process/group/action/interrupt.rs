use crate::tasks::process::group::builder::ProcessGroup;
use crate::tasks::process::group::error::ProcessGroupError;

impl ProcessGroup {
    /// Interrupts all processes in the group (Unix).
    ///
    /// Sends SIGINT to the process group using killpg().
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the signal was sent successfully
    /// * `Err(ProcessGroupError)` - If interrupting fails due to permissions or other errors
    ///
    /// # Example
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::group::builder::ProcessGroup;
    /// let mut group = ProcessGroup::new();
    /// // ... spawn processes in the group ...
    /// group.interrupt_group().unwrap();
    /// ```
    #[cfg(unix)]
    pub fn interrupt_group(&self) -> Result<(), ProcessGroupError> {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        if let Some(pgid) = self.inner.process_group_id {
            use nix::errno::Errno;
            match killpg(Pid::from_raw(pgid), Signal::SIGINT) {
                Ok(_) => Ok(()),
                Err(e) => match e {
                    Errno::ESRCH => Ok(()), // Already terminated
                    Errno::EPERM => Err(ProcessGroupError::SignalFailed(format!(
                        "Permission denied to interrupt process group {}",
                        pgid
                    ))),
                    _ => Err(ProcessGroupError::SignalFailed(format!(
                        "Failed to send SIGINT to process group {}: {}",
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

    /// Interrupts all processes in the job object (Windows).
    ///
    /// Sends interrupt signals to all processes in the job object.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the interrupt signals were sent successfully
    /// * `Err(ProcessGroupError)` - If interrupting fails due to permissions or other errors
    ///
    /// # Example
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::group::builder::ProcessGroup;
    /// let mut group = ProcessGroup::new();
    /// // ... spawn processes in the group ...
    /// group.interrupt_group().unwrap();
    /// ```
    #[cfg(windows)]
    pub fn interrupt_group(&self) -> Result<(), ProcessGroupError> {
        use crate::tasks::process::group::builder::SendHandle;
        use windows::Win32::System::Console::{CTRL_C_EVENT, GenerateConsoleCtrlEvent};
        use windows::Win32::System::JobObjects::{
            JOBOBJECT_BASIC_PROCESS_ID_LIST, JobObjectBasicProcessIdList, QueryInformationJobObject,
        };

        if let Some(SendHandle(job_handle)) = &self.inner.job_handle {
            unsafe {
                // First, get the list of processes in the job
                let mut process_list = JOBOBJECT_BASIC_PROCESS_ID_LIST::default();
                let mut returned_length = 0u32;

                QueryInformationJobObject(
                    Some(*job_handle),
                    JobObjectBasicProcessIdList,
                    &mut process_list as *mut _ as *mut std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>() as u32,
                    Some(&mut returned_length),
                )
                .map_err(|e| {
                    ProcessGroupError::SignalFailed(format!(
                        "Failed to query job object process list: {}",
                        e
                    ))
                })?;

                let mut interrupted_count = 0;

                // Send CTRL+C to each process in the job
                for i in 0..process_list.NumberOfProcessIdsInList {
                    let pid = process_list.ProcessIdList[i as usize] as u32;

                    // Send CTRL+C to this process
                    if GenerateConsoleCtrlEvent(CTRL_C_EVENT, pid).is_ok() {
                        interrupted_count += 1;
                    }
                }

                if interrupted_count > 0 {
                    Ok(())
                } else {
                    Err(ProcessGroupError::SignalFailed(
                        "No processes were interrupted in the job object".to_string(),
                    ))
                }
            }
        } else {
            Err(ProcessGroupError::SignalFailed(
                "No Job Object handle available".to_string(),
            ))
        }
    }

    /// Process group interrupting is not available on this platform.
    #[cfg(not(any(unix, windows)))]
    pub fn interrupt_group(&self) -> Result<(), ProcessGroupError> {
        Err(ProcessGroupError::UnsupportedPlatform(
            "Process group interrupting not available on this platform".to_string(),
        ))
    }
}

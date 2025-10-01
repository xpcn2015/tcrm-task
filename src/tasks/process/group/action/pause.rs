use crate::tasks::process::group::builder::ProcessGroup;
use crate::tasks::process::group::error::ProcessGroupError;

impl ProcessGroup {
    /// Pauses all processes in the group (Unix).
    ///
    /// Sends SIGSTOP to the process group using killpg().
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the signal was sent successfully or processes were already stopped
    /// * `Err(ProcessGroupError)` - If pausing fails due to permissions or other errors
    ///
    /// # Example
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::group::builder::ProcessGroup;
    /// let mut group = ProcessGroup::new();
    /// // ... spawn processes in the group ...
    /// group.pause_group().unwrap();
    /// ```
    #[cfg(unix)]
    pub fn pause_group(&self) -> Result<(), ProcessGroupError> {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        if let Some(pgid) = self.inner.process_group_id {
            use nix::errno::Errno;
            match killpg(Pid::from_raw(pgid), Signal::SIGSTOP) {
                Ok(_) => Ok(()),
                Err(e) => match e {
                    Errno::ESRCH => Ok(()), // Already stopped/terminated
                    Errno::EPERM => Err(ProcessGroupError::SignalFailed(format!(
                        "Permission denied to pause process group {}",
                        pgid
                    ))),
                    _ => Err(ProcessGroupError::SignalFailed(format!(
                        "Failed to send SIGSTOP to process group {}: {}",
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

    /// Pauses all processes in the job object (Windows).
    ///
    /// Suspends all processes in the job object by iterating through them.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the processes were suspended successfully
    /// * `Err(ProcessGroupError)` - If pausing fails due to permissions or other errors
    ///
    /// # Example
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::group::builder::ProcessGroup;
    /// let mut group = ProcessGroup::new();
    /// // ... spawn processes in the group ...
    /// group.pause_group().unwrap();
    /// ```
    #[cfg(windows)]
    pub fn pause_group(&self) -> Result<(), ProcessGroupError> {
        use crate::tasks::process::group::builder::SendHandle;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        };
        use windows::Win32::System::JobObjects::{
            JOBOBJECT_BASIC_PROCESS_ID_LIST, JobObjectBasicProcessIdList, QueryInformationJobObject,
        };
        use windows::Win32::System::Threading::{OpenThread, SuspendThread, THREAD_SUSPEND_RESUME};

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

                let mut suspended_count = 0;

                // Suspend all threads in each process
                for i in 0..process_list.NumberOfProcessIdsInList {
                    let pid = process_list.ProcessIdList[i as usize] as u32;

                    // Take a snapshot of all threads in the system
                    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| {
                        ProcessGroupError::SignalFailed(format!(
                            "Failed to create thread snapshot: {}",
                            e
                        ))
                    })?;

                    let mut thread_entry = THREADENTRY32 {
                        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                        ..Default::default()
                    };

                    // Iterate through all threads and suspend those belonging to this process
                    if Thread32First(snapshot, &mut thread_entry).is_ok() {
                        loop {
                            if thread_entry.th32OwnerProcessID == pid {
                                let thread_handle = OpenThread(
                                    THREAD_SUSPEND_RESUME,
                                    false,
                                    thread_entry.th32ThreadID,
                                );
                                if let Ok(handle) = thread_handle {
                                    SuspendThread(handle);
                                    let _ = CloseHandle(handle);
                                    suspended_count += 1;
                                }
                            }

                            if Thread32Next(snapshot, &mut thread_entry).is_err() {
                                break;
                            }
                        }
                    }

                    let _ = CloseHandle(snapshot);
                }

                if suspended_count > 0 {
                    Ok(())
                } else {
                    Err(ProcessGroupError::SignalFailed(
                        "No threads were suspended in the job object".to_string(),
                    ))
                }
            }
        } else {
            Err(ProcessGroupError::SignalFailed(
                "No Job Object handle available".to_string(),
            ))
        }
    }

    /// Process group pausing is not available on this platform.
    #[cfg(not(any(unix, windows)))]
    pub fn pause_group(&self) -> Result<(), ProcessGroupError> {
        Err(ProcessGroupError::UnsupportedPlatform(
            "Process group pausing not available on this platform".to_string(),
        ))
    }
}

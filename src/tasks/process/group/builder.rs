use tokio::process::Command;

use crate::tasks::process::group::error::ProcessGroupError;

/// A cross-platform wrapper for managing process groups/jobs.
///
/// On Unix systems, this uses process groups with `setsid()`.
/// On Windows, this uses Job Objects for full process tree termination.
///
/// # Platform Support
/// - **Unix/Linux**: Full process group support using `setsid()` and `killpg()`
/// - **Windows**: Full process tree support using Job Objects
/// - **Other platforms**: No special handling
#[derive(Debug)]
pub struct ProcessGroup {
    pub(crate) inner: ProcessGroupInner,
}

#[derive(Debug)]
pub(crate) struct ProcessGroupInner {
    #[cfg(unix)]
    pub(crate) process_group_id: Option<i32>,
    #[cfg(windows)]
    pub(crate) job_handle: Option<SendHandle>,
    #[cfg(not(any(unix, windows)))]
    _phantom: (),
}

#[cfg(windows)]
#[derive(Debug)]
pub(crate) struct SendHandle(pub(crate) windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for SendHandle {}

#[cfg(windows)]
unsafe impl Sync for SendHandle {}

impl ProcessGroup {
    /// Create a new, inactive process group
    ///
    /// # Returns
    ///
    /// A new `ProcessGroup` instance that is not yet active
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tcrm_task::tasks::process::process_group::ProcessGroup;
    ///
    /// let group = ProcessGroup::new();
    /// assert!(!group.is_active());
    /// ```
    pub fn new() -> Self {
        Self {
            inner: ProcessGroupInner {
                #[cfg(unix)]
                process_group_id: None,
                #[cfg(windows)]
                job_handle: None,
                #[cfg(not(any(unix, windows)))]
                _phantom: (),
            },
        }
    }

    /// Check if the process group is active
    ///
    /// # Returns
    ///
    /// `true` if the process group has been created and is active, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tcrm_task::tasks::process::process_group::ProcessGroup;
    ///
    /// let group = ProcessGroup::new();
    /// assert!(!group.is_active());
    /// ```
    pub fn is_active(&self) -> bool {
        #[cfg(unix)]
        {
            self.inner.process_group_id.is_some()
        }
        #[cfg(windows)]
        {
            self.inner.job_handle.is_some()
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }
    /// Creates a new process group and configures the command to use it.
    ///
    /// This method prepares a Command to run as part of this process group. On Unix systems,
    /// it configures the command to create a new session and process group using setsid().
    /// On Windows, it configures the command to run in a new job object with appropriate
    /// creation flags.
    ///
    /// # Arguments
    ///
    /// * `command` - The Command to configure for process group execution
    ///
    /// # Returns
    ///
    /// * `Ok(Command)` - The configured command ready for execution
    /// * `Err(ProcessGroupError)` - If process group configuration fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tcrm_task::tasks::process::process_group::ProcessGroup;
    /// use tokio::process::Command;
    ///
    /// let mut group = ProcessGroup::new();
    /// let mut cmd = Command::new("echo");
    /// cmd.arg("hello");
    ///
    /// let configured_cmd = group.create_with_command(cmd).unwrap();
    /// // Command is now configured to run in the process group
    /// ```
    pub fn create_with_command(
        &mut self,
        #[allow(unused_mut)] mut command: Command,
    ) -> Result<Command, ProcessGroupError> {
        #[cfg(unix)]
        {
            // Configure the command to create a new session and process group
            unsafe {
                command.pre_exec(|| {
                    use nix::unistd::setsid;
                    if setsid().is_err() {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            Ok(command)
        }
        #[cfg(windows)]
        {
            use windows::Win32::System::JobObjects::JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            use windows::Win32::System::JobObjects::{
                CreateJobObjectW, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectExtendedLimitInformation, SetInformationJobObject,
            };
            use windows::core::PCWSTR;

            // Create a Job Object for the process group
            let job_handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| {
                ProcessGroupError::CreationFailed(format!("Failed to create Job Object: {}", e))
            })?;

            // Configure the job to kill all processes when the job handle is closed
            let mut job_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            job_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            unsafe {
                SetInformationJobObject(
                    job_handle,
                    JobObjectExtendedLimitInformation,
                    &job_info as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            }
            .map_err(|e| {
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(job_handle);
                }
                ProcessGroupError::CreationFailed(format!("Failed to configure Job Object: {}", e))
            })?;
            self.inner.job_handle = Some(SendHandle(job_handle));

            Ok(command)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(ProcessGroupError::UnsupportedPlatform(
                "Process group management not available on this platform".to_string(),
            ))
        }
    }

    /// Assigns a spawned child process to this process group/job.
    ///
    /// On Unix systems, this stores the process group ID. On Windows, this assigns
    /// the process to the job object for group management.
    ///
    /// After assignment, all future children of the process will be contained in the job, unless the process has
    /// breakaway privileges (which are not enabled by default in this implementation).
    ///
    /// # Arguments
    ///
    /// * `child_id` - The process ID of the child to assign to this group
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the assignment was successful
    /// * `Err(ProcessGroupError)` - If assignment fails or the platform is unsupported
    ///
    /// # Example
    ///
    /// ```rust
    /// use tcrm_task::tasks::process::process_group::ProcessGroup;
    /// use std::process::Command;
    ///
    /// let mut group = ProcessGroup::new();
    ///
    /// // After spawning a process, assign it to the group
    /// // let child = Command::new("echo").spawn()?;
    /// // group.assign_child(child.id())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Windows Race Condition Note
    /// On Windows, there is a well-known race condition: if a spawned process creates child processes
    /// before it is assigned to the job object, those children will not be part of the job
    /// and can escape containment.
    ///
    /// See: <https://devblogs.microsoft.com/oldnewthing/20130405-00/?p=4743>
    ///
    /// To avoid this issue, the process needs to be spawned in a suspended state,
    /// assigned to the job object, and only then resuming it. This ensures that no
    /// child processes can escape the job before assignment
    pub fn assign_child(&mut self, child_id: u32) -> Result<(), ProcessGroupError> {
        #[cfg(unix)]
        {
            self.inner.process_group_id = Some(child_id as i32);
            Ok(())
        }
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::JobObjects::AssignProcessToJobObject;
            use windows::Win32::System::Threading::{
                OpenProcess, PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
            };

            let process_handle = unsafe {
                OpenProcess(
                    PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_SET_INFORMATION,
                    false,
                    child_id,
                )
            }
            .map_err(|e| {
                ProcessGroupError::AssignmentFailed(format!("Failed to open process handle: {}", e))
            })?;

            let result = if let Some(SendHandle(job_handle)) = &self.inner.job_handle {
                unsafe { AssignProcessToJobObject(*job_handle, process_handle) }
            } else {
                unsafe {
                    let _ = CloseHandle(process_handle);
                }
                return Err(ProcessGroupError::AssignmentFailed(
                    "No Job Object handle available".to_string(),
                ));
            };

            unsafe {
                let _ = CloseHandle(process_handle);
            }

            result.map_err(|e| {
                ProcessGroupError::AssignmentFailed(format!(
                    "Failed to assign process to Job Object: {}",
                    e
                ))
            })?;
            Ok(())
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = child;
            Err(ProcessGroupError::UnsupportedPlatform(
                "Process group assignment not available on this platform".to_string(),
            ))
        }
    }
}

impl Drop for ProcessGroupInner {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if let Some(SendHandle(job_handle)) = self.job_handle.take() {
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(job_handle);
                }
            }
        }
    }
}

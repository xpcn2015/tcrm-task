//! Cross-platform process group management for killing entire process trees.
//!
//! This module provides utilities to manage process groups on Unix systems
//! and job objects on Windows to ensure that when a parent process is killed,
//! all its children and grandchildren are also terminated.

use std::sync::Arc;
use thiserror::Error;
use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

/// A cross-platform wrapper for managing process groups/jobs.
///
/// On Unix systems, this uses process groups with `setsid()`.
/// On Windows, this uses Job Objects for full process tree termination.
///
/// # Platform Support
/// - **Unix/Linux**: Full process group support using `setsid()` and `killpg()`
/// - **Windows**: Full process tree support using Job Objects
/// - **Other platforms**: No special handling
#[derive(Clone)]
pub struct ProcessGroup {
    inner: Arc<Mutex<ProcessGroupInner>>,
}

struct ProcessGroupInner {
    #[cfg(unix)]
    process_group_id: Option<i32>,
    #[cfg(windows)]
    job_handle: Option<SendHandle>,
    #[cfg(not(any(unix, windows)))]
    _phantom: (),
}

#[cfg(windows)]
#[derive(Debug)]
struct SendHandle(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for SendHandle {}

#[cfg(windows)]
unsafe impl Sync for SendHandle {}

/// Error type for process group operations.
#[derive(Error, Debug)]
pub enum ProcessGroupError {
    /// Failed to create process group/job
    #[error("Failed to create process group/job: {0}")]
    CreationFailed(String),
    /// Failed to assign process to group/job
    #[error("Failed to assign process to group/job: {0}")]
    AssignmentFailed(String),
    /// Failed to terminate process group/job
    #[error("Failed to terminate process group/job: {0}")]
    TerminationFailed(String),

    /// Unsupported platform for process group/job operations
    #[error("Unsupported platform: {0}")]
    #[allow(dead_code)]
    UnsupportedPlatform(String),
}

impl ProcessGroup {
    /// Creates a new process group and configures the command to use it.
    ///
    /// # Arguments
    /// * `command` - The command to configure for process group management
    ///
    /// # Returns
    /// A tuple of (configured_command, process_group) ready for spawning
    ///
    /// # Platform-specific behavior
    /// - **Unix**: Creates a new session and process group using `setsid()`
    /// - **Windows**: Creates a Job Object for process tree management
    /// - **Other platforms**: Returns command as-is with no special handling
    pub fn create_with_command(
        #[allow(unused_mut)] mut command: Command,
    ) -> Result<(Command, Self), ProcessGroupError> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // Configure the command to create a new session and process group
            unsafe {
                command.pre_exec(|| {
                    // Create a new session, making this process the session leader
                    // and creating a new process group
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let inner = ProcessGroupInner {
                process_group_id: None,
            };
            Ok((
                command,
                ProcessGroup {
                    inner: Arc::new(Mutex::new(inner)),
                },
            ))
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
            let job_handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) };
            let job_handle = match job_handle {
                Ok(h) => h,
                Err(e) => {
                    return Err(ProcessGroupError::CreationFailed(format!(
                        "Failed to create Job Object: {}",
                        e
                    )));
                }
            };

            // Configure the job to kill all processes when the job handle is closed
            let mut job_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            job_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let result = unsafe {
                SetInformationJobObject(
                    job_handle,
                    JobObjectExtendedLimitInformation,
                    &job_info as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };

            if let Err(e) = result {
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(job_handle);
                }
                return Err(ProcessGroupError::CreationFailed(format!(
                    "Failed to configure Job Object: {}",
                    e
                )));
            }

            let inner = ProcessGroupInner {
                job_handle: Some(SendHandle(job_handle)),
            };
            Ok((
                command,
                ProcessGroup {
                    inner: Arc::new(Mutex::new(inner)),
                },
            ))
        }
        #[cfg(not(any(unix, windows)))]
        {
            // Other platforms are not supported for process group management
            Err(ProcessGroupError::UnsupportedPlatform(
                "Process group management not available on this platform".to_string(),
            ))
        }
    }

    /// Assigns a spawned child process to this process group.
    ///
    /// # Arguments
    /// * `child` - The child process to assign to the group
    ///
    /// # Platform-specific behavior
    /// - **Unix**: Stores the process group ID from the child process
    /// - **Windows**: Assigns the process to the Job Object for process tree management  
    /// - **Other platforms**: Returns an error indicating unsupported functionality
    ///
    /// # Errors
    /// Returns `ProcessGroupError::AssignmentFailed` if the process cannot be assigned,
    /// or `ProcessGroupError::UnsupportedPlatform` on unsupported platforms.
    pub async fn assign_child(&self, child: &Child) -> Result<(), ProcessGroupError> {
        #[cfg(unix)]
        {
            let mut inner = self.inner.lock().await;
            inner.process_group_id = child.id().map(|id| id as i32);
            Ok(())
        }
        #[cfg(windows)]
        {
            use windows::Win32::System::JobObjects::AssignProcessToJobObject;
            use windows::Win32::System::Threading::OpenProcess;
            use windows::Win32::System::Threading::PROCESS_ALL_ACCESS;
            let inner = self.inner.lock().await;
            if let Some(pid) = child.id() {
                let process_handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid) };
                let process_handle = match process_handle {
                    Ok(h) => h,
                    Err(e) => {
                        return Err(ProcessGroupError::AssignmentFailed(format!(
                            "Failed to open process handle: {}",
                            e
                        )));
                    }
                };
                if let Some(SendHandle(job_handle)) = &inner.job_handle {
                    let result = unsafe { AssignProcessToJobObject(*job_handle, process_handle) };
                    if let Err(e) = result {
                        return Err(ProcessGroupError::AssignmentFailed(format!(
                            "Failed to assign process to Job Object: {}",
                            e
                        )));
                    }
                } else {
                    return Err(ProcessGroupError::AssignmentFailed(
                        "No Job Object handle available".to_string(),
                    ));
                }
            }
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

    /// Terminates the entire process group/job, killing all processes in it.
    ///
    /// This will kill the original process and all its children and grandchildren.
    ///
    /// # Platform-specific behavior
    /// - **Unix**: Uses `killpg()` to terminate the entire process group with SIGKILL
    /// - **Windows**: Uses `TerminateJobObject()` to terminate all processes in the Job Object
    /// - **Other platforms**: Returns an error indicating unsupported functionality
    ///
    /// # Errors
    /// Returns `ProcessGroupError::TerminationFailed` if termination fails,
    /// or `ProcessGroupError::UnsupportedPlatform` on unsupported platforms.
    pub async fn terminate_all(&self) -> Result<(), ProcessGroupError> {
        #[cfg(unix)]
        {
            let inner = self.inner.lock().await;
            if let Some(pgid) = inner.process_group_id {
                use nix::sys::signal::{Signal, killpg};
                use nix::unistd::Pid;

                // Kill the entire process group
                killpg(Pid::from_raw(pgid), Signal::SIGKILL).map_err(|e| {
                    ProcessGroupError::TerminationFailed(format!("killpg failed: {}", e))
                })?;
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            use windows::Win32::System::JobObjects::TerminateJobObject;

            let inner = self.inner.lock().await;
            if let Some(SendHandle(job_handle)) = inner.job_handle {
                unsafe {
                    // Terminate all processes in the job object
                    match TerminateJobObject(job_handle, 1) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(ProcessGroupError::TerminationFailed(format!(
                            "Failed to terminate job object: {}",
                            e
                        ))),
                    }
                }
            } else {
                Err(ProcessGroupError::TerminationFailed(
                    "No job object handle available for termination".to_string(),
                ))
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            // On other platforms, we can't kill process groups
            // The caller should fall back to individual process termination
            Err(ProcessGroupError::UnsupportedPlatform(
                "Process group termination not available on this platform".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::process::Command;

    #[tokio::test]
    async fn test_process_group_creation() {
        let command = Command::new("echo");
        let result = ProcessGroup::create_with_command(command);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_tree_termination() {
        // This test demonstrates the cross-platform process tree termination
        // On Unix systems, it will kill the entire process group
        // On Windows systems, it will use Job Objects
        // On other systems, it falls back to individual process termination

        #[cfg(windows)]
        let config = crate::tasks::config::TaskConfig::new("cmd")
            .args(["/C", "ping", "127.0.0.1", "-n", "10"]) // 10 second ping
            .timeout_ms(5000); // 5 second timeout

        #[cfg(unix)]
        let config = crate::tasks::config::TaskConfig::new("sleep")
            .args(["10"]) // 10 second sleep
            .timeout_ms(5000); // 5 second timeout

        #[cfg(not(any(windows, unix)))]
        let config = crate::tasks::config::TaskConfig::new("echo").args(["test"]);

        let mut spawner = crate::tasks::async_tokio::spawner::TaskSpawner::new(
            "test-process-tree".to_string(),
            config,
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Start the task
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok(), "Failed to start task: {:?}", result);

        // Wait a moment for the process to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Manually terminate the task (this should kill the process tree)
        let terminate_result = spawner
            .send_terminate_signal(crate::tasks::event::TaskTerminateReason::UserRequested)
            .await;
        assert!(
            terminate_result.is_ok(),
            "Failed to send terminate signal: {:?}",
            terminate_result
        );

        // Wait for termination events
        let mut started = false;

        while let Some(event) = rx.recv().await {
            match event {
                crate::tasks::event::TaskEvent::Started { .. } => {
                    started = true;
                }
                crate::tasks::event::TaskEvent::Stopped { reason, .. } => {
                    if matches!(
                        reason,
                        crate::tasks::event::TaskEventStopReason::Terminated(_)
                    ) {
                        break;
                    }
                }
                crate::tasks::event::TaskEvent::Error { .. } => {
                    // Errors are acceptable in this test case
                    break;
                }
                _ => {}
            }
        }

        assert!(started, "Task should have started");
    }
}

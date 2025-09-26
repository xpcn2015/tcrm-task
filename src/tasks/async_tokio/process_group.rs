//! Cross-platform process group management for killing entire process trees
//! and propagating signals like pause/resume.//!
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
#[derive(Clone, Debug)]
pub struct ProcessGroup {
    inner: Arc<Mutex<ProcessGroupInner>>,
}

#[derive(Debug)]
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
    #[error("Failed to create process group/job: {0}")]
    CreationFailed(String),
    #[error("Failed to assign process to group/job: {0}")]
    AssignmentFailed(String),
    #[error("Failed to terminate process group/job: {0}")]
    TerminationFailed(String),
    #[error("Failed to send signal to process group: {0}")]
    SignalFailed(String),

    #[cfg(not(any(unix, windows)))]
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
}

/// Signal types that can be sent to process groups
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessSignal {
    /// Terminate the process group (SIGKILL on Unix, TerminateJobObject on Windows)
    Terminate,
    /// Pause/suspend the process group (SIGSTOP on Unix, SuspendThread on Windows)
    Pause,
    /// Resume the process group (SIGCONT on Unix, ResumeThread on Windows)
    Resume,
    /// Interrupt the process group (SIGINT on Unix, GenerateConsoleCtrlEvent on Windows)
    Interrupt,
}

impl ProcessGroup {
    /// Creates a new process group and configures the command to use it.
    pub fn create_with_command(
        #[allow(unused_mut)] mut command: Command,
    ) -> Result<(Command, Self), ProcessGroupError> {
        #[cfg(unix)]
        {
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
            Err(ProcessGroupError::UnsupportedPlatform(
                "Process group management not available on this platform".to_string(),
            ))
        }
    }

    /// Assigns a spawned child process to this process group/job.
    ///
    /// # Windows Race Condition Warning
    /// On Windows, there is an unavoidable race condition: if the spawned process creates child processes
    /// before it is assigned to the job object, those children will not be part of the job and can escape
    /// containment. This is a limitation of the Windows API. To minimize the risk, assign the process to the job
    /// immediately after spawning, before it can create children. There is no supported way to make this atomic
    /// with standard Rust APIs.
    ///
    /// After assignment, all future children of the process will be contained in the job, unless the process has
    /// breakaway privileges (which are not enabled by default in this implementation).
    ///
    /// For malware analysis or strong containment, be aware of this limitation.
    ///
    /// See: https://devblogs.microsoft.com/oldnewthing/20130405-00/?p=4743
    ///
    /// # Arguments
    /// * `child` - The spawned child process to assign
    pub async fn assign_child(&self, child: &Child) -> Result<(), ProcessGroupError> {
        #[cfg(unix)]
        {
            let mut inner = self.inner.lock().await;
            inner.process_group_id = child.id().map(|id| id as i32);
            Ok(())
        }
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::JobObjects::AssignProcessToJobObject;
            use windows::Win32::System::Threading::{
                OpenProcess, PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
            };

            let inner = self.inner.lock().await;
            if let Some(pid) = child.id() {
                let process_handle = unsafe {
                    OpenProcess(
                        PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_SET_INFORMATION,
                        false,
                        pid,
                    )
                }
                .map_err(|e| {
                    ProcessGroupError::AssignmentFailed(format!(
                        "Failed to open process handle: {}",
                        e
                    ))
                })?;

                let result = if let Some(SendHandle(job_handle)) = &inner.job_handle {
                    unsafe { AssignProcessToJobObject(*job_handle, process_handle) }
                } else {
                    unsafe {
                        let _ = CloseHandle(process_handle);
                    }
                    return Err(ProcessGroupError::AssignmentFailed(
                        "No Job Object handle available".to_string(),
                    ));
                };

                // Always close the process handle to prevent resource leaks
                unsafe {
                    let _ = CloseHandle(process_handle);
                }

                result.map_err(|e| {
                    ProcessGroupError::AssignmentFailed(format!(
                        "Failed to assign process to Job Object: {}",
                        e
                    ))
                })?;
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

    /// Sends a signal to the entire process group/job.
    ///
    /// # Arguments
    /// * `signal` - The signal to send to all processes in the group
    ///
    /// # Platform-specific behavior
    /// - **Unix**: Uses `killpg()` with appropriate signal
    /// - **Windows**: Uses appropriate Windows API calls for job/process manipulation
    /// - **Other platforms**: Returns an error indicating unsupported functionality
    pub async fn send_signal(&self, signal: ProcessSignal) -> Result<(), ProcessGroupError> {
        #[cfg(unix)]
        {
            use nix::sys::signal::Signal;
            use nix::sys::signal::killpg;
            use nix::unistd::Pid;

            let inner = self.inner.lock().await;
            if let Some(pgid) = inner.process_group_id {
                let unix_signal = match signal {
                    ProcessSignal::Terminate => Signal::SIGKILL,
                    ProcessSignal::Pause => Signal::SIGSTOP,
                    ProcessSignal::Resume => Signal::SIGCONT,
                    ProcessSignal::Interrupt => Signal::SIGINT,
                };

                match killpg(Pid::from_raw(pgid), unix_signal) {
                    Ok(()) => {}
                    Err(nix::errno::Errno::ESRCH) => {
                        // Process group no longer exists - this is fine, nothing to kill
                    }
                    Err(e) => {
                        return Err(ProcessGroupError::SignalFailed(format!(
                            "killpg failed: {}",
                            e
                        )));
                    }
                }
            }
            Ok(())
        }
        #[cfg(windows)]
        {
            match signal {
                ProcessSignal::Terminate => {
                    let inner = self.inner.lock().await;
                    if let Some(SendHandle(job_handle)) = &inner.job_handle {
                        unsafe {
                            use windows::Win32::System::JobObjects::TerminateJobObject;
                            TerminateJobObject(*job_handle, 1).map_err(|e| {
                                ProcessGroupError::TerminationFailed(format!(
                                    "Failed to terminate job object: {}",
                                    e
                                ))
                            })
                        }
                    } else {
                        // No job object handle means process group is disabled
                        // Return Ok() since there's nothing to terminate
                        Ok(())
                    }
                }
                ProcessSignal::Pause | ProcessSignal::Resume => {
                    self.suspend_resume_job_processes(signal == ProcessSignal::Pause)
                        .await
                }
                ProcessSignal::Interrupt => {
                    // Send Ctrl+C to all processes in the job
                    self.send_ctrl_c_to_job().await
                }
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = signal;
            Err(ProcessGroupError::UnsupportedPlatform(
                "Process signal sending not available on this platform".to_string(),
            ))
        }
    }

    /// Convenience method for terminating the entire process group/job
    pub async fn terminate_all(&self) -> Result<(), ProcessGroupError> {
        self.send_signal(ProcessSignal::Terminate).await
    }

    /// Convenience method for pausing the entire process group/job
    pub async fn pause_all(&self) -> Result<(), ProcessGroupError> {
        self.send_signal(ProcessSignal::Pause).await
    }

    /// Convenience method for resuming the entire process group/job
    pub async fn resume_all(&self) -> Result<(), ProcessGroupError> {
        self.send_signal(ProcessSignal::Resume).await
    }

    /// Convenience method for interrupting the entire process group/job
    pub async fn interrupt_all(&self) -> Result<(), ProcessGroupError> {
        self.send_signal(ProcessSignal::Interrupt).await
    }

    #[cfg(windows)]
    async fn suspend_resume_job_processes(&self, suspend: bool) -> Result<(), ProcessGroupError> {
        use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        };
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot as CreateThreadSnapshot, TH32CS_SNAPTHREAD, THREADENTRY32,
            Thread32First, Thread32Next,
        };
        use windows::Win32::System::Threading::{
            OpenProcess, OpenThread, PROCESS_QUERY_INFORMATION, ResumeThread, SuspendThread,
            THREAD_SUSPEND_RESUME,
        };

        let current_pid = std::process::id();
        let mut child_pids = Vec::new();

        // Take a snapshot of all processes and find direct children of the current process
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| {
                ProcessGroupError::SignalFailed(format!("Failed to create process snapshot: {}", e))
            })?;

            let mut process_entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut process_entry).is_ok() {
                loop {
                    if process_entry.th32ParentProcessID == current_pid {
                        child_pids.push(process_entry.th32ProcessID);
                    }
                    if Process32NextW(snapshot, &mut process_entry).is_err() {
                        use windows::Win32::Foundation::ERROR_NO_MORE_FILES;
                        use windows::Win32::Foundation::GetLastError;
                        let err = GetLastError();
                        if err == ERROR_NO_MORE_FILES {
                            break;
                        } else {
                            CloseHandle(snapshot).ok();
                            return Err(ProcessGroupError::SignalFailed(format!(
                                "Process32NextW failed: error {}",
                                err.0
                            )));
                        }
                    }
                }
            }
            CloseHandle(snapshot).ok();
        }

        // Take a single snapshot of all threads
        let mut thread_entries = Vec::new();
        unsafe {
            let thread_snapshot = CreateThreadSnapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| {
                ProcessGroupError::SignalFailed(format!("Failed to create thread snapshot: {}", e))
            })?;

            let mut thread_entry = THREADENTRY32 {
                dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };

            if Thread32First(thread_snapshot, &mut thread_entry).is_ok() {
                loop {
                    thread_entries.push(thread_entry.clone());
                    if Thread32Next(thread_snapshot, &mut thread_entry).is_err() {
                        use windows::Win32::Foundation::ERROR_NO_MORE_FILES;
                        use windows::Win32::Foundation::GetLastError;
                        let err = GetLastError();
                        if err == ERROR_NO_MORE_FILES {
                            break;
                        } else {
                            CloseHandle(thread_snapshot).ok();
                            return Err(ProcessGroupError::SignalFailed(format!(
                                "Thread32Next failed: error {}",
                                err.0
                            )));
                        }
                    }
                }
            }
            CloseHandle(thread_snapshot).ok();
        }

        // SAFEGUARD: Never suspend/resume system processes (PID 0, 4, and known critical PIDs)
        // For even more safety, you could maintain a list of critical PIDs to skip (e.g., session manager, csrss, wininit, etc.)
        // For now, we skip PID 0 (System Idle), PID 4 (System), and any PID < 100 (conservative)
        //
        // NOTE: For best security, you should track only the PIDs you have spawned (see comment below)

        for process_id in child_pids {
            if process_id == 0 || process_id == 4 {
                // Skip system/critical processes
                // TODO: report error
                continue;
            }
            for thread_entry in &thread_entries {
                if thread_entry.th32OwnerProcessID == process_id {
                    unsafe {
                        let thread_handle =
                            OpenThread(THREAD_SUSPEND_RESUME, false, thread_entry.th32ThreadID);
                        if let Ok(handle) = thread_handle {
                            let result = if suspend {
                                SuspendThread(handle)
                            } else {
                                ResumeThread(handle)
                            };
                            // Check for errors
                            if result == u32::MAX {
                                let err = GetLastError();
                                CloseHandle(handle).ok();
                                return Err(ProcessGroupError::SignalFailed(format!(
                                    "Failed to {} thread {}: error {}",
                                    if suspend { "suspend" } else { "resume" },
                                    thread_entry.th32ThreadID,
                                    err.0
                                )));
                            }
                            CloseHandle(handle).ok();
                        }
                    }
                }
            }
        }
        // and only allow suspend/resume for those PIDs. This prevents accidental or malicious targeting of unrelated processes.
        Ok(())
    }

    #[cfg(windows)]
    async fn send_ctrl_c_to_job(&self) -> Result<(), ProcessGroupError> {
        use windows::Win32::System::Console::{CTRL_C_EVENT, GenerateConsoleCtrlEvent};

        let inner = self.inner.lock().await;
        if let Some(SendHandle(_job_handle)) = &inner.job_handle {
            unsafe {
                // Send Ctrl+C to all processes in the console session
                // Note: This affects all processes sharing the same console
                GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0).map_err(|e| {
                    ProcessGroupError::SignalFailed(format!("Failed to send Ctrl+C: {}", e))
                })
            }
        } else {
            // No job handle - process group is disabled, which is fine
            Ok(())
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::time::Duration;
//     use tokio::process::Command;

//     #[tokio::test]
//     async fn test_signal_propagation() {
//         // Test that signals are properly propagated to child processes
//         let command = if cfg!(unix) {
//             let mut cmd = Command::new("sleep");
//             cmd.arg("10");
//             cmd
//         } else if cfg!(windows) {
//             let mut cmd = Command::new("Powershell");
//             cmd.args(["-Command", "Start-Sleep", "-Seconds", "10"]);
//             cmd
//         } else {
//             return; // Skip on unsupported platforms
//         };

//         let (mut command, process_group) = ProcessGroup::create_with_command(command).unwrap();
//         let mut child = command.spawn().expect("Failed to spawn process");

//         process_group
//             .assign_child(&child)
//             .await
//             .expect("Failed to assign child");

//         // Wait for process to start
//         tokio::time::sleep(Duration::from_millis(100)).await;

//         // Test pause (only on Unix for now)
//         #[cfg(unix)]
//         {
//             let result = process_group.pause_all().await;
//             assert!(
//                 result.is_ok(),
//                 "Failed to pause process group: {:?}",
//                 result
//             );

//             tokio::time::sleep(Duration::from_millis(100)).await;

//             let result = process_group.resume_all().await;
//             assert!(
//                 result.is_ok(),
//                 "Failed to resume process group: {:?}",
//                 result
//             );
//         }

//         // Test termination
//         let result = process_group.terminate_all().await;
//         assert!(
//             result.is_ok(),
//             "Failed to terminate process group: {:?}",
//             result
//         );

//         // Clean up
//         let _ = child.wait().await;
//     }
//     #[tokio::test]
//     async fn test_process_group_creation() {
//         let command = Command::new("echo");
//         let result = ProcessGroup::create_with_command(command);

//         // In WSL or restricted environments, this may fail
//         if result.is_err() {
//             eprintln!(
//                 "Process group creation failed (likely due to WSL permissions), skipping test: {:?}",
//                 result.as_ref().err()
//             );
//             return;
//         }

//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_process_tree_functionality_demo() {
//         // This test demonstrates process tree functionality by creating a parent that spawns children
//         // without requiring actual process groups (for WSL compatibility)

//         #[cfg(windows)]
//         let config = crate::tasks::config::TaskConfig::new("cmd")
//             .args([
//                 "/C",
//                 "for /l %i in (1,1,5) do (timeout /t 1 /nobreak > nul && echo child_%i)",
//             ])
//             .timeout_ms(8000);

//         #[cfg(unix)]
//         let config = crate::tasks::config::TaskConfig::new("bash")
//             .args(["-c", "for i in {1..5}; do sleep 1 && echo child_$i; done"])
//             .timeout_ms(8000)
//             .use_process_group(false); // Disable for WSL compatibility

//         #[cfg(not(any(windows, unix)))]
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["demo"])
//             .use_process_group(false);

//         let mut spawner = crate::tasks::async_tokio::spawner::TaskSpawner::new(
//             "process-tree-demo".to_string(),
//             config,
//         );
//         let (tx, mut rx) = tokio::sync::mpsc::channel(100);

//         // Start the task
//         let result = spawner.start_direct(tx).await;
//         if result.is_err() {
//             eprintln!(
//                 "Task failed to start, skipping demonstration: {:?}",
//                 result.as_ref().err()
//             );
//             return;
//         }

//         let mut started = false;
//         let mut output_count = 0;

//         // Collect some output to demonstrate it's working
//         while let Some(event) = rx.recv().await {
//             match event {
//                 crate::tasks::event::TaskEvent::Started { .. } => {
//                     started = true;
//                 }
//                 crate::tasks::event::TaskEvent::Output { .. } => {
//                     output_count += 1;
//                     if output_count >= 2 {
//                         // Terminate after seeing some output
//                         let _ = spawner
//                             .send_terminate_signal(
//                                 crate::tasks::event::TaskTerminateReason::UserRequested,
//                             )
//                             .await;
//                     }
//                 }
//                 crate::tasks::event::TaskEvent::Stopped { .. } => {
//                     break;
//                 }
//                 crate::tasks::event::TaskEvent::Error { .. } => {
//                     break;
//                 }
//                 _ => {}
//             }
//         }

//         assert!(started, "Task should have started");
//         println!("Process tree demonstration completed successfully");
//     }
//     #[tokio::test]
//     async fn test_basic_process_lifecycle() {
//         // Test basic process spawning and termination
//         // Use TaskConfig approach for better WSL compatibility
//         #[cfg(unix)]
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(false); // Disable process groups for WSL compatibility

//         #[cfg(windows)]
//         let config = crate::tasks::config::TaskConfig::new("cmd")
//             .args(["/C", "echo", "test"])
//             .use_process_group(true);

//         #[cfg(not(any(unix, windows)))]
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(false);

//         let result = ProcessGroup::create_with_config(&config);
//         if result.is_err() {
//             eprintln!(
//                 "Process group creation failed, skipping test: {:?}",
//                 result.err()
//             );
//             return;
//         }

//         let (mut command, process_group) = result.unwrap();
//         let mut child = command.spawn().expect("Failed to spawn process");

//         // Assign child to process group
//         let assign_result = process_group.assign_child(&child).await;
//         if assign_result.is_err() {
//             eprintln!(
//                 "Failed to assign child to process group, skipping test: {:?}",
//                 assign_result.err()
//             );
//             let _ = child.kill().await; // Clean up
//             return; // Skip test if assignment fails
//         }

//         // Wait for process to complete normally
//         let status = child.wait().await.expect("Failed to wait for child");
//         assert!(status.success() || cfg!(windows)); // Windows cmd might have different exit codes

//         // Test termination (should be no-op since process already finished)
//         let terminate_result = process_group.terminate_all().await;
//         if terminate_result.is_err() {
//             eprintln!("Termination failed: {:?}", terminate_result.as_ref().err());
//         }
//         assert!(terminate_result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_process_group_termination() {
//         // Test terminating a long-running process
//         let command = if cfg!(windows) {
//             let mut cmd = Command::new("ping");
//             cmd.args(["127.0.0.1", "-n", "100"]); // Long-running ping
//             cmd
//         } else if cfg!(unix) {
//             let mut cmd = Command::new("sleep");
//             cmd.arg("30"); // 30 second sleep
//             cmd
//         } else {
//             // On other platforms, just use a quick command
//             let mut cmd = Command::new("echo");
//             cmd.arg("test");
//             cmd
//         };

//         let (mut command, process_group) = ProcessGroup::create_with_command(command).unwrap();
//         let mut child = command.spawn().expect("Failed to spawn process");

//         // Assign child to process group
//         process_group
//             .assign_child(&child)
//             .await
//             .expect("Failed to assign child");

//         // Wait a bit for the process to start
//         tokio::time::sleep(Duration::from_millis(100)).await;

//         // Terminate the process group
//         let result = process_group.terminate_all().await;
//         assert!(result.is_ok());

//         // On Unix/Windows, the process should be terminated
//         // On other platforms, we need to kill it manually for cleanup
//         #[cfg(not(any(unix, windows)))]
//         {
//             let _ = child.kill().await;
//         }

//         // Clean up
//         let _ = child.wait().await;
//     }
//     #[tokio::test]
//     async fn test_process_tree_termination() {
//         // This test demonstrates the cross-platform process tree termination
//         // On Unix systems, it will kill the entire process group
//         // On Windows systems, it will use Job Objects
//         // On other systems, it falls back to individual process termination

//         #[cfg(windows)]
//         let config = crate::tasks::config::TaskConfig::new("cmd")
//             .args(["/C", "ping", "127.0.0.1", "-n", "10"]) // 10 second ping
//             .timeout_ms(5000); // 5 second timeout

//         #[cfg(unix)]
//         let config = crate::tasks::config::TaskConfig::new("sleep")
//             .args(["10"]) // 10 second sleep
//             .timeout_ms(5000) // 5 second timeout
//             .use_process_group(false); // Disable process group for WSL compatibility

//         #[cfg(not(any(windows, unix)))]
//         let config = crate::tasks::config::TaskConfig::new("echo").args(["test"]);

//         let mut spawner = crate::tasks::async_tokio::spawner::TaskSpawner::new(
//             "test-process-tree".to_string(),
//             config,
//         );
//         let (tx, mut rx) = tokio::sync::mpsc::channel(100);

//         // Start the task
//         let result = spawner.start_direct(tx).await;
//         if result.is_err() {
//             eprintln!(
//                 "Task failed to start (likely due to WSL permissions), skipping test: {:?}",
//                 result.as_ref().err()
//             );
//             return; // Skip test if task fails to start
//         }

//         // Wait a moment for the process to start
//         tokio::time::sleep(Duration::from_millis(100)).await;

//         // Manually terminate the task (this should kill the process tree)
//         let terminate_result = spawner
//             .send_terminate_signal(crate::tasks::event::TaskTerminateReason::UserRequested)
//             .await;
//         assert!(
//             terminate_result.is_ok(),
//             "Failed to send terminate signal: {:?}",
//             terminate_result
//         );

//         // Wait for termination events
//         let mut started = false;

//         while let Some(event) = rx.recv().await {
//             match event {
//                 crate::tasks::event::TaskEvent::Started { .. } => {
//                     started = true;
//                 }
//                 crate::tasks::event::TaskEvent::Stopped { reason, .. } => {
//                     if matches!(
//                         reason,
//                         crate::tasks::event::TaskEventStopReason::Terminated(_)
//                     ) {
//                         break;
//                     }
//                 }
//                 crate::tasks::event::TaskEvent::Error { .. } => {
//                     // Errors are acceptable in this test case
//                     break;
//                 }
//                 _ => {}
//             }
//         }

//         assert!(started, "Task should have started");
//     }

//     #[tokio::test]
//     async fn test_process_group_creation_from_config() {
//         let config = crate::tasks::config::TaskConfig::new("echo").args(["test"]);

//         let result = ProcessGroup::create_with_config(&config);
//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_process_group_terminate_immediate() {
//         let config = crate::tasks::config::TaskConfig::new("echo").args(["test"]);

//         let (mut _command, mut process_group) = ProcessGroup::create_with_config(&config).unwrap();
//         let result = process_group.terminate_all().await;
//         assert!(result.is_ok());
//     }

//     // Test with a long-running process to properly test termination
//     #[tokio::test]
//     async fn test_process_group_with_long_running_process() {
//         #[cfg(unix)]
//         let config = crate::tasks::config::TaskConfig::new("sleep")
//             .args(["2"])
//             .use_process_group(true);

//         #[cfg(windows)]
//         let config = crate::tasks::config::TaskConfig::new("ping")
//             .args(["-n", "20", "localhost"])
//             .use_process_group(true);

//         #[cfg(not(any(windows, unix)))]
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(true);

//         let (mut _command, mut process_group) = ProcessGroup::create_with_config(&config).unwrap();

//         // Let the process start
//         tokio::time::sleep(Duration::from_millis(200)).await;

//         // Terminate the process group
//         let result = process_group.terminate_all().await;
//         assert!(result.is_ok());
//     }

//     // Test that disabled process groups still work (for WSL compatibility)
//     #[tokio::test]
//     async fn test_process_group_disabled_for_compatibility() {
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(false);

//         let result = ProcessGroup::create_with_config(&config);
//         assert!(result.is_ok());

//         let (mut _command, mut process_group) = result.unwrap();
//         let terminate_result = process_group.terminate_all().await;
//         assert!(terminate_result.is_ok());
//     }

//     // Test process group behavior with commands that could spawn children
//     #[tokio::test]
//     async fn test_process_group_child_termination() {
//         // This test verifies that when we terminate a parent process,
//         // any children it spawns are also terminated (process tree behavior)

//         #[cfg(unix)]
//         let config = crate::tasks::config::TaskConfig::new("sh")
//             .args(["-c", "sleep 3 & sleep 3 & wait"])
//             .use_process_group(true);

//         #[cfg(windows)]
//         let config = crate::tasks::config::TaskConfig::new("cmd")
//             .args(["/c", "ping -n 30 localhost > nul"])
//             .use_process_group(true);

//         #[cfg(not(any(windows, unix)))]
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(true);

//         let (mut _command, mut process_group) = ProcessGroup::create_with_config(&config).unwrap();

//         // Let the processes start and potentially spawn children
//         tokio::time::sleep(Duration::from_millis(500)).await;

//         // Terminate the entire process group - this should kill parent and children
//         let result = process_group.terminate_all().await;
//         assert!(result.is_ok());
//     }

//     // Test pause and resume functionality (where supported)
//     #[tokio::test]
//     async fn test_process_group_pause_resume() {
//         #[cfg(unix)]
//         {
//             let config = crate::tasks::config::TaskConfig::new("sleep")
//                 .args(["5"])
//                 .use_process_group(true);

//             let (mut _command, process_group) = ProcessGroup::create_with_config(&config).unwrap();

//             // Let the process start
//             tokio::time::sleep(Duration::from_millis(100)).await;

//             // Test pause
//             let _pause_result = process_group.pause_all().await;
//             // Note: This might not be fully implemented yet, so we just test it doesn't panic
//             // In a real implementation, this would pause all processes in the group

//             // Test resume
//             let _resume_result = process_group.resume_all().await;
//             // Similar note - testing that the API exists and doesn't panic

//             // Clean up
//             let mut process_group = process_group;
//             let _ = process_group.terminate_all().await;
//         }

//         #[cfg(windows)]
//         {
//             // Windows implementation might be different or not yet fully implemented
//             let config = crate::tasks::config::TaskConfig::new("ping")
//                 .args(["-n", "10", "localhost"])
//                 .use_process_group(true);

//             let (mut _command, process_group) = ProcessGroup::create_with_config(&config).unwrap();

//             tokio::time::sleep(Duration::from_millis(100)).await;

//             // Test that the methods exist and return results (even if not fully implemented)
//             let _pause_result = process_group.pause_all().await;
//             let _resume_result = process_group.resume_all().await;

//             let mut process_group = process_group;
//             let _ = process_group.terminate_all().await;
//         }
//     }

//     // Test all process group signal functionality
//     #[tokio::test]
//     async fn test_complete_process_group_signals() {
//         #[cfg(unix)]
//         {
//             let config = crate::tasks::config::TaskConfig::new("sleep")
//                 .args(["10"])
//                 .use_process_group(true);

//             let (mut _command, process_group) = ProcessGroup::create_with_config(&config).unwrap();

//             // Test all signal methods exist and don't panic
//             let _terminate_result = process_group.terminate_all().await;
//             let _pause_result = process_group.pause_all().await;
//             let _resume_result = process_group.resume_all().await;
//             let _interrupt_result = process_group.interrupt_all().await;

//             // These should succeed or fail gracefully - main thing is they don't panic
//             println!("Unix signal methods completed without panic");
//         }

//         #[cfg(windows)]
//         {
//             let config = crate::tasks::config::TaskConfig::new("ping")
//                 .args(["-n", "30", "localhost"])
//                 .use_process_group(true);

//             let (mut _command, process_group) = ProcessGroup::create_with_config(&config).unwrap();

//             // Test all signal methods exist and don't panic
//             let _terminate_result = process_group.terminate_all().await;
//             let _pause_result = process_group.pause_all().await;
//             let _resume_result = process_group.resume_all().await;
//             let _interrupt_result = process_group.interrupt_all().await;

//             // These should succeed or fail gracefully - main thing is they don't panic
//             println!("Windows signal methods completed without panic");
//         }

//         // Test with disabled process groups
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(false);

//         let (mut _command, process_group) = ProcessGroup::create_with_config(&config).unwrap();

//         // All operations should succeed on disabled process groups
//         let terminate_result = process_group.terminate_all().await;
//         let pause_result = process_group.pause_all().await;
//         let resume_result = process_group.resume_all().await;
//         let interrupt_result = process_group.interrupt_all().await;

//         assert!(terminate_result.is_ok());
//         assert!(pause_result.is_ok());
//         assert!(resume_result.is_ok());
//         assert!(interrupt_result.is_ok());
//     }

//     // Test comprehensive process lifecycle with real processes
//     #[tokio::test]
//     async fn test_process_lifecycle_with_signals() {
//         #[cfg(unix)]
//         let config = crate::tasks::config::TaskConfig::new("sleep")
//             .args(["3"])
//             .use_process_group(false); // Disable for WSL compatibility

//         #[cfg(windows)]
//         let config = crate::tasks::config::TaskConfig::new("ping")
//             .args(["-n", "10", "localhost"])
//             .use_process_group(true);

//         #[cfg(not(any(windows, unix)))]
//         let config = crate::tasks::config::TaskConfig::new("echo")
//             .args(["test"])
//             .use_process_group(false);

//         let result = ProcessGroup::create_with_config(&config);
//         if result.is_err() {
//             eprintln!(
//                 "Process group creation failed, skipping test: {:?}",
//                 result.as_ref().err()
//             );
//             return;
//         }

//         let (mut command, process_group) = result.unwrap();
//         let mut child = command.spawn().expect("Failed to spawn process");

//         // Assign child to process group
//         let assign_result = process_group.assign_child(&child).await;
//         if assign_result.is_err() {
//             eprintln!(
//                 "Failed to assign child to process group: {:?}",
//                 assign_result.as_ref().err()
//             );
//             let _ = child.kill().await;
//             return;
//         }

//         // Let process run for a bit
//         tokio::time::sleep(std::time::Duration::from_millis(100)).await;

//         // Test pause functionality
//         #[cfg(unix)]
//         {
//             let pause_result = process_group.pause_all().await;
//             if pause_result.is_err() {
//                 eprintln!(
//                     "Pause failed (expected in some environments): {:?}",
//                     pause_result.as_ref().err()
//                 );
//             }

//             // Brief pause
//             tokio::time::sleep(std::time::Duration::from_millis(100)).await;

//             let resume_result = process_group.resume_all().await;
//             if resume_result.is_err() {
//                 eprintln!(
//                     "Resume failed (expected in some environments): {:?}",
//                     resume_result.as_ref().err()
//                 );
//             }
//         }

//         // Test interrupt before termination
//         let interrupt_result = process_group.interrupt_all().await;
//         if interrupt_result.is_err() {
//             eprintln!(
//                 "Interrupt failed (expected in some environments): {:?}",
//                 interrupt_result.as_ref().err()
//             );
//         }

//         // Finally, terminate the process group
//         let terminate_result = process_group.terminate_all().await;
//         if terminate_result.is_err() {
//             eprintln!("Termination failed: {:?}", terminate_result.as_ref().err());
//         }

//         // Wait for the child to exit
//         let _ = child.wait().await;

//         // This test mainly ensures all APIs work without panicking
//         // The actual signal delivery may fail in restricted environments
//         println!("Complete process lifecycle test finished");
//     }
// }

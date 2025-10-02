/// Pause a process by process ID (Unix).
///
/// Sends SIGSTOP signal to the specified process to pause its execution.
///
/// # Arguments
///
/// * `pid` - The process ID to pause
///
/// # Returns
///
/// - `Ok(())` if the signal was sent successfully
/// - `Err(std::io::Error)` if pausing failed
///
/// # Example
/// ```rust,no_run
/// use tcrm_task::tasks::process::action::pause::pause_process;
/// let pid = 1234;
/// pause_process(pid).unwrap();
/// ```
#[cfg(unix)]
pub(crate) fn pause_process(pid: u32) -> Result<(), std::io::Error> {
    use nix::errno::Errno;
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    // Check for invalid pid
    if pid == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid PID: 0",
        ));
    }

    // Convert u32 to i32 safely, checking for overflow
    let pid_i32 = match i32::try_from(pid) {
        Ok(p) => p,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("PID {} is too large for this system", pid),
            ));
        }
    };

    match kill(Pid::from_raw(pid_i32), Signal::SIGSTOP) {
        Ok(_) => Ok(()),
        Err(e) => match e {
            Errno::ESRCH => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Process with PID {} does not exist", pid),
            )),
            Errno::EPERM => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Permission denied to pause PID {}", pid),
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to send SIGSTOP to PID {}: {}", pid, e),
            )),
        },
    }
}

/// Pause a process by process ID (Windows).
///
/// Suspends all threads in the specified process using NtSuspendProcess.
///
/// # Arguments
///
/// * `pid` - The process ID to pause
///
/// # Returns
///
/// - `Ok(())` if the process was suspended successfully
/// - `Err(std::io::Error)` if pausing failed
///
/// # Example
/// ```rust,no_run
/// use tcrm_task::tasks::process::action::pause::pause_process;
/// let pid = 1234;
/// pause_process(pid).unwrap();
/// ```
#[cfg(windows)]
pub(crate) fn pause_process(pid: u32) -> Result<(), std::io::Error> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows::Win32::System::Threading::{OpenThread, SuspendThread, THREAD_SUSPEND_RESUME};

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create thread snapshot: {:?}", e),
            )
        })?;

        let mut thread_entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };

        let mut suspended_count = 0;

        if Thread32First(snapshot, &mut thread_entry).is_ok() {
            loop {
                if thread_entry.th32OwnerProcessID == pid {
                    let thread_handle =
                        OpenThread(THREAD_SUSPEND_RESUME, false, thread_entry.th32ThreadID);
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

        if suspended_count == 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No threads found for process with PID {}", pid),
            ))
        } else {
            Ok(())
        }
    }
}

/// Process pausing is not available on this platform.
#[cfg(not(any(unix, windows)))]
pub fn pause_process(_pid: u32) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Process pausing not supported on this platform",
    ))
}

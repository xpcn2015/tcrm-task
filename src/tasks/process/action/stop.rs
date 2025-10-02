/// Terminate a process by process ID (Unix).
///
/// Sends SIGTERM signal to the specified process.
///
/// # Arguments
///
/// * `pid` - The process ID to terminate
///
/// # Returns
///
/// - `Ok(())` if the signal was sent successfully
/// - `Err(std::io::Error)` if termination failed
///
/// # Example
/// ```rust,no_run
/// use tcrm_task::tasks::process::action::terminate::terminate_process;
/// let pid = 1234;
/// terminate_process(pid).unwrap();
/// ```
#[cfg(unix)]
pub fn terminate_process(pid: u32) -> Result<(), std::io::Error> {
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

    match kill(Pid::from_raw(pid_i32), Signal::SIGTERM) {
        Ok(_) => Ok(()),
        Err(e) => match e {
            Errno::ESRCH => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Process with PID {} does not exist", pid),
            )),
            Errno::EPERM => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Permission denied to terminate PID {}", pid),
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to send SIGTERM to PID {}: {}", pid, e),
            )),
        },
    }
}

/// Terminate a process by process ID (Windows).
///
/// Uses TerminateProcess to forcefully terminate the specified process.
///
/// # Arguments
///
/// * `pid` - The process ID to terminate
///
/// # Returns
///
/// - `Ok(())` if the process was terminated successfully
/// - `Err(std::io::Error)` if termination failed
///
/// # Example
/// ```rust,no_run
/// use tcrm_task::tasks::process::action::terminate::terminate_process;
/// let pid = 1234;
/// terminate_process(pid).unwrap();
/// ```
#[cfg(windows)]
pub fn stop_process(pid: u32) -> Result<(), std::io::Error> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

    unsafe {
        let process_handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to open process with PID {}: {:?}", pid, e),
            )
        })?;

        TerminateProcess(process_handle, 1).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to terminate process with PID {}: {:?}", pid, e),
            )
        })?;

        let _ = CloseHandle(process_handle);

        Ok(())
    }
}

/// Process termination is not available on this platform.
#[cfg(not(any(unix, windows)))]
pub fn terminate_process(_pid: u32) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Unsupported platform",
    ))
}

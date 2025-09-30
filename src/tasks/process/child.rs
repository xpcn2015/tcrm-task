/// Terminate a process by process ID
///
/// Attempts to gracefully terminate a process using platform-specific methods.
/// On Unix systems, sends SIGTERM signal. On Windows, uses TerminateProcess.
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
/// # Errors
///
/// Returns an error if:
/// - The process does not exist (no such process)
/// - Permission denied
/// - Invalid process ID
/// - Platform-specific termination failed
///
/// # Examples
///
/// ```rust,no_run
/// use tcrm_task::tasks::process::child::terminate_process;
///
/// let pid = 1234;
/// match terminate_process(pid) {
///     Ok(()) => println!("Process terminated successfully"),
///     Err(e) => eprintln!("Failed to terminate process: {}", e),
/// }
/// ```
pub fn terminate_process(pid: u32) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        use nix::errno::Errno;
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;

        let pid = pid as i32;
        // Check for invalid pid
        if pid <= 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid PID: {}", pid),
            ));
        }

        match kill(Pid::from_raw(pid), Signal::SIGTERM) {
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

    #[cfg(windows)]
    {
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

    #[cfg(not(any(unix, windows)))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unsupported platform",
        ))
    }
}

/// Interrupt a process by process ID (Unix).
///
/// Sends SIGINT signal to the specified process to interrupt its execution.
///
/// # Arguments
///
/// * `pid` - The process ID to interrupt
///
/// # Returns
///
/// - `Ok(())` if the signal was sent successfully
/// - `Err(std::io::Error)` if interrupting failed
///
/// # Example
/// ```rust,no_run
/// use tcrm_task::tasks::process::action::interrupt::interrupt_process;
/// let pid = 1234;
/// interrupt_process(pid).unwrap();
/// ```
#[cfg(unix)]
pub fn interrupt_process(pid: u32) -> Result<(), std::io::Error> {
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

    match kill(Pid::from_raw(pid_i32), Signal::SIGINT) {
        Ok(_) => Ok(()),
        Err(e) => match e {
            Errno::ESRCH => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Process with PID {} does not exist", pid),
            )),
            Errno::EPERM => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Permission denied to interrupt PID {}", pid),
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to send SIGINT to PID {}: {}", pid, e),
            )),
        },
    }
}

/// Interrupt a process by process ID (Windows).
///
/// Sends a CTRL+C signal to the specified process using GenerateConsoleCtrlEvent.
///
/// # Arguments
///
/// * `pid` - The process ID to interrupt
///
/// # Returns
///
/// - `Ok(())` if the interrupt signal was sent successfully
/// - `Err(std::io::Error)` if interrupting failed
///
/// # Example
/// ```rust,no_run
/// use tcrm_task::tasks::process::action::interrupt::interrupt_process;
/// let pid = 1234;
/// interrupt_process(pid).unwrap();
/// ```
#[cfg(windows)]
pub fn interrupt_process(pid: u32) -> Result<(), std::io::Error> {
    use windows::Win32::System::Console::{CTRL_C_EVENT, GenerateConsoleCtrlEvent};

    // Validate PID
    if pid == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid PID: 0",
        ));
    }

    unsafe {
        GenerateConsoleCtrlEvent(CTRL_C_EVENT, pid).map_err(|e| {
            let error_message = match e.code().0 as u32 {
                0x80070005 => format!("Access denied when sending interrupt to PID {}", pid),
                0x80070057 => format!("Invalid parameter when sending interrupt to PID {}", pid),
                _ => format!(
                    "Failed to send interrupt to process with PID {}: {:?}",
                    pid, e
                ),
            };

            std::io::Error::new(std::io::ErrorKind::Other, error_message)
        })?;

        Ok(())
    }
}

/// Process interrupting is not available on this platform.
#[cfg(not(any(unix, windows)))]
pub fn interrupt_process(_pid: u32) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Process interrupting not supported on this platform",
    ))
}

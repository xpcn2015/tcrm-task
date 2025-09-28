pub fn terminate_process(pid: u32) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
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
            Err(nix::Error::Sys(nix::errno::Errno::ESRCH)) => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Process with PID {} does not exist", pid),
            )),
            Err(nix::Error::Sys(nix::errno::Errno::EPERM)) => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Permission denied to terminate PID {}", pid),
            )),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to send SIGTERM to PID {}: {}", pid, e),
            )),
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

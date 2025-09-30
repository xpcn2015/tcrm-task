#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessSignal {
    /// Interrupt signal (Unix: SIGINT, Windows: CTRL_C_EVENT)
    #[cfg(unix)]
    SIGINT,
    #[cfg(windows)]
    CtrlCEvent,

    /// Terminate signal (Unix: SIGTERM, Windows: TerminateProcess)
    #[cfg(unix)]
    SIGTERM,
    #[cfg(windows)]
    TerminateProcess,

    /// Kill signal - cannot be caught or ignored (Unix: SIGKILL, Windows: TerminateProcess with force)
    #[cfg(unix)]
    SIGKILL,
    #[cfg(windows)]
    ForceTerminateProcess,

    /// Continue execution if stopped (Unix: SIGCONT, Windows: ResumeThread)
    #[cfg(unix)]
    SIGCONT,
    #[cfg(windows)]
    ResumeThread,

    /// Stop execution (Unix: SIGSTOP, Windows: SuspendThread)
    #[cfg(unix)]
    SIGSTOP,
    #[cfg(windows)]
    SuspendThread,

    /// Hangup - terminal disconnected (Unix: SIGHUP)
    #[cfg(unix)]
    SIGHUP,

    /// Quit with core dump (Unix: SIGQUIT)
    #[cfg(unix)]
    SIGQUIT,

    /// User-defined signal 1 (Unix: SIGUSR1)
    #[cfg(unix)]
    SIGUSR1,

    /// User-defined signal 2 (Unix: SIGUSR2)
    #[cfg(unix)]
    SIGUSR2,

    /// Broken pipe (Unix: SIGPIPE)
    #[cfg(unix)]
    SIGPIPE,

    /// Alarm/timer signal (Unix: SIGALRM)
    #[cfg(unix)]
    SIGALRM,

    /// Child process status changed (Unix: SIGCHLD)
    #[cfg(unix)]
    SIGCHLD,

    /// Terminal stop signal (Unix: SIGTSTP)
    #[cfg(unix)]
    SIGTSTP,

    /// Terminal input for background process (Unix: SIGTTIN)
    #[cfg(unix)]
    SIGTTIN,

    /// Terminal output for background process (Unix: SIGTTOU)
    #[cfg(unix)]
    SIGTTOU,

    /// Urgent condition on socket (Unix: SIGURG)
    #[cfg(unix)]
    SIGURG,

    /// CPU time limit exceeded (Unix: SIGXCPU)
    #[cfg(unix)]
    SIGXCPU,

    /// File size limit exceeded (Unix: SIGXFSZ)
    #[cfg(unix)]
    SIGXFSZ,

    /// Virtual timer expired (Unix: SIGVTALRM)
    #[cfg(unix)]
    SIGVTALRM,

    /// Profiling timer expired (Unix: SIGPROF)
    #[cfg(unix)]
    SIGPROF,

    /// Window size changed (Unix: SIGWINCH)
    #[cfg(unix)]
    SIGWINCH,

    /// I/O possible on descriptor (Unix: SIGIO)
    #[cfg(unix)]
    SIGIO,

    /// Power failure (Unix: SIGPWR)
    #[cfg(unix)]
    SIGPWR,

    /// Break signal (Windows: CTRL_BREAK_EVENT)
    #[cfg(windows)]
    CtrlBreakEvent,
}
/// Send a signal to a process by process ID (Unix implementation)
///
/// # Arguments
///
/// * `signal` - The signal to send
/// * `process_id` - The process ID to send the signal to
///
/// # Returns
///
/// - `Ok(())` if the signal was sent successfully
/// - `Err(std::io::Error)` if the signal could not be sent
///
/// # Errors
///
/// Returns an error if:
/// - The process does not exist
/// - Permission denied
/// - Invalid signal or process ID
///
/// # Examples
///
/// ```rust,no_run
/// use tcrm_task::tasks::signal::{ProcessSignal, send_signal_to_process_id};
///
/// let pid = 1234;
/// let result = send_signal_to_process_id(ProcessSignal::SIGTERM, pid);
/// match result {
///     Ok(()) => println!("Signal sent successfully"),
///     Err(e) => eprintln!("Failed to send signal: {}", e),
/// }
/// ```
#[cfg(unix)]
pub fn send_signal_to_process_id(signal: ProcessSignal, process_id: u32) -> std::io::Result<()> {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    let sig = match signal {
        ProcessSignal::SIGINT => Signal::SIGINT,
        ProcessSignal::SIGTERM => Signal::SIGTERM,
        ProcessSignal::SIGKILL => Signal::SIGKILL,
        ProcessSignal::SIGCONT => Signal::SIGCONT,
        ProcessSignal::SIGSTOP => Signal::SIGSTOP,
        // Add other mappings as needed
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unsupported signal",
            ));
        }
    };

    kill(Pid::from_raw(process_id as i32), sig)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

/// Send a signal to a process by process ID (Windows implementation)
///
/// # Arguments
///
/// * `signal` - The signal to send (mapped to Windows console events or process termination)
/// * `process_id` - The process ID to send the signal to
///
/// # Returns
///
/// - `Ok(())` if the signal was sent successfully
/// - `Err(std::io::Error)` if the signal could not be sent
///
/// # Errors
///
/// Returns an error if:
/// - The process does not exist
/// - Permission denied
/// - Invalid signal or process ID
/// - Console event generation failed
///
/// # Examples
///
/// ```rust,no_run
/// use tcrm_task::tasks::signal::{ProcessSignal, send_signal_to_process_id};
///
/// let pid = 1234;
/// let result = send_signal_to_process_id(ProcessSignal::SIGTERM, pid);
/// match result {
///     Ok(()) => println!("Signal sent successfully"),
///     Err(e) => eprintln!("Failed to send signal: {}", e),
/// }
/// ```
#[cfg(windows)]
pub fn send_signal_to_process_id(signal: ProcessSignal, process_id: u32) -> std::io::Result<()> {
    use windows::Win32::System::Console::{
        CTRL_BREAK_EVENT, CTRL_C_EVENT, GenerateConsoleCtrlEvent,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

    match signal {
        ProcessSignal::CtrlCEvent => unsafe {
            GenerateConsoleCtrlEvent(CTRL_C_EVENT, process_id)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
        },
        ProcessSignal::CtrlBreakEvent => unsafe {
            GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_id)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
        },
        ProcessSignal::TerminateProcess | ProcessSignal::ForceTerminateProcess => unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, process_id)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;
            TerminateProcess(handle, 1)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
        },
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unsupported signal",
        )),
    }
}

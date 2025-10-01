use thiserror::Error;

/// Error type for process group operations.
#[derive(Error, Debug)]
pub enum ProcessGroupError {
    #[error("Failed to create process group/job: {0}")]
    CreationFailed(String),
    #[error("Failed to assign process to group/job: {0}")]
    AssignmentFailed(String),
    #[error("Failed to send signal to process group: {0}")]
    SignalFailed(String),

    #[cfg(not(any(unix, windows)))]
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
}

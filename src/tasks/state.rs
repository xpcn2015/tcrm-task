#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Initiating,
    Running,
    // Some tasks might be running until user tell it to stop
    Ready,
    Finished,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskTerminateReason {
    Timeout,
    Cleanup,
    DependenciesFinished,
    Custom(String),
}

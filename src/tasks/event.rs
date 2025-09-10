use crate::tasks::{config::StreamSource, error::TaskError, state::TaskTerminateReason};
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum TaskEvent {
    Started {
        task_name: String,
    },
    Output {
        task_name: String,
        line: String,
        src: StreamSource,
    },
    Ready {
        task_name: String,
    },
    Stopped {
        task_name: String,
        exit_code: Option<i32>,
        reason: TaskEventStopReason,
    },
    Error {
        task_name: String,
        error: TaskError,
    },
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TaskEventStopReason {
    Finished,
    Terminated(TaskTerminateReason),
    Error(String),
}

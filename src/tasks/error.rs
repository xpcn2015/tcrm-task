use std::io;
use std::sync::PoisonError;
use std::sync::mpsc::SendError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    #[error("Thread error: {0}")]
    Thread(String),

    #[error("MPSC channel error: {0}")]
    MPSC(String),

    #[error("Process timed out")]
    ProcessTimeout,

    #[error("Failed to kill process")]
    ProcessKillFailed,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Custom error: {0}")]
    Custom(String),
}

impl<T> From<PoisonError<T>> for TaskError {
    fn from(err: PoisonError<T>) -> Self {
        TaskError::Thread(format!("{:?}", err))
    }
}
impl<T> From<SendError<T>> for TaskError {
    fn from(err: SendError<T>) -> Self {
        TaskError::MPSC(format!("{:?}", err))
    }
}

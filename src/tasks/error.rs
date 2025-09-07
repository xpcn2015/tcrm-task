use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    #[error("Handle error: {0}")]
    Handle(String),

    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Custom error: {0}")]
    Custom(String),
}

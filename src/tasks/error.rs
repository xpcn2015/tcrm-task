use thiserror::Error;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Error, Debug, Clone)]
pub enum TaskError {
    #[error("IO error: {0}")]
    IO(String),

    #[error("Handle error: {0}")]
    Handle(String),

    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Custom error: {0}")]
    Custom(String),
}

use crate::{flatbuffers::tcrm_task_generated, tasks::error::TaskError};

#[derive(Debug)]

pub enum ConversionError {
    InvalidStreamSource(i8),
    InvalidTaskShell(i8),
    InvalidTaskState(i8),
    InvalidTaskTerminateReasonType(i8),
    InvalidTaskEventStopReasonType(i8),
    InvalidTaskEventType(i8),
    InvalidTaskErrorType(i8),
    MissingRequiredField(&'static str),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::InvalidStreamSource(val) => {
                write!(f, "Invalid StreamSource value: {}", val)
            }
            ConversionError::InvalidTaskShell(val) => write!(f, "Invalid TaskShell value: {}", val),
            ConversionError::InvalidTaskState(val) => write!(f, "Invalid TaskState value: {}", val),
            ConversionError::InvalidTaskTerminateReasonType(val) => {
                write!(f, "Invalid TaskTerminateReasonType value: {}", val)
            }
            ConversionError::InvalidTaskEventStopReasonType(val) => {
                write!(f, "Invalid TaskEventStopReasonType value: {}", val)
            }
            ConversionError::InvalidTaskEventType(val) => {
                write!(f, "Invalid TaskEventType value: {}", val)
            }
            ConversionError::InvalidTaskErrorType(val) => {
                write!(f, "Invalid TaskErrorType value: {}", val)
            }
            ConversionError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}
impl std::error::Error for ConversionError {}

impl TaskError {
    pub fn from_flatbuffer(
        fb_error: tcrm_task_generated::tcrm::task::TaskError,
    ) -> Result<Self, ConversionError> {
        let kind = fb_error.kind();
        let message = fb_error.message().unwrap_or("").to_string();

        match kind {
            tcrm_task_generated::tcrm::task::TaskErrorType::IO => Ok(TaskError::IO(message)),
            tcrm_task_generated::tcrm::task::TaskErrorType::Handle => {
                Ok(TaskError::Handle(message))
            }
            tcrm_task_generated::tcrm::task::TaskErrorType::Channel => {
                Ok(TaskError::Channel(message))
            }
            tcrm_task_generated::tcrm::task::TaskErrorType::InvalidConfiguration => {
                Ok(TaskError::InvalidConfiguration(message))
            }
            tcrm_task_generated::tcrm::task::TaskErrorType::Custom => {
                Ok(TaskError::Custom(message))
            }
            _ => Err(ConversionError::InvalidTaskErrorType(kind.0)),
        }
    }

    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::TaskError<'a>> {
        let message = match self {
            TaskError::IO(msg) => msg,
            TaskError::Handle(msg) => msg,
            TaskError::Channel(msg) => msg,
            TaskError::InvalidConfiguration(msg) => msg,
            TaskError::Custom(msg) => msg,
        };
        let msg_offset = builder.create_string(message);

        let kind = match self {
            TaskError::IO(_) => tcrm_task_generated::tcrm::task::TaskErrorType::IO,
            TaskError::Handle(_) => tcrm_task_generated::tcrm::task::TaskErrorType::Handle,
            TaskError::Channel(_) => tcrm_task_generated::tcrm::task::TaskErrorType::Channel,
            TaskError::InvalidConfiguration(_) => {
                tcrm_task_generated::tcrm::task::TaskErrorType::InvalidConfiguration
            }
            TaskError::Custom(_) => tcrm_task_generated::tcrm::task::TaskErrorType::Custom,
        };

        tcrm_task_generated::tcrm::task::TaskError::create(
            builder,
            &tcrm_task_generated::tcrm::task::TaskErrorArgs {
                kind,
                message: Some(msg_offset),
            },
        )
    }
}

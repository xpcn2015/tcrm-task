use crate::{
    flatbuffers::{
        conversion::{FromFlatbuffers, ToFlatbuffers},
        tcrm_task_generated,
    },
    tasks::error::TaskError,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum ConversionError {
    InvalidStreamSource(i8),
    InvalidTaskState(i8),
    InvalidProcessState(i8),
    InvalidProcessControlAction(i8),
    InvalidTaskTerminateReasonType(i8),
    InvalidTaskEventStopReasonType(i8),
    InvalidTaskEventType(i8),
    InvalidTaskErrorType(i8),
    MissingRequiredField(&'static str),
    FlatBuffersError(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::InvalidStreamSource(val) => {
                write!(f, "Invalid StreamSource value: {val}")
            }
            ConversionError::InvalidTaskState(val) => write!(f, "Invalid TaskState value: {val}"),
            ConversionError::InvalidProcessState(val) => {
                write!(f, "Invalid ProcessState value: {val}")
            }
            ConversionError::InvalidProcessControlAction(val) => {
                write!(f, "Invalid ProcessControlAction value: {val}")
            }
            ConversionError::InvalidTaskTerminateReasonType(val) => {
                write!(f, "Invalid TaskTerminateReasonType value: {val}")
            }
            ConversionError::InvalidTaskEventStopReasonType(val) => {
                write!(f, "Invalid TaskEventStopReasonType value: {val}")
            }
            ConversionError::InvalidTaskEventType(val) => {
                write!(f, "Invalid TaskEventType value: {val}")
            }
            ConversionError::InvalidTaskErrorType(val) => {
                write!(f, "Invalid TaskErrorType value: {val}")
            }
            ConversionError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {field}")
            }
            ConversionError::FlatBuffersError(msg) => {
                write!(f, "FlatBuffers error: {msg}")
            }
        }
    }
}
impl std::error::Error for ConversionError {}

impl FromFlatbuffers<tcrm_task_generated::tcrm::task::TaskError<'_>> for TaskError {
    fn from_flatbuffers(
        fb_error: tcrm_task_generated::tcrm::task::TaskError<'_>,
    ) -> Result<Self, ConversionError> {
        let kind = fb_error.kind();
        let message = fb_error.message().to_string();

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
            tcrm_task_generated::tcrm::task::TaskErrorType::Control => {
                Ok(TaskError::Control(message))
            }
            _ => Err(ConversionError::InvalidTaskErrorType(kind.0)),
        }
    }
}

impl<'a> ToFlatbuffers<'a> for TaskError {
    type Output = flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::TaskError<'a>>;

    fn to_flatbuffers(&self, builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output {
        let message = match self {
            TaskError::IO(msg)
            | TaskError::Handle(msg)
            | TaskError::Channel(msg)
            | TaskError::InvalidConfiguration(msg)
            | TaskError::Control(msg) => msg,
        };
        let msg_offset = builder.create_string(message);

        let kind = match self {
            TaskError::IO(_) => tcrm_task_generated::tcrm::task::TaskErrorType::IO,
            TaskError::Handle(_) => tcrm_task_generated::tcrm::task::TaskErrorType::Handle,
            TaskError::Channel(_) => tcrm_task_generated::tcrm::task::TaskErrorType::Channel,
            TaskError::InvalidConfiguration(_) => {
                tcrm_task_generated::tcrm::task::TaskErrorType::InvalidConfiguration
            }
            TaskError::Control(_) => tcrm_task_generated::tcrm::task::TaskErrorType::Control,
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

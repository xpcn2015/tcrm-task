use crate::{flatbuffers::tcrm_task_generated, tasks::error::TaskError};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
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
    pub fn from_flatbuffers(
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

    pub fn to_flatbuffers<'a>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::error::TaskError;

    #[test]
    fn test_task_error_roundtrip() {
        let test_cases = vec![
            TaskError::IO("io error message".to_string()),
            TaskError::Handle("handle error message".to_string()),
            TaskError::Channel("channel error message".to_string()),
            TaskError::InvalidConfiguration("invalid config message".to_string()),
            TaskError::Custom("custom error message".to_string()),
        ];

        for original_error in test_cases {
            // Convert to FlatBuffer
            let mut builder = flatbuffers::FlatBufferBuilder::new();
            let fb_error = original_error.to_flatbuffers(&mut builder);
            builder.finish(fb_error, None);

            // Get bytes and create new FlatBuffer instance
            let bytes = builder.finished_data();
            let fb_error =
                flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();

            // Convert back to Rust
            let converted_error = TaskError::from_flatbuffers(fb_error).unwrap();

            // Verify roundtrip
            match (&original_error, &converted_error) {
                (TaskError::IO(orig), TaskError::IO(conv)) => assert_eq!(orig, conv),
                (TaskError::Handle(orig), TaskError::Handle(conv)) => assert_eq!(orig, conv),
                (TaskError::Channel(orig), TaskError::Channel(conv)) => assert_eq!(orig, conv),
                (TaskError::InvalidConfiguration(orig), TaskError::InvalidConfiguration(conv)) => {
                    assert_eq!(orig, conv)
                }
                (TaskError::Custom(orig), TaskError::Custom(conv)) => assert_eq!(orig, conv),
                _ => panic!(
                    "Error type mismatch: {:?} vs {:?}",
                    original_error, converted_error
                ),
            }
        }
    }
        #[test]
        fn test_flatbuffer_direct_read() {
            let error = TaskError::Channel("direct_channel_error".to_string());
            let mut builder = flatbuffers::FlatBufferBuilder::new();
            let fb_error = error.to_flatbuffers(&mut builder);
            builder.finish(fb_error, None);
            let bytes = builder.finished_data();
            let fb = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskError>(bytes).unwrap();
            assert_eq!(fb.kind(), tcrm_task_generated::tcrm::task::TaskErrorType::Channel);
            assert_eq!(fb.message().unwrap(), "direct_channel_error");
        }

    #[test]
    fn test_conversion_error_display() {
        let errors = vec![
            ConversionError::InvalidStreamSource(99),
            ConversionError::InvalidTaskShell(88),
            ConversionError::InvalidTaskState(77),
            ConversionError::InvalidTaskTerminateReasonType(66),
            ConversionError::InvalidTaskEventStopReasonType(55),
            ConversionError::InvalidTaskEventType(44),
            ConversionError::InvalidTaskErrorType(33),
            ConversionError::MissingRequiredField("test_field"),
        ];

        for error in errors {
            let display_str = format!("{}", error);
            assert!(!display_str.is_empty());
            assert!(display_str.contains("Invalid") || display_str.contains("Missing"));
        }
    }
}

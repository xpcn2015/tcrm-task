#[derive(Debug)]

pub enum ConversionError {
    InvalidStreamSource(i8),
    InvalidTaskShell(i8),
    InvalidTaskState(i8),
    InvalidTaskTerminateReasonType(i8),
    InvalidTaskEventStopReasonType(i8),
    InvalidTaskEventType(i8),
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
            ConversionError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}
impl std::error::Error for ConversionError {}

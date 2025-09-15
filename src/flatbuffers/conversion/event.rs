use crate::{
    flatbuffers::{conversion::error::ConversionError, tcrm_task_generated},
    tasks::{
        error::TaskError,
        event::{TaskEvent, TaskEventStopReason},
    },
};

impl TaskEvent {
    /// Converts the task event to `FlatBuffers` representation.
    ///
    /// Returns both the discriminant enum value and the associated union data
    /// required for `FlatBuffers` union serialization.
    ///
    /// # Arguments
    ///
    /// * `builder` - `FlatBuffers` builder for creating the serialized data.
    ///
    /// # Returns
    ///
    /// A tuple containing the event discriminant and the union offset data.
    pub fn to_flatbuffers(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'_>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEvent,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskEvent::Started { task_name } => {
                let task_name_offset = builder.create_string(task_name);
                let ev = tcrm_task_generated::tcrm::task::StartedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StartedEventArgs {
                        task_name: Some(task_name_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEvent::Started,
                    ev.as_union_value(),
                )
            }
            TaskEvent::Output {
                task_name,
                line,
                src,
            } => {
                let task_name_offset = builder.create_string(task_name);
                let line_offset = builder.create_string(line);
                let ev = tcrm_task_generated::tcrm::task::OutputEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::OutputEventArgs {
                        task_name: Some(task_name_offset),
                        line: Some(line_offset),
                        src: src.clone().into(),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEvent::Output,
                    ev.as_union_value(),
                )
            }
            TaskEvent::Ready { task_name } => {
                let task_name_offset = builder.create_string(task_name);
                let ev = tcrm_task_generated::tcrm::task::ReadyEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ReadyEventArgs {
                        task_name: Some(task_name_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEvent::Ready,
                    ev.as_union_value(),
                )
            }
            TaskEvent::Stopped {
                task_name,
                exit_code,
                reason,
            } => {
                let task_name_offset = builder.create_string(task_name);
                let (reason_type, reason) = reason.to_flatbuffers(builder);
                let ev = tcrm_task_generated::tcrm::task::StoppedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StoppedEventArgs {
                        task_name: Some(task_name_offset),
                        exit_code: exit_code.unwrap_or(-1), // Use -1 as sentinel for None
                        reason_type,
                        reason: Some(reason),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEvent::Stopped,
                    ev.as_union_value(),
                )
            }
            TaskEvent::Error { task_name, error } => {
                let task_name_offset = builder.create_string(task_name);
                let error_fb = error.to_flatbuffers(builder);
                let ev = tcrm_task_generated::tcrm::task::ErrorEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorEventArgs {
                        task_name: Some(task_name_offset),
                        error: Some(error_fb),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEvent::Error,
                    ev.as_union_value(),
                )
            }
        }
    }

    /// Converts from `FlatBuffers` representation to `TaskEvent`.
    ///
    /// # Arguments
    ///
    /// * `fb_wrapper` - The `FlatBuffers` TaskEventWrapper containing the event data.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError`] if:
    /// - The event type is not recognized
    /// - Required fields are missing
    /// - Nested conversions fail
    pub fn from_flatbuffers(
        fb_wrapper: tcrm_task_generated::tcrm::task::TaskEventWrapper,
    ) -> Result<Self, ConversionError> {
        let event_type = fb_wrapper.event_type();

        match event_type {
            tcrm_task_generated::tcrm::task::TaskEvent::Started => {
                let started_event = fb_wrapper
                    .event_as_started()
                    .ok_or(ConversionError::MissingRequiredField("started_event"))?;
                Ok(TaskEvent::Started {
                    task_name: started_event.task_name().to_string(),
                })
            }
            tcrm_task_generated::tcrm::task::TaskEvent::Output => {
                let output_event = fb_wrapper
                    .event_as_output()
                    .ok_or(ConversionError::MissingRequiredField("output_event"))?;
                Ok(TaskEvent::Output {
                    task_name: output_event.task_name().to_string(),
                    line: output_event.line().to_string(),
                    src: output_event.src().try_into()?,
                })
            }
            tcrm_task_generated::tcrm::task::TaskEvent::Ready => {
                let ready_event = fb_wrapper
                    .event_as_ready()
                    .ok_or(ConversionError::MissingRequiredField("ready_event"))?;
                Ok(TaskEvent::Ready {
                    task_name: ready_event.task_name().to_string(),
                })
            }
            tcrm_task_generated::tcrm::task::TaskEvent::Stopped => {
                let stopped_event = fb_wrapper
                    .event_as_stopped()
                    .ok_or(ConversionError::MissingRequiredField("stopped_event"))?;
                let reason_type = stopped_event.reason_type();
                let reason = TaskEventStopReason::from_flatbuffers_with_type(
                    reason_type,
                    stopped_event.reason(),
                )?;
                Ok(TaskEvent::Stopped {
                    task_name: stopped_event.task_name().to_string(),
                    exit_code: if stopped_event.exit_code() == -1 {
                        None // -1 is sentinel value for no exit code
                    } else {
                        Some(stopped_event.exit_code())
                    },
                    reason,
                })
            }
            tcrm_task_generated::tcrm::task::TaskEvent::Error => {
                let error_event = fb_wrapper
                    .event_as_error()
                    .ok_or(ConversionError::MissingRequiredField("error_event"))?;
                let error = TaskError::from_flatbuffers(error_event.error())?;
                Ok(TaskEvent::Error {
                    task_name: error_event.task_name().to_string(),
                    error,
                })
            }
            _ => Err(ConversionError::InvalidTaskEventType(event_type.0 as i8)),
        }
    }
}

impl TaskEventStopReason {
    /// Converts the stop reason to `FlatBuffers` representation.
    ///
    /// Returns both the discriminant enum value and the associated union data
    /// required for `FlatBuffers` union serialization.
    ///
    /// # Arguments
    ///
    /// * `builder` - `FlatBuffers` builder for creating the serialized data.
    ///
    /// # Returns
    ///
    /// A tuple containing the stop reason discriminant and the union offset data.
    pub fn to_flatbuffers(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'_>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskEventStopReason::Finished => {
                let r = tcrm_task_generated::tcrm::task::FinishedReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::FinishedReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::Finished,
                    r.as_union_value(),
                )
            }
            TaskEventStopReason::Terminated(reason) => reason.to_flatbuffers_terminated(builder),
            TaskEventStopReason::Error(msg) => {
                let msg_offset = builder.create_string(msg);
                let r = tcrm_task_generated::tcrm::task::ErrorStopReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorStopReasonArgs {
                        message: Some(msg_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::Error,
                    r.as_union_value(),
                )
            }
        }
    }

    /// Converts from `FlatBuffers` representation to `TaskEventStopReason`.
    ///
    /// # Arguments
    ///
    /// * `reason_type` - The discriminant enum value for the stop reason type.
    /// * `reason_data` - The associated union data.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError`] if:
    /// - The reason type is not recognized
    /// - Required fields are missing
    /// - Nested conversions fail
    pub fn from_flatbuffers_with_type(
        reason_type: tcrm_task_generated::tcrm::task::TaskEventStopReason,
        reason_data: flatbuffers::Table,
    ) -> Result<Self, ConversionError> {
        match reason_type {
            tcrm_task_generated::tcrm::task::TaskEventStopReason::Finished => {
                Ok(TaskEventStopReason::Finished)
            }
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout => {
                use crate::tasks::state::TaskTerminateReason;
                Ok(TaskEventStopReason::Terminated(
                    TaskTerminateReason::Timeout,
                ))
            }
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup => {
                use crate::tasks::state::TaskTerminateReason;
                Ok(TaskEventStopReason::Terminated(
                    TaskTerminateReason::Cleanup,
                ))
            }
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished => {
                use crate::tasks::state::TaskTerminateReason;
                Ok(TaskEventStopReason::Terminated(
                    TaskTerminateReason::DependenciesFinished,
                ))
            }
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCustom => {
                use crate::tasks::state::TaskTerminateReason;
                let custom_reason = unsafe {
                    tcrm_task_generated::tcrm::task::CustomReason::init_from_table(reason_data)
                };
                Ok(TaskEventStopReason::Terminated(
                    TaskTerminateReason::Custom(custom_reason.message().to_string()),
                ))
            }
            tcrm_task_generated::tcrm::task::TaskEventStopReason::Error => {
                let error_reason = unsafe {
                    tcrm_task_generated::tcrm::task::ErrorStopReason::init_from_table(reason_data)
                };
                Ok(TaskEventStopReason::Error(error_reason.message().to_string()))
            }
            _ => Err(ConversionError::InvalidTaskEventStopReasonType(reason_type.0 as i8)),
        }
    }
}

/// High-level wrapper conversions for TaskEventWrapper root type.
///
/// TaskEventWrapper is the root type in the FlatBuffers schema and provides
/// convenient methods for complete serialization/deserialization.
impl TaskEvent {
    /// Converts a TaskEvent to a complete FlatBuffers byte buffer.
    ///
    /// This creates a complete, self-contained FlatBuffers buffer that can be
    /// stored or transmitted and later deserialized with `from_flatbuffers_bytes`.
    ///
    /// # Arguments
    ///
    /// * `builder` - Optional FlatBuffers builder to reuse. If None, creates a new one.
    ///
    /// # Returns
    ///
    /// A byte vector containing the complete FlatBuffers data.
    pub fn to_flatbuffers_bytes(&self) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (event_type, event_offset) = self.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type,
                event: Some(event_offset),
            },
        );

        builder.finish(wrapper, None);
        builder.finished_data().to_vec()
    }

    /// Converts a FlatBuffers byte buffer to a TaskEvent.
    ///
    /// This is a convenience method that handles the complete deserialization
    /// process from a byte buffer created with `to_flatbuffers_bytes`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The FlatBuffers byte buffer containing TaskEventWrapper data.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError`] if:
    /// - The byte buffer is invalid or corrupted
    /// - The FlatBuffers data doesn't contain a valid TaskEventWrapper
    /// - Any nested conversion errors occur
    pub fn from_flatbuffers_bytes(bytes: &[u8]) -> Result<Self, ConversionError> {
        let wrapper = flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes)
            .map_err(|e| ConversionError::FlatBuffersError(e.to_string()))?;
        Self::from_flatbuffers(wrapper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{
        config::StreamSource,
        error::TaskError,
        event::{TaskEvent, TaskEventStopReason},
        state::TaskTerminateReason,
    };

    #[test]
    fn test_task_event_started() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let event = TaskEvent::Started {
            task_name: task_name.to_string(),
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Started
        );
    }

    #[test]
    fn test_task_event_output() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let line = "output line";
        let src = StreamSource::Stdout;
        let event = TaskEvent::Output {
            task_name: task_name.to_string(),
            line: line.to_string(),
            src,
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Output
        );
    }

    #[test]
    fn test_task_event_ready() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let event = TaskEvent::Ready {
            task_name: task_name.to_string(),
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Ready
        );
    }

    #[test]
    fn test_task_event_stopped() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let exit_code = Some(0);
        let reason = TaskEventStopReason::Finished;
        let event = TaskEvent::Stopped {
            task_name: task_name.to_string(),
            exit_code,
            reason,
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Stopped
        );
    }

    #[test]
    fn test_task_event_error() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let error = TaskError::Custom("Test error".to_string());
        let event = TaskEvent::Error {
            task_name: task_name.to_string(),
            error,
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Error
        );
    }

    #[test]
    fn test_task_event_stop_reason_finished() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskEventStopReason::Finished;

        let (fb_reason_type, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason_type,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::Finished
        );
    }

    #[test]
    fn test_task_event_stop_reason_terminated() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let terminate_reason = TaskTerminateReason::Timeout;
        let reason = TaskEventStopReason::Terminated(terminate_reason);

        let (fb_reason_type, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason_type,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout
        );
    }

    #[test]
    fn test_task_event_stop_reason_error() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let error_msg = "Task failed";
        let reason = TaskEventStopReason::Error(error_msg.to_string());

        let (fb_reason_type, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason_type,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::Error
        );
    }

    #[test]
    fn test_task_event_output_stderr() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let line = "error output";
        let src = StreamSource::Stderr;
        let event = TaskEvent::Output {
            task_name: task_name.to_string(),
            line: line.to_string(),
            src,
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Output
        );
    }
    #[test]
    fn test_flatbuffer_direct_read_started_event() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let event = TaskEvent::Started {
            task_name: "direct_event_task".to_string(),
        };
        let (fb_event_type, offset) = event.to_flatbuffers(&mut builder);
        builder.finish(offset, None);
        let bytes = builder.finished_data();
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Started
        );
        let started_event =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::StartedEvent>(bytes).unwrap();
        assert_eq!(started_event.task_name(), "direct_event_task");
    }
    #[test]
    fn test_task_event_stopped_with_exit_code() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let exit_code = Some(1);
        let reason = TaskEventStopReason::Error("Process failed".to_string());
        let event = TaskEvent::Stopped {
            task_name: task_name.to_string(),
            exit_code,
            reason,
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Stopped
        );
    }

    #[test]
    fn test_task_event_stopped_no_exit_code() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let task_name = "test_task";
        let exit_code = None;
        let reason = TaskEventStopReason::Terminated(TaskTerminateReason::Cleanup);
        let event = TaskEvent::Stopped {
            task_name: task_name.to_string(),
            exit_code,
            reason,
        };

        let (fb_event_type, _offset) = event.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_event_type,
            tcrm_task_generated::tcrm::task::TaskEvent::Stopped
        );
    }

    // Roundtrip tests for from_flatbuffers functionality
    #[test]
    fn test_task_event_started_roundtrip() {
        let original_event = TaskEvent::Started {
            task_name: "roundtrip_task".to_string(),
        };

        // Create FlatBuffer with complete wrapper
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_ready_roundtrip() {
        let original_event = TaskEvent::Ready {
            task_name: "ready_task".to_string(),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_output_stdout_roundtrip() {
        let original_event = TaskEvent::Output {
            task_name: "output_task".to_string(),
            line: "Hello World".to_string(),
            src: StreamSource::Stdout,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_output_stderr_roundtrip() {
        let original_event = TaskEvent::Output {
            task_name: "error_output_task".to_string(),
            line: "Error message".to_string(),
            src: StreamSource::Stderr,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_error_roundtrip() {
        let original_event = TaskEvent::Error {
            task_name: "error_task".to_string(),
            error: TaskError::Custom("Custom error message".to_string()),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_stopped_finished_roundtrip() {
        let original_event = TaskEvent::Stopped {
            task_name: "finished_task".to_string(),
            exit_code: Some(0),
            reason: TaskEventStopReason::Finished,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_stopped_terminated_timeout_roundtrip() {
        let original_event = TaskEvent::Stopped {
            task_name: "timeout_task".to_string(),
            exit_code: None,
            reason: TaskEventStopReason::Terminated(TaskTerminateReason::Timeout),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_stopped_terminated_custom_roundtrip() {
        let original_event = TaskEvent::Stopped {
            task_name: "custom_task".to_string(),
            exit_code: Some(255),
            reason: TaskEventStopReason::Terminated(TaskTerminateReason::Custom(
                "User requested termination".to_string(),
            )),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_stopped_error_reason_roundtrip() {
        let original_event = TaskEvent::Stopped {
            task_name: "error_stopped_task".to_string(),
            exit_code: Some(1),
            reason: TaskEventStopReason::Error("Internal process error".to_string()),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_event_type, event_offset) = original_event.to_flatbuffers(&mut builder);

        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: fb_event_type,
                event: Some(event_offset),
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let converted_event = TaskEvent::from_flatbuffers(fb_wrapper).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_from_flatbuffers_invalid_event_type() {
        // Create a wrapper with invalid event type
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let wrapper = tcrm_task_generated::tcrm::task::TaskEventWrapper::create(
            &mut builder,
            &tcrm_task_generated::tcrm::task::TaskEventWrapperArgs {
                event_type: tcrm_task_generated::tcrm::task::TaskEvent::NONE,
                event: None,
            },
        );
        builder.finish(wrapper, None);
        let bytes = builder.finished_data();

        let fb_wrapper =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskEventWrapper>(bytes).unwrap();
        let result = TaskEvent::from_flatbuffers(fb_wrapper);

        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::InvalidTaskEventType(_) => {}
            _ => panic!("Expected InvalidTaskEventType error"),
        }
    }

    #[test]
    fn test_task_event_stop_reason_from_flatbuffers_invalid_type() {
        // Test invalid stop reason type using a dummy table
        use flatbuffers::Table;

        let empty_table = unsafe { Table::new(&[0u8; 16], 0) };
        let result = TaskEventStopReason::from_flatbuffers_with_type(
            tcrm_task_generated::tcrm::task::TaskEventStopReason::NONE,
            empty_table,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::InvalidTaskEventStopReasonType(_) => {}
            _ => panic!("Expected InvalidTaskEventStopReasonType error"),
        }
    }

    #[test]
    fn test_task_event_wrapper_convenience_bytes_roundtrip() {
        let original_event = TaskEvent::Started {
            task_name: "wrapper_convenience_task".to_string(),
        };

        // Test bytes roundtrip
        let bytes = original_event.to_flatbuffers_bytes();
        let converted_event = TaskEvent::from_flatbuffers_bytes(&bytes).unwrap();

        assert_eq!(original_event, converted_event);
    }

    #[test]
    fn test_task_event_wrapper_convenience_bytes_all_variants() {
        let test_events = vec![
            TaskEvent::Started {
                task_name: "test_started".to_string(),
            },
            TaskEvent::Output {
                task_name: "test_output".to_string(),
                line: "test line".to_string(),
                src: StreamSource::Stdout,
            },
            TaskEvent::Ready {
                task_name: "test_ready".to_string(),
            },
            TaskEvent::Stopped {
                task_name: "test_stopped".to_string(),
                exit_code: Some(0),
                reason: TaskEventStopReason::Finished,
            },
            TaskEvent::Error {
                task_name: "test_error".to_string(),
                error: TaskError::Custom("test error".to_string()),
            },
        ];

        for original_event in test_events {
            let bytes = original_event.to_flatbuffers_bytes();
            let converted_event = TaskEvent::from_flatbuffers_bytes(&bytes).unwrap();
            assert_eq!(original_event, converted_event);
        }
    }

    #[test]
    fn test_task_event_wrapper_invalid_bytes() {
        let invalid_bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let result = TaskEvent::from_flatbuffers_bytes(&invalid_bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::FlatBuffersError(_) => {}
            _ => panic!("Expected FlatBuffersError"),
        }
    }
}

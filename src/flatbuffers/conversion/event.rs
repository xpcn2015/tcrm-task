use crate::tasks::event::TaskTerminateReason;

use crate::flatbuffers::conversion::ConversionError;
use crate::flatbuffers::conversion::FromFlatbuffers;
use crate::tasks::error::TaskError;
use crate::{
    flatbuffers::{
        conversion::{ToFlatbuffers, ToFlatbuffersUnion},
        tcrm_task_generated,
    },
    tasks::event::{TaskEvent, TaskEventStopReason},
};

impl<'a>
    FromFlatbuffers<(
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::Table<'a>,
    )> for TaskEventStopReason
{
    fn from_flatbuffers(
        input: (
            tcrm_task_generated::tcrm::task::TaskEventStopReason,
            flatbuffers::Table<'a>,
        ),
    ) -> Result<Self, ConversionError> {
        let disc = input.0.0; // .0 to get the u8 discriminant
        match disc {
            0 => Ok(TaskEventStopReason::Finished),
            1 => {
                // Error
                let error_reason = unsafe {
                    tcrm_task_generated::tcrm::task::ErrorStopReason::init_from_table(input.1)
                };
                let msg = error_reason.message().to_string();
                Ok(TaskEventStopReason::Error(msg))
            }
            2 => Ok(TaskEventStopReason::Terminated(
                TaskTerminateReason::Timeout,
            )),
            3 => Ok(TaskEventStopReason::Terminated(
                TaskTerminateReason::Cleanup,
            )),
            4 => Ok(TaskEventStopReason::Terminated(
                TaskTerminateReason::DependenciesFinished,
            )),
            5 => Ok(TaskEventStopReason::Terminated(
                TaskTerminateReason::UserRequested,
            )),
            _ => Err(ConversionError::InvalidTaskEventStopReasonType(disc as i8)),
        }
    }
}
impl<'a> FromFlatbuffers<tcrm_task_generated::tcrm::task::TaskEvent<'a>> for TaskEvent {
    fn from_flatbuffers(
        fb_event: tcrm_task_generated::tcrm::task::TaskEvent<'a>,
    ) -> Result<Self, ConversionError> {
        use tcrm_task_generated::tcrm::task::TaskEventUnion;
        match fb_event.event_type() {
            TaskEventUnion::Started => {
                let started = fb_event
                    .event_as_started()
                    .ok_or(ConversionError::MissingRequiredField("StartedEvent"))?;
                let task_name = started.task_name().to_string();
                Ok(TaskEvent::Started { task_name })
            }
            TaskEventUnion::Output => {
                let output = fb_event
                    .event_as_output()
                    .ok_or(ConversionError::MissingRequiredField("OutputEvent"))?;
                let task_name = output.task_name().to_string();
                let line = output.line().to_string();
                let src = output
                    .src()
                    .try_into()
                    .map_err(|_| ConversionError::InvalidStreamSource(output.src().0))?;
                Ok(TaskEvent::Output {
                    task_name,
                    line,
                    src,
                })
            }
            TaskEventUnion::Ready => {
                let ready = fb_event
                    .event_as_ready()
                    .ok_or(ConversionError::MissingRequiredField("ReadyEvent"))?;
                let task_name = ready.task_name().to_string();
                Ok(TaskEvent::Ready { task_name })
            }
            TaskEventUnion::Stopped => {
                let stopped = fb_event
                    .event_as_stopped()
                    .ok_or(ConversionError::MissingRequiredField("StoppedEvent"))?;
                let task_name = stopped.task_name().to_string();
                let exit_code = Some(stopped.exit_code());
                let fb_reason_type = stopped.reason_type();
                let fb_reason_table = stopped.reason();
                let reason =
                    TaskEventStopReason::from_flatbuffers((fb_reason_type, fb_reason_table))?;
                Ok(TaskEvent::Stopped {
                    task_name,
                    exit_code,
                    reason,
                })
            }
            TaskEventUnion::Error => {
                let error_event = fb_event
                    .event_as_error()
                    .ok_or(ConversionError::MissingRequiredField("ErrorEvent"))?;
                let task_name = error_event.task_name().to_string();
                let fb_error = error_event.error();
                let error = TaskError::from_flatbuffers(fb_error)?;
                Ok(TaskEvent::Error { task_name, error })
            }
            other => Err(ConversionError::InvalidTaskEventType(other.0 as i8)),
        }
    }
}

impl<'a> ToFlatbuffers<'a> for TaskEvent {
    type Output = flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::TaskEvent<'a>>;

    fn to_flatbuffers(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> <Self as ToFlatbuffers<'a>>::Output {
        let (event_type, event_offset) = match self {
            TaskEvent::Started { task_name } => {
                let name_offset = builder.create_string(task_name);
                let started = tcrm_task_generated::tcrm::task::StartedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StartedEventArgs {
                        task_name: Some(name_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Started,
                    started.as_union_value(),
                )
            }
            TaskEvent::Output {
                task_name,
                line,
                src,
            } => {
                let name_offset = builder.create_string(task_name);
                let line_offset = builder.create_string(line);
                let fb_src: tcrm_task_generated::tcrm::task::StreamSource = src.clone().into();
                let output = tcrm_task_generated::tcrm::task::OutputEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::OutputEventArgs {
                        task_name: Some(name_offset),
                        line: Some(line_offset),
                        src: fb_src,
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Output,
                    output.as_union_value(),
                )
            }
            TaskEvent::Ready { task_name } => {
                let name_offset = builder.create_string(task_name);
                let ready = tcrm_task_generated::tcrm::task::ReadyEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ReadyEventArgs {
                        task_name: Some(name_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Ready,
                    ready.as_union_value(),
                )
            }
            TaskEvent::Stopped {
                task_name,
                exit_code,
                reason,
            } => {
                let name_offset = builder.create_string(task_name);
                let (_, stop_reason_offset) = reason.to_flatbuffers_union(builder);
                let stopped = tcrm_task_generated::tcrm::task::StoppedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StoppedEventArgs {
                        task_name: Some(name_offset),
                        exit_code: exit_code.unwrap_or(0),
                        reason_type: tcrm_task_generated::tcrm::task::TaskEventStopReason::Finished,
                        reason: Some(stop_reason_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Stopped,
                    stopped.as_union_value(),
                )
            }
            TaskEvent::Error { task_name, error } => {
                let name_offset = builder.create_string(task_name);
                let error_offset = error.to_flatbuffers(builder);
                let error_event = tcrm_task_generated::tcrm::task::ErrorEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorEventArgs {
                        task_name: Some(name_offset),
                        error: Some(error_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Error,
                    error_event.as_union_value(),
                )
            }
        };

        tcrm_task_generated::tcrm::task::TaskEvent::create(
            builder,
            &tcrm_task_generated::tcrm::task::TaskEventArgs {
                event: Some(event_offset),
                event_type,
            },
        )
    }
}

impl<'a> ToFlatbuffersUnion<'a, tcrm_task_generated::tcrm::task::TaskEventStopReason>
    for TaskEventStopReason
{
    fn to_flatbuffers_union(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskEventStopReason::Finished => {
                let dummy = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::Finished,
                    dummy.as_union_value(),
                )
            }
            TaskEventStopReason::Terminated(reason) => {
                let (discriminant, offset) = reason.to_flatbuffers_union(builder);
                (discriminant, offset)
            }
            TaskEventStopReason::Error(message) => {
                let msg_offset = builder.create_string(message);
                let error_reason = tcrm_task_generated::tcrm::task::ErrorStopReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorStopReasonArgs {
                        message: Some(msg_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::Error,
                    error_reason.as_union_value(),
                )
            }
        }
    }
}

use std::time::{SystemTime, UNIX_EPOCH};

use crate::tasks::event::TaskTerminateReason;

use crate::flatbuffers::conversion::ConversionError;
use crate::flatbuffers::conversion::FromFlatbuffers;
use crate::tasks::error::TaskError;
use crate::{
    flatbuffers::{
        conversion::{ToFlatbuffers, ToFlatbuffersUnion},
        tcrm_task_generated,
    },
    tasks::config::StreamSource,
    tasks::event::{TaskEvent, TaskStopReason},
};

#[cfg(feature = "process-control")]
use crate::tasks::process::control::ProcessControlAction;

// SystemTime helper conversions
impl<'a> ToFlatbuffers<'a> for SystemTime {
    type Output = flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::SystemTime<'a>>;

    fn to_flatbuffers(&self, builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output {
        let nanos = self
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        tcrm_task_generated::tcrm::task::SystemTime::create(
            builder,
            &tcrm_task_generated::tcrm::task::SystemTimeArgs {
                nanos_since_epoch: nanos,
            },
        )
    }
}

impl FromFlatbuffers<tcrm_task_generated::tcrm::task::SystemTime<'_>> for SystemTime {
    fn from_flatbuffers(
        fb_time: tcrm_task_generated::tcrm::task::SystemTime<'_>,
    ) -> Result<Self, ConversionError> {
        let nanos = fb_time.nanos_since_epoch();
        let duration = std::time::Duration::from_nanos(nanos);
        Ok(UNIX_EPOCH + duration)
    }
}

impl<'a>
    FromFlatbuffers<(
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::Table<'a>,
    )> for TaskStopReason
{
    fn from_flatbuffers(
        input: (
            tcrm_task_generated::tcrm::task::TaskEventStopReason,
            flatbuffers::Table<'a>,
        ),
    ) -> Result<Self, ConversionError> {
        let disc = input.0.0; // .0 to get the u8 discriminant
        match disc {
            1 => Ok(TaskStopReason::Finished), // TaskEventStopReason::Finished
            2 => Ok(TaskStopReason::Terminated(TaskTerminateReason::Timeout)), // TerminatedTimeout
            3 => Ok(TaskStopReason::Terminated(TaskTerminateReason::Cleanup)), // TerminatedCleanup
            4 => Ok(TaskStopReason::Terminated(
                TaskTerminateReason::DependenciesFinished,
            )), // TerminatedDependenciesFinished
            5 => Ok(TaskStopReason::Terminated(
                TaskTerminateReason::UserRequested,
            )), // TerminatedUserRequested
            6 => Ok(TaskStopReason::Terminated(
                TaskTerminateReason::InternalError,
            )), // TerminatedInternalError
            7 => {
                // Error
                let error_reason = unsafe {
                    tcrm_task_generated::tcrm::task::ErrorStopReason::init_from_table(input.1)
                };
                let msg = error_reason.message().to_string();
                Ok(TaskStopReason::Error(TaskError::IO(msg)))
            }
            _ => Err(ConversionError::InvalidTaskEventStopReasonType(disc as i8)),
        }
    }
}

impl<'a> FromFlatbuffers<tcrm_task_generated::tcrm::task::TaskEvent<'a>> for TaskEvent {
    fn from_flatbuffers(
        fb_event: tcrm_task_generated::tcrm::task::TaskEvent<'a>,
    ) -> Result<Self, ConversionError> {
        let event_type = fb_event.event_type();
        let event_table = fb_event.event();

        match event_type {
            tcrm_task_generated::tcrm::task::TaskEventUnion::Started => {
                let started = unsafe {
                    tcrm_task_generated::tcrm::task::StartedEvent::init_from_table(event_table)
                };
                let process_id = started.process_id();
                let created_at = if let Some(fb_time) = started.created_at() {
                    SystemTime::from_flatbuffers(fb_time)?
                } else {
                    return Err(ConversionError::MissingRequiredField("created_at"));
                };
                let running_at = if let Some(fb_time) = started.running_at() {
                    SystemTime::from_flatbuffers(fb_time)?
                } else {
                    return Err(ConversionError::MissingRequiredField("running_at"));
                };
                Ok(TaskEvent::Started {
                    process_id,
                    created_at,
                    running_at,
                })
            }
            tcrm_task_generated::tcrm::task::TaskEventUnion::Output => {
                let output = unsafe {
                    tcrm_task_generated::tcrm::task::OutputEvent::init_from_table(event_table)
                };
                let line = output.line().to_string();
                let src = StreamSource::try_from(output.src())?;
                Ok(TaskEvent::Output { line, src })
            }
            tcrm_task_generated::tcrm::task::TaskEventUnion::Ready => Ok(TaskEvent::Ready),
            tcrm_task_generated::tcrm::task::TaskEventUnion::Stopped => {
                let stopped = unsafe {
                    tcrm_task_generated::tcrm::task::StoppedEvent::init_from_table(event_table)
                };
                let exit_code = if stopped.exit_code() == i32::MIN {
                    // Using i32::MIN as a sentinel value for None
                    None
                } else {
                    Some(stopped.exit_code())
                };

                let fb_reason_type = stopped.reason_type();
                let fb_reason_table = stopped.reason();
                let reason = TaskStopReason::from_flatbuffers((fb_reason_type, fb_reason_table))?;

                let finished_at = if let Some(fb_time) = stopped.finished_at() {
                    SystemTime::from_flatbuffers(fb_time)?
                } else {
                    return Err(ConversionError::MissingRequiredField("finished_at"));
                };

                #[cfg(unix)]
                let signal = if stopped.signal() == 0 {
                    None
                } else {
                    Some(stopped.signal())
                };

                Ok(TaskEvent::Stopped {
                    exit_code,
                    reason,
                    finished_at,
                    #[cfg(unix)]
                    signal,
                })
            }
            tcrm_task_generated::tcrm::task::TaskEventUnion::Error => {
                let error_event = unsafe {
                    tcrm_task_generated::tcrm::task::ErrorEvent::init_from_table(event_table)
                };
                let error = TaskError::from_flatbuffers(error_event.error())?;
                Ok(TaskEvent::Error { error })
            }
            #[cfg(feature = "process-control")]
            tcrm_task_generated::tcrm::task::TaskEventUnion::ProcessControl => {
                let process_control = unsafe {
                    tcrm_task_generated::tcrm::task::ProcessControlEvent::init_from_table(
                        event_table,
                    )
                };
                let action = ProcessControlAction::try_from(process_control.action())?;
                Ok(TaskEvent::ProcessControl { action })
            }
            _ => Err(ConversionError::InvalidTaskEventType(event_type.0 as i8)),
        }
    }
}

impl<'a> ToFlatbuffersUnion<'a, tcrm_task_generated::tcrm::task::TaskEventUnion> for TaskEvent {
    fn to_flatbuffers_union(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEventUnion,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                let created_at_offset = created_at.to_flatbuffers(builder);
                let running_at_offset = running_at.to_flatbuffers(builder);

                let started = tcrm_task_generated::tcrm::task::StartedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StartedEventArgs {
                        process_id: *process_id,
                        created_at: Some(created_at_offset),
                        running_at: Some(running_at_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Started,
                    started.as_union_value(),
                )
            }
            TaskEvent::Output { line, src } => {
                let line_offset = builder.create_string(line);
                let output = tcrm_task_generated::tcrm::task::OutputEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::OutputEventArgs {
                        line: Some(line_offset),
                        src: src.clone().into(),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Output,
                    output.as_union_value(),
                )
            }
            TaskEvent::Ready => {
                let ready = tcrm_task_generated::tcrm::task::ReadyEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ReadyEventArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Ready,
                    ready.as_union_value(),
                )
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                #[cfg(unix)]
                signal,
            } => {
                let finished_at_offset = finished_at.to_flatbuffers(builder);
                let (reason_type, reason_offset) = reason.to_flatbuffers_union(builder);

                let stopped = tcrm_task_generated::tcrm::task::StoppedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StoppedEventArgs {
                        exit_code: exit_code.unwrap_or(i32::MIN),
                        reason_type,
                        reason: Some(reason_offset),
                        finished_at: Some(finished_at_offset),
                        #[cfg(unix)]
                        signal: signal.unwrap_or(0),
                        #[cfg(not(unix))]
                        signal: 0,
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Stopped,
                    stopped.as_union_value(),
                )
            }
            TaskEvent::Error { error } => {
                let error_offset = error.to_flatbuffers(builder);
                let error_event = tcrm_task_generated::tcrm::task::ErrorEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorEventArgs {
                        error: Some(error_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::Error,
                    error_event.as_union_value(),
                )
            }
            #[cfg(feature = "process-control")]
            TaskEvent::ProcessControl { action } => {
                let process_control = tcrm_task_generated::tcrm::task::ProcessControlEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ProcessControlEventArgs {
                        action: (*action).into(),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventUnion::ProcessControl,
                    process_control.as_union_value(),
                )
            }
        }
    }
}

impl<'a> ToFlatbuffers<'a> for TaskEvent {
    type Output = flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::TaskEvent<'a>>;

    fn to_flatbuffers(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> <Self as ToFlatbuffers<'a>>::Output {
        let (event_type, event_offset) = self.to_flatbuffers_union(builder);
        tcrm_task_generated::tcrm::task::TaskEvent::create(
            builder,
            &tcrm_task_generated::tcrm::task::TaskEventArgs {
                event_type,
                event: Some(event_offset),
            },
        )
    }
}

impl<'a> ToFlatbuffersUnion<'a, tcrm_task_generated::tcrm::task::TaskEventStopReason>
    for TaskStopReason
{
    fn to_flatbuffers_union(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskStopReason::Finished => {
                let r = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::Finished,
                    r.as_union_value(),
                )
            }
            TaskStopReason::Terminated(reason) => {
                let r = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                let discriminant = match reason {
                    TaskTerminateReason::Timeout => tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout,
                    TaskTerminateReason::Cleanup => tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup,
                    TaskTerminateReason::DependenciesFinished => tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished,
                    TaskTerminateReason::UserRequested => tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedUserRequested,
                    TaskTerminateReason::InternalError => tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedInternalError,
                };
                (discriminant, r.as_union_value())
            }
            TaskStopReason::Error(error) => {
                let message_offset = builder.create_string(&error.to_string());
                let error_reason = tcrm_task_generated::tcrm::task::ErrorStopReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorStopReasonArgs {
                        message: Some(message_offset),
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

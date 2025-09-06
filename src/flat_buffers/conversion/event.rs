use crate::{
    flat_buffers::tcrm_task_generated,
    tasks::event::{TaskEvent, TaskEventStopReason},
};

impl TaskEvent {
    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
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
                let (reason_type, reason) = reason.to_flatbuffer(builder);
                let ev = tcrm_task_generated::tcrm::task::StoppedEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::StoppedEventArgs {
                        task_name: Some(task_name_offset),
                        exit_code: exit_code.unwrap_or(0),
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
                let error_offset = builder.create_string(error);
                let ev = tcrm_task_generated::tcrm::task::ErrorEvent::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::ErrorEventArgs {
                        task_name: Some(task_name_offset),
                        error: Some(error_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEvent::Error,
                    ev.as_union_value(),
                )
            }
        }
    }
}

impl TaskEventStopReason {
    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
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
            TaskEventStopReason::Terminated(reason) => reason.to_flatbuffer_terminated(builder),
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
}

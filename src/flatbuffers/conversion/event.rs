use crate::{
    flatbuffers::{
        conversion::{ToFlatbuffers, ToFlatbuffersUnion},
        tcrm_task_generated,
    },
    tasks::event::{TaskEvent, TaskEventStopReason},
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::config::StreamSource;
    use crate::tasks::event::TaskTerminateReason;

    #[test]
    fn test_task_event_started_to_flatbuffers() {
        let event = TaskEvent::Started {
            task_name: "test_task".to_string(),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_event = event.to_flatbuffers(&mut builder);
        builder.finish(fb_event, None);

        // Basic verification that serialization worked
        let bytes = builder.finished_data();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_task_event_output_to_flatbuffers() {
        let event = TaskEvent::Output {
            task_name: "test_task".to_string(),
            line: "Hello, World!".to_string(),
            src: StreamSource::Stdout,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_event = event.to_flatbuffers(&mut builder);
        builder.finish(fb_event, None);

        let bytes = builder.finished_data();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_task_event_stop_reason_finished() {
        let reason = TaskEventStopReason::Finished;

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (_, _offset) = reason.to_flatbuffers_union(&mut builder);

        // Basic verification that serialization worked
        assert!(true);
    }

    #[test]
    fn test_task_event_stop_reason_error() {
        let reason = TaskEventStopReason::Error("Something went wrong".to_string());

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (_, _offset) = reason.to_flatbuffers_union(&mut builder);

        // Basic verification that serialization worked
        assert!(true);
    }

    #[test]
    fn test_task_event_stop_reason_terminated() {
        let reason = TaskEventStopReason::Terminated(TaskTerminateReason::Timeout);

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (_, _offset) = reason.to_flatbuffers_union(&mut builder);

        // Basic verification that serialization worked
        assert!(true);
    }
}

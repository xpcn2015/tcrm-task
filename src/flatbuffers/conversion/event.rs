use crate::{
    flatbuffers::tcrm_task_generated,
    tasks::event::{TaskEvent, TaskEventStopReason},
};

impl TaskEvent {
    pub fn to_flatbuffers<'a>(
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
                let (reason_type, reason) = reason.to_flatbuffers(builder);
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
}

impl TaskEventStopReason {
    pub fn to_flatbuffers<'a>(
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
}

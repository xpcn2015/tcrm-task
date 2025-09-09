use crate::{
    flatbuffers::{conversion::error::ConversionError, tcrm_task_generated},
    tasks::state::{TaskState, TaskTerminateReason},
};

impl TryFrom<tcrm_task_generated::tcrm::task::TaskState> for TaskState {
    type Error = ConversionError;

    fn try_from(fb_state: tcrm_task_generated::tcrm::task::TaskState) -> Result<Self, Self::Error> {
        match fb_state {
            tcrm_task_generated::tcrm::task::TaskState::Pending => Ok(TaskState::Pending),
            tcrm_task_generated::tcrm::task::TaskState::Initiating => Ok(TaskState::Initiating),
            tcrm_task_generated::tcrm::task::TaskState::Running => Ok(TaskState::Running),
            tcrm_task_generated::tcrm::task::TaskState::Ready => Ok(TaskState::Ready),
            tcrm_task_generated::tcrm::task::TaskState::Finished => Ok(TaskState::Finished),
            _ => Err(ConversionError::InvalidTaskState(fb_state.0)),
        }
    }
}

impl From<TaskState> for tcrm_task_generated::tcrm::task::TaskState {
    fn from(state: TaskState) -> Self {
        match state {
            TaskState::Pending => tcrm_task_generated::tcrm::task::TaskState::Pending,
            TaskState::Initiating => tcrm_task_generated::tcrm::task::TaskState::Initiating,
            TaskState::Running => tcrm_task_generated::tcrm::task::TaskState::Running,
            TaskState::Ready => tcrm_task_generated::tcrm::task::TaskState::Ready,
            TaskState::Finished => tcrm_task_generated::tcrm::task::TaskState::Finished,
        }
    }
}

impl TaskTerminateReason {
    pub fn to_flatbuffers<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskTerminateReason,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskTerminateReason::Timeout => {
                let r = tcrm_task_generated::tcrm::task::TimeoutReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::TimeoutReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskTerminateReason::Timeout,
                    r.as_union_value(),
                )
            }

            TaskTerminateReason::Cleanup => {
                let r = tcrm_task_generated::tcrm::task::CleanupReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::CleanupReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskTerminateReason::Cleanup,
                    r.as_union_value(),
                )
            }
            TaskTerminateReason::DependenciesFinished => {
                let r = tcrm_task_generated::tcrm::task::DependenciesFinishedReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DependenciesFinishedReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskTerminateReason::DependenciesFinished,
                    r.as_union_value(),
                )
            }
            TaskTerminateReason::Custom(message) => {
                let msg_offset = builder.create_string(message);
                let r = tcrm_task_generated::tcrm::task::CustomReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::CustomReasonArgs {
                        message: Some(msg_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskTerminateReason::Custom,
                    r.as_union_value(),
                )
            }
        }
    }
    pub fn to_flatbuffers_terminated<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskTerminateReason::Timeout => {
                let r = tcrm_task_generated::tcrm::task::TimeoutReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::TimeoutReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout,
                    r.as_union_value(),
                )
            }

            TaskTerminateReason::Cleanup => {
                let r = tcrm_task_generated::tcrm::task::CleanupReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::CleanupReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup,
                    r.as_union_value(),
                )
            }
            TaskTerminateReason::DependenciesFinished => {
                let r = tcrm_task_generated::tcrm::task::DependenciesFinishedReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DependenciesFinishedReasonArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished,
                    r.as_union_value(),
                )
            }
            TaskTerminateReason::Custom(msg) => {
                let msg_offset = builder.create_string(msg);
                let r = tcrm_task_generated::tcrm::task::CustomReason::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::CustomReasonArgs {
                        message: Some(msg_offset),
                    },
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCustom,
                    r.as_union_value(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::state::{TaskState, TaskTerminateReason};

    #[test]
    fn test_task_state_roundtrip() {
        let states = vec![
            TaskState::Pending,
            TaskState::Initiating,
            TaskState::Running,
            TaskState::Ready,
            TaskState::Finished,
        ];

        for state in states {
            let fb_state: tcrm_task_generated::tcrm::task::TaskState = state.clone().into();
            let converted_back: TaskState = fb_state.try_into().unwrap();
            assert_eq!(state, converted_back);
        }
    }

    #[test]
    fn test_task_state_invalid() {
        let invalid_state = tcrm_task_generated::tcrm::task::TaskState(99); // Invalid value
        let result: Result<TaskState, ConversionError> = invalid_state.try_into();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::InvalidTaskState(val) => assert_eq!(val, 99),
            _ => panic!("Expected InvalidTaskState error"),
        }
    }

    #[test]
    fn test_task_terminate_reason_timeout() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskTerminateReason::Timeout;

        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Timeout
        );
    }

    #[test]
    fn test_task_terminate_reason_cleanup() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskTerminateReason::Cleanup;

        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Cleanup
        );
    }

    #[test]
    fn test_task_terminate_reason_dependencies_finished() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskTerminateReason::DependenciesFinished;

        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::DependenciesFinished
        );
    }

    #[test]
    fn test_task_terminate_reason_custom() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let custom_msg = "Custom termination message";
        let reason = TaskTerminateReason::Custom(custom_msg.to_string());

        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);
        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Custom
        );
    }

    #[test]
    fn test_task_terminate_reason_to_flatbuffers_terminated_timeout() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskTerminateReason::Timeout;

        let (stop_reason, _offset) = reason.to_flatbuffers_terminated(&mut builder);
        assert_eq!(
            stop_reason,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout
        );
    }

    #[test]
    fn test_task_terminate_reason_to_flatbuffers_terminated_cleanup() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskTerminateReason::Cleanup;

        let (stop_reason, _offset) = reason.to_flatbuffers_terminated(&mut builder);
        assert_eq!(
            stop_reason,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup
        );
    }

    #[test]
    fn test_task_terminate_reason_to_flatbuffers_terminated_dependencies() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let reason = TaskTerminateReason::DependenciesFinished;

        let (stop_reason, _offset) = reason.to_flatbuffers_terminated(&mut builder);
        assert_eq!(
            stop_reason,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished
        );
    }

    #[test]
    fn test_task_terminate_reason_to_flatbuffers_terminated_custom() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let custom_msg = "Custom termination";
        let reason = TaskTerminateReason::Custom(custom_msg.to_string());

        let (stop_reason, _offset) = reason.to_flatbuffers_terminated(&mut builder);
        assert_eq!(
            stop_reason,
            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCustom
        );
    }
}

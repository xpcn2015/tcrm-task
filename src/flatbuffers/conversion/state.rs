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
    pub fn to_flatbuffer<'a>(
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
    pub fn to_flatbuffer_terminated<'a>(
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

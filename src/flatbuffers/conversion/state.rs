use crate::{
    flatbuffers::{
        conversion::{ToFlatbuffersUnion, error::ConversionError},
        tcrm_task_generated,
    },
    tasks::{event::TaskTerminateReason, state::TaskState},
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

impl TryFrom<tcrm_task_generated::tcrm::task::TaskTerminateReason> for TaskTerminateReason {
    type Error = ConversionError;

    fn try_from(
        fb_reason: tcrm_task_generated::tcrm::task::TaskTerminateReason,
    ) -> Result<Self, Self::Error> {
        match fb_reason {
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Timeout => {
                Ok(TaskTerminateReason::Timeout)
            }
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Cleanup => {
                Ok(TaskTerminateReason::Cleanup)
            }
            tcrm_task_generated::tcrm::task::TaskTerminateReason::DependenciesFinished => {
                Ok(TaskTerminateReason::DependenciesFinished)
            }
            _ => Err(ConversionError::InvalidTaskTerminateReasonType(fb_reason.0)),
        }
    }
}

impl From<TaskTerminateReason> for tcrm_task_generated::tcrm::task::TaskTerminateReason {
    fn from(reason: TaskTerminateReason) -> Self {
        match reason {
            TaskTerminateReason::Timeout => {
                tcrm_task_generated::tcrm::task::TaskTerminateReason::Timeout
            }
            TaskTerminateReason::Cleanup => {
                tcrm_task_generated::tcrm::task::TaskTerminateReason::Cleanup
            }
            TaskTerminateReason::DependenciesFinished => {
                tcrm_task_generated::tcrm::task::TaskTerminateReason::DependenciesFinished
            }
            TaskTerminateReason::UserRequested => {
                tcrm_task_generated::tcrm::task::TaskTerminateReason::UserRequested
            }
        }
    }
}

impl<'a> ToFlatbuffersUnion<'a, tcrm_task_generated::tcrm::task::TaskEventStopReason>
    for TaskTerminateReason
{
    fn to_flatbuffers_union(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        tcrm_task_generated::tcrm::task::TaskEventStopReason,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match self {
            TaskTerminateReason::Timeout => {
                let r = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout,
                    r.as_union_value(),
                )
            }
            TaskTerminateReason::Cleanup => {
                let r = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup,
                    r.as_union_value(),
                )
            }
            TaskTerminateReason::DependenciesFinished => {
                let r = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                (
                            tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished,
                            r.as_union_value(),
                        )
            }
            TaskTerminateReason::UserRequested => {
                let r = tcrm_task_generated::tcrm::task::DummyTable::create(
                    builder,
                    &tcrm_task_generated::tcrm::task::DummyTableArgs {},
                );
                (
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedUserRequested,
                    r.as_union_value(),
                )
            }
        }
    }
}

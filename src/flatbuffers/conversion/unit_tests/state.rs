use crate::{
    flatbuffers::{conversion::ToFlatbuffersUnion, tcrm_task_generated},
    tasks::{event::TaskTerminateReason, state::TaskState},
};

#[test]
fn roundtrip() {
    let test_cases = vec![
        TaskState::Pending,
        TaskState::Initiating,
        TaskState::Running,
        TaskState::Ready,
        TaskState::Finished,
    ];

    for original_state in test_cases {
        let fb_state: tcrm_task_generated::tcrm::task::TaskState = original_state.clone().into();
        let converted_state = TaskState::try_from(fb_state).unwrap();
        assert_eq!(original_state, converted_state);
    }
}

#[test]
fn terminate_reason_roundtrip() {
    let test_cases = vec![
        TaskTerminateReason::Timeout,
        TaskTerminateReason::Cleanup,
        TaskTerminateReason::DependenciesFinished,
    ];

    for original_reason in test_cases {
        let fb_reason: tcrm_task_generated::tcrm::task::TaskTerminateReason =
            original_reason.clone().into();
        let converted_reason = TaskTerminateReason::try_from(fb_reason).unwrap();
        assert_eq!(original_reason, converted_reason);
    }
}

#[test]
fn terminate_reason_to_flatbuffers_terminated() {
    let reasons = vec![
        TaskTerminateReason::Timeout,
        TaskTerminateReason::Cleanup,
        TaskTerminateReason::DependenciesFinished,
    ];

    for reason in reasons {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (stop_reason, _offset) = reason.to_flatbuffers_union(&mut builder);

        match reason {
            TaskTerminateReason::Timeout => {
                assert_eq!(
                    stop_reason,
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout
                );
            }
            TaskTerminateReason::Cleanup => {
                assert_eq!(
                    stop_reason,
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup
                );
            }
            TaskTerminateReason::DependenciesFinished => {
                assert_eq!(
                        stop_reason,
                        tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished
                    );
            }
            TaskTerminateReason::UserRequested => {
                assert_eq!(
                    stop_reason,
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedUserRequested
                );
            }
        }
    }
}

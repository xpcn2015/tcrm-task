use crate::{
    flatbuffers::{conversion::ToFlatbuffersUnion, tcrm_task_generated},
    tasks::{
        event::TaskTerminateReason,
        process::control::ProcessControlAction,
        state::{ProcessState, TaskState},
    },
};

/// Test TaskState roundtrip conversions
#[test]
fn task_state_roundtrip() {
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
        assert_eq!(
            original_state, converted_state,
            "TaskState roundtrip failed for {:?}",
            original_state
        );
    }
}

/// Test ProcessState roundtrip conversions  
#[test]
fn process_state_roundtrip() {
    let test_cases = vec![ProcessState::Running, ProcessState::Stopped];

    for original_state in test_cases {
        let fb_state: tcrm_task_generated::tcrm::task::ProcessState = original_state.into();
        let converted_state = ProcessState::try_from(fb_state).unwrap();
        assert_eq!(
            original_state, converted_state,
            "ProcessState roundtrip failed for {:?}",
            original_state
        );
    }
}

/// Test ProcessControlAction roundtrip conversions
#[cfg(feature = "process-control")]
#[test]
fn process_control_action_roundtrip() {
    let test_cases = vec![
        ProcessControlAction::Pause,
        ProcessControlAction::Resume,
        ProcessControlAction::Stop,
    ];

    for original_action in test_cases {
        let fb_action: tcrm_task_generated::tcrm::task::ProcessControlAction =
            original_action.into();
        let converted_action = ProcessControlAction::try_from(fb_action).unwrap();
        assert_eq!(
            original_action, converted_action,
            "ProcessControlAction roundtrip failed for {:?}",
            original_action
        );
    }
}

/// Test enum value limits to ensure we don't exceed FlatBuffers signed byte limits
#[test]
fn enum_value_limits() {
    use tcrm_task_generated::tcrm::task;

    // All enum values should be within signed byte range (-128 to 127)

    // TaskState
    assert!(task::TaskState::Pending.0 >= -128 && task::TaskState::Pending.0 <= 127);
    assert!(task::TaskState::Initiating.0 >= -128 && task::TaskState::Initiating.0 <= 127);
    assert!(task::TaskState::Running.0 >= -128 && task::TaskState::Running.0 <= 127);
    assert!(task::TaskState::Ready.0 >= -128 && task::TaskState::Ready.0 <= 127);
    assert!(task::TaskState::Finished.0 >= -128 && task::TaskState::Finished.0 <= 127);
    assert!(task::TaskState::Invalid.0 >= -128 && task::TaskState::Invalid.0 <= 127);

    // ProcessState
    assert!(task::ProcessState::Running.0 >= -128 && task::ProcessState::Running.0 <= 127);
    assert!(task::ProcessState::Stopped.0 >= -128 && task::ProcessState::Stopped.0 <= 127);
    assert!(task::ProcessState::Invalid.0 >= -128 && task::ProcessState::Invalid.0 <= 127);

    // ProcessControlAction
    assert!(
        task::ProcessControlAction::Pause.0 >= -128 && task::ProcessControlAction::Pause.0 <= 127
    );
    assert!(
        task::ProcessControlAction::Resume.0 >= -128 && task::ProcessControlAction::Resume.0 <= 127
    );
    assert!(
        task::ProcessControlAction::Stop.0 >= -128 && task::ProcessControlAction::Stop.0 <= 127
    );
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
        TaskTerminateReason::UserRequested,
        TaskTerminateReason::Timeout,
        TaskTerminateReason::Cleanup,
        TaskTerminateReason::DependenciesFinished,
        TaskTerminateReason::InternalError,
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
            TaskTerminateReason::InternalError => {
                assert_eq!(
                    stop_reason,
                    tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedInternalError
                );
            }
        }
    }
}

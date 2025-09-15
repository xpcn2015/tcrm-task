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
    /// Converts the termination reason to `FlatBuffers` representation.
    ///
    /// Returns both the discriminant enum value and the associated union data
    /// required for `FlatBuffers` union serialization.
    ///
    /// # Arguments
    ///
    /// * `builder` - `FlatBuffers` builder for creating the serialized data.
    ///
    /// # Returns
    ///
    /// A tuple containing the discriminant enum and the union offset data.
    pub fn to_flatbuffers(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'_>,
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
    /// Converts the termination reason to `FlatBuffers` terminated event representation.
    ///
    /// Similar to `to_flatbuffers` but creates the appropriate union data for
    /// `TaskEventStopReason` when the task has been terminated.
    ///
    /// # Arguments
    ///
    /// * `builder` - `FlatBuffers` builder for creating the serialized data.
    ///
    /// # Returns
    ///
    /// A tuple containing the stop reason discriminant and the union offset data.
    pub fn to_flatbuffers_terminated(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'_>,
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

    /// Converts from `FlatBuffers` representation to `TaskTerminateReason`.
    ///
    /// # Arguments
    ///
    /// * `reason_type` - The discriminant enum indicating which variant is stored
    /// * `reason_data` - The FlatBuffers table containing the variant data
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError`] if:
    /// - The reason type is not recognized
    /// - Required fields are missing
    pub fn from_flatbuffers_with_type(
        reason_type: tcrm_task_generated::tcrm::task::TaskTerminateReason,
        reason_data: flatbuffers::Table<'_>,
    ) -> Result<Self, ConversionError> {
        match reason_type {
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Timeout => {
                Ok(TaskTerminateReason::Timeout)
            }
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Cleanup => {
                Ok(TaskTerminateReason::Cleanup)
            }
            tcrm_task_generated::tcrm::task::TaskTerminateReason::DependenciesFinished => {
                Ok(TaskTerminateReason::DependenciesFinished)
            }
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Custom => {
                let custom_reason = unsafe {
                    tcrm_task_generated::tcrm::task::CustomReason::init_from_table(reason_data)
                };
                Ok(TaskTerminateReason::Custom(
                    custom_reason.message().to_string(),
                ))
            }
            _ => Err(ConversionError::InvalidTaskTerminateReasonType(
                reason_type.0 as i8,
            )),
        }
    }
}

// Individual reason type conversions for standalone usage

impl tcrm_task_generated::tcrm::task::TimeoutReason<'_> {
    /// Converts from `FlatBuffers` TimeoutReason to `TaskTerminateReason::Timeout`.
    pub fn to_task_terminate_reason(&self) -> TaskTerminateReason {
        TaskTerminateReason::Timeout
    }
}

impl tcrm_task_generated::tcrm::task::CleanupReason<'_> {
    /// Converts from `FlatBuffers` CleanupReason to `TaskTerminateReason::Cleanup`.
    pub fn to_task_terminate_reason(&self) -> TaskTerminateReason {
        TaskTerminateReason::Cleanup
    }
}

impl tcrm_task_generated::tcrm::task::DependenciesFinishedReason<'_> {
    /// Converts from `FlatBuffers` DependenciesFinishedReason to `TaskTerminateReason::DependenciesFinished`.
    pub fn to_task_terminate_reason(&self) -> TaskTerminateReason {
        TaskTerminateReason::DependenciesFinished
    }
}

impl tcrm_task_generated::tcrm::task::CustomReason<'_> {
    /// Converts from `FlatBuffers` CustomReason to `TaskTerminateReason::Custom`.
    pub fn to_task_terminate_reason(&self) -> TaskTerminateReason {
        TaskTerminateReason::Custom(self.message().to_string())
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

    #[test]
    fn test_flatbuffer_direct_read_all_states() {
        let states = vec![
            (
                TaskState::Pending,
                tcrm_task_generated::tcrm::task::TaskState::Pending,
            ),
            (
                TaskState::Initiating,
                tcrm_task_generated::tcrm::task::TaskState::Initiating,
            ),
            (
                TaskState::Running,
                tcrm_task_generated::tcrm::task::TaskState::Running,
            ),
            (
                TaskState::Ready,
                tcrm_task_generated::tcrm::task::TaskState::Ready,
            ),
            (
                TaskState::Finished,
                tcrm_task_generated::tcrm::task::TaskState::Finished,
            ),
        ];

        for (state, expected_fb_state) in states {
            let fb_state: tcrm_task_generated::tcrm::task::TaskState = state.clone().into();
            assert_eq!(fb_state, expected_fb_state);
            assert_eq!(fb_state.0, expected_fb_state.0);

            let converted_back: TaskState = fb_state.try_into().unwrap();
            assert_eq!(state, converted_back);
        }
    }

    #[test]
    fn test_task_terminate_reason_unicode_custom() {
        let unicode_msg = "Unicode message: 终止原因 🛑";
        let reason = TaskTerminateReason::Custom(unicode_msg.to_string());

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);

        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Custom
        );
    }

    #[test]
    fn test_task_terminate_reason_empty_custom() {
        let reason = TaskTerminateReason::Custom("".to_string());

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);

        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Custom
        );
    }

    #[test]
    fn test_task_terminate_reason_large_custom() {
        let large_msg = "a".repeat(10000);
        let reason = TaskTerminateReason::Custom(large_msg.clone());

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);

        assert_eq!(
            fb_reason,
            tcrm_task_generated::tcrm::task::TaskTerminateReason::Custom
        );
    }

    #[test]
    fn test_multiple_state_conversions() {
        for _ in 0..100 {
            for state in &[
                TaskState::Pending,
                TaskState::Initiating,
                TaskState::Running,
                TaskState::Ready,
                TaskState::Finished,
            ] {
                let fb_state: tcrm_task_generated::tcrm::task::TaskState = state.clone().into();
                let converted_back: TaskState = fb_state.try_into().unwrap();
                assert_eq!(state, &converted_back);
            }
        }
    }

    #[test]
    fn test_all_terminate_reasons_to_flatbuffers_terminated() {
        let reasons = vec![
            (TaskTerminateReason::Timeout, tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedTimeout),
            (TaskTerminateReason::Cleanup, tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCleanup),
            (TaskTerminateReason::DependenciesFinished, tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedDependenciesFinished),
            (TaskTerminateReason::Custom("test".to_string()), tcrm_task_generated::tcrm::task::TaskEventStopReason::TerminatedCustom),
        ];

        for (reason, expected_stop_reason) in reasons {
            let mut builder = flatbuffers::FlatBufferBuilder::new();
            let (stop_reason, _offset) = reason.to_flatbuffers_terminated(&mut builder);
            assert_eq!(stop_reason, expected_stop_reason);
        }
    }

    #[test]
    fn test_task_terminate_reason_roundtrip() {
        let test_cases = vec![
            TaskTerminateReason::Timeout,
            TaskTerminateReason::Cleanup,
            TaskTerminateReason::DependenciesFinished,
            TaskTerminateReason::Custom("Custom termination message".to_string()),
            TaskTerminateReason::Custom("".to_string()),
            TaskTerminateReason::Custom("Unicode: 终止原因 🛑".to_string()),
        ];

        for original_reason in test_cases {
            // Use the proper roundtrip approach with specific table types
            match &original_reason {
                TaskTerminateReason::Timeout => {
                    let mut builder = flatbuffers::FlatBufferBuilder::new();
                    let timeout_reason = tcrm_task_generated::tcrm::task::TimeoutReason::create(
                        &mut builder,
                        &tcrm_task_generated::tcrm::task::TimeoutReasonArgs {},
                    );
                    builder.finish(timeout_reason, None);
                    let bytes = builder.finished_data();
                    let fb_reason =
                        flatbuffers::root::<tcrm_task_generated::tcrm::task::TimeoutReason>(bytes)
                            .unwrap();
                    let converted = fb_reason.to_task_terminate_reason();
                    assert_eq!(original_reason, converted);
                }
                TaskTerminateReason::Cleanup => {
                    let mut builder = flatbuffers::FlatBufferBuilder::new();
                    let cleanup_reason = tcrm_task_generated::tcrm::task::CleanupReason::create(
                        &mut builder,
                        &tcrm_task_generated::tcrm::task::CleanupReasonArgs {},
                    );
                    builder.finish(cleanup_reason, None);
                    let bytes = builder.finished_data();
                    let fb_reason =
                        flatbuffers::root::<tcrm_task_generated::tcrm::task::CleanupReason>(bytes)
                            .unwrap();
                    let converted = fb_reason.to_task_terminate_reason();
                    assert_eq!(original_reason, converted);
                }
                TaskTerminateReason::DependenciesFinished => {
                    let mut builder = flatbuffers::FlatBufferBuilder::new();
                    let deps_reason =
                        tcrm_task_generated::tcrm::task::DependenciesFinishedReason::create(
                            &mut builder,
                            &tcrm_task_generated::tcrm::task::DependenciesFinishedReasonArgs {},
                        );
                    builder.finish(deps_reason, None);
                    let bytes = builder.finished_data();
                    let fb_reason = flatbuffers::root::<
                        tcrm_task_generated::tcrm::task::DependenciesFinishedReason,
                    >(bytes)
                    .unwrap();
                    let converted = fb_reason.to_task_terminate_reason();
                    assert_eq!(original_reason, converted);
                }
                TaskTerminateReason::Custom(msg) => {
                    let mut builder = flatbuffers::FlatBufferBuilder::new();
                    let msg_offset = builder.create_string(msg);
                    let custom_reason = tcrm_task_generated::tcrm::task::CustomReason::create(
                        &mut builder,
                        &tcrm_task_generated::tcrm::task::CustomReasonArgs {
                            message: Some(msg_offset),
                        },
                    );
                    builder.finish(custom_reason, None);
                    let bytes = builder.finished_data();
                    let fb_reason =
                        flatbuffers::root::<tcrm_task_generated::tcrm::task::CustomReason>(bytes)
                            .unwrap();
                    let converted = fb_reason.to_task_terminate_reason();
                    assert_eq!(original_reason, converted);
                }
            }
        }
    }
}

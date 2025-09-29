use std::time::UNIX_EPOCH;

use crate::tasks::{control::TaskStatusInfo, tokio::executor::TaskExecutor};

pub(crate) fn expected_started_executor_state(executor: &TaskExecutor) {
    assert!(executor.get_process_id().is_some());
    assert!(
        executor
            .get_create_at()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            > 0
    );
    assert!(executor.get_running_at().is_some());
    assert!(executor.get_exit_code().is_none());
    assert!(executor.get_finished_at().is_none());
}

pub(crate) fn expected_completed_executor_state(executor: &TaskExecutor) {
    expected_stopped_executor_state(&executor);
    assert!(executor.get_running_at().unwrap() <= executor.get_finished_at().unwrap());
}
pub(crate) fn expected_stopped_executor_state(executor: &TaskExecutor) {
    assert!(executor.get_process_id().is_none());
    assert!(
        executor
            .get_create_at()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            > 0
    );
    assert!(executor.get_finished_at().is_some());
}

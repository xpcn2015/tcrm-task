use core::panic;
use std::time::Duration;

use tokio::{sync::mpsc, time::timeout};

use crate::tasks::{
    config::TaskConfig,
    control::TaskStatusInfo,
    event::{TaskEvent, TaskStopReason},
    tokio::{
        coordinate::integration_tests::helper::{
            expected_started_executor_state, expected_stopped_executor_state,
        },
        executor::TaskExecutor,
    },
};

#[tokio::test]
async fn exit_with_error() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    #[cfg(windows)]
    let config = TaskConfig::new("cmd").args(["/C", "exit 1"]);
    #[cfg(unix)]
    let config = TaskConfig::new("false");

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    let result = executor.coordinate_start(tx).await;
    assert!(matches!(result, Ok(())));
    let mut started_event = false;
    let mut stopped_event = false;

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                started_event = true;
                expected_started_executor_state(&executor);
                assert_eq!(process_id, executor.get_process_id().unwrap());
                assert_eq!(created_at, executor.get_create_at());
                assert_eq!(running_at, executor.get_running_at().unwrap());
            }
            TaskEvent::Output { line, src } => {
                panic!("Unexpected output: {} from {:?}", line, src);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
            } => {
                stopped_event = true;
                expected_stopped_executor_state(&executor);
                assert_eq!(exit_code, Some(1));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert!(matches!(reason, TaskStopReason::Finished));
            }

            TaskEvent::Error { error } => {
                panic!("Unexpected error event: {}", error);
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(started_event);
    assert!(stopped_event);
}

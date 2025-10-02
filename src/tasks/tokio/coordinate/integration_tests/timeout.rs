use std::time::Duration;

use tokio::{sync::mpsc, time::timeout};

use crate::tasks::config::TaskConfig;
use crate::tasks::event::TaskTerminateReason;
use crate::tasks::{
    control::TaskStatusInfo,
    event::{TaskEvent, TaskStopReason},
    tokio::{
        coordinate::integration_tests::helper::{
            expected_completed_executor_state, expected_started_executor_state,
        },
        executor::TaskExecutor,
    },
};
#[tokio::test]
async fn valid() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "sleep 10"]);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep").args(["10"]);

    let config = config.timeout_ms(100);
    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();

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
                #[cfg(unix)]
                signal,
            } => {
                expected_completed_executor_state(&executor);

                #[cfg(windows)]
                assert_eq!(exit_code, None);
                #[cfg(unix)]
                assert_eq!(exit_code, None);

                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(
                    reason,
                    TaskStopReason::Terminated(TaskTerminateReason::Timeout)
                );
                #[cfg(unix)]
                assert_eq!(signal, Some(9));
                stopped_event = true;
            }

            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
            TaskEvent::ProcessControl { action } => {
                panic!("Unexpected ProcessControl event: {:?}", action);
            }
        }
    }

    assert!(started_event);
    assert!(stopped_event);
}

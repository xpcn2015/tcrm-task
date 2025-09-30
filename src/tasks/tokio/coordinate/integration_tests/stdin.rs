use std::time::Duration;

use tokio::{sync::mpsc, time::timeout};

use crate::tasks::config::TaskConfig;
use crate::tasks::error::TaskError;
use crate::tasks::{
    config::StreamSource,
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
    let config = TaskConfig::new("powershell")
        .args(["-Command", "$line = Read-Host; Write-Output $line"])
        .enable_stdin(true);
    #[cfg(unix)]
    let config = TaskConfig::new("head").args(["-n", "1"]).enable_stdin(true);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    executor.coordinate_start(tx).await.unwrap();

    let mut started = false;
    let mut output_received = false;
    let mut stopped = false;
    executor.send_stdin("สวัสดี 你好 how are you?").await.unwrap();

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                started = true;
                expected_started_executor_state(&executor);
                assert_eq!(process_id, executor.get_process_id().unwrap());
                assert_eq!(created_at, executor.get_create_at());
                assert_eq!(running_at, executor.get_running_at().unwrap());
            }
            TaskEvent::Output { line, src } => {
                output_received = true;
                assert_eq!(line, "สวัสดี 你好 how are you?");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                #[cfg(unix)]
                signal,
            } => {
                expected_completed_executor_state(&executor);
                assert_eq!(exit_code, Some(0));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                #[cfg(unix)]
                assert_eq!(signal, None);
                stopped = true;
            }

            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(started);
    assert!(output_received);
    assert!(stopped);
}
#[tokio::test]
async fn ignore() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config =
        TaskConfig::new("powershell").args(["-Command", "$line = Read-Host; Write-Output $line"]);
    #[cfg(unix)]
    let config = TaskConfig::new("head").args(["-n", "1"]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    executor.coordinate_start(tx).await.unwrap();

    let mut started = false;
    let mut stopped = false;
    let result = executor.send_stdin("สวัสดี 你好 how are you?").await;
    assert!(matches!(result.unwrap_err(), TaskError::Control(_)));

    while let Ok(Some(event)) = timeout(Duration::from_secs(1), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                started = true;
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
                assert_eq!(exit_code, Some(0));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                #[cfg(unix)]
                assert_eq!(signal, None);
                stopped = true;
            }

            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(started);
    assert!(stopped);
}

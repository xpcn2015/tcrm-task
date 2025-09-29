use std::time::Duration;

use tokio::{sync::mpsc, time::timeout};

use crate::tasks::config::TaskConfig;
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
async fn on_stdout() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Write-Output 'READY_INDICATOR'"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stdout);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["READY_INDICATOR"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stdout);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    executor.coordinate_start(tx).await.unwrap();

    let mut started = false;
    let mut output_received = false;
    let mut ready_event = false;
    let mut stopped = false;

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
                assert_eq!(line, "READY_INDICATOR");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
            } => {
                expected_completed_executor_state(&executor);
                assert_eq!(exit_code, Some(0));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                stopped = true;
            }

            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                ready_event = true;
            }
        }
    }

    assert!(started);
    assert!(output_received);
    assert!(ready_event);
    assert!(stopped);
}
#[tokio::test]
async fn on_stderr() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Write-Error", "READY_INDICATOR"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["READY_INDICATOR", " >&2"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    executor.coordinate_start(tx).await.unwrap();

    let mut started = false;
    let mut output_received = false;
    let mut ready_event = false;
    let mut stopped = false;

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
                println!("Output line: {}", line);
                assert_eq!(src, StreamSource::Stderr);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
            } => {
                expected_completed_executor_state(&executor);
                assert_eq!(exit_code, Some(1));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                stopped = true;
            }

            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                ready_event = true;
            }
        }
    }

    assert!(started);
    assert!(output_received);
    assert!(ready_event);
    assert!(stopped);
}
#[tokio::test]
async fn src_mismatch() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Write-Output 'READY_INDICATOR'"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["READY_INDICATOR"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    executor.coordinate_start(tx).await.unwrap();

    let mut started = false;
    let mut output_received = false;
    let mut stopped = false;

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
                assert_eq!(line, "READY_INDICATOR");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
            } => {
                expected_completed_executor_state(&executor);
                assert_eq!(exit_code, Some(0));
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert_eq!(reason, TaskStopReason::Finished);
                stopped = true;
            }

            TaskEvent::Error { error } => {
                panic!("Task encountered an error: {:?}", error);
            }
            TaskEvent::Ready => {
                panic!("Should not emit Ready event when source mismatches");
            }
        }
    }

    assert!(started);
    assert!(output_received);
    assert!(stopped);
}

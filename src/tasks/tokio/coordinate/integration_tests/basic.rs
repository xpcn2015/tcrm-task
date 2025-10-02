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
async fn echo_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "echo hello"]);
    #[cfg(unix)]
    let config = TaskConfig::new("echo").args(["hello"]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();

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
                assert_eq!(line, "hello");
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
                stopped = true;
                #[cfg(unix)]
                assert_eq!(signal, None);
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

    assert!(started);
    assert!(output_received);
    assert!(stopped);
}

#[tokio::test]
async fn env_echo() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    let mut env = std::collections::HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value_123".to_string());

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "echo %TEST_VAR%"])
        .env(env);
    #[cfg(unix)]
    let config = TaskConfig::new("printenv").args(["TEST_VAR"]).env(env);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config, tx);
    executor.coordinate_start().await.unwrap();
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
                assert_eq!(line, "test_value_123");
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
            TaskEvent::ProcessControl { action } => {
                panic!("Unexpected ProcessControl event: {:?}", action);
            }
        }
    }

    assert!(started);
    assert!(output_received);
    assert!(stopped);
}

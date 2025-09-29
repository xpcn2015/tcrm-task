use core::panic;
use std::time::Duration;

use tokio::{sync::mpsc, time::timeout};

use crate::tasks::{
    config::TaskConfig,
    control::TaskStatusInfo,
    error::TaskError,
    event::{TaskEvent, TaskStopReason},
    tokio::{
        coordinate::integration_tests::helper::expected_stopped_executor_state,
        executor::TaskExecutor,
    },
};

#[tokio::test]
async fn empty_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    let config = TaskConfig::new("");

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    let result = executor.coordinate_start(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));
    let mut error_event = false;
    let mut stopped_event = false;

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                panic!(
                    "Unexpected start event: pid {}, created at {:?}, running at {:?}",
                    process_id, created_at, running_at
                );
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
                assert_eq!(exit_code, None);
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert!(matches!(
                    reason,
                    TaskStopReason::Error(TaskError::InvalidConfiguration(_))
                ));
            }

            TaskEvent::Error { error } => {
                error_event = true;
                assert!(matches!(error, TaskError::InvalidConfiguration(_)));
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(stopped_event);
    assert!(error_event);
}

#[tokio::test]
async fn command_not_found() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    let config = TaskConfig::new("non_existent_command");

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    let result = executor.coordinate_start(tx).await;
    assert!(matches!(result, Err(TaskError::IO(_))));
    let mut error_event = false;
    let mut stopped_event = false;

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                panic!(
                    "Unexpected start event: pid {}, created at {:?}, running at {:?}",
                    process_id, created_at, running_at
                );
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
                assert_eq!(exit_code, None);
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert!(matches!(reason, TaskStopReason::Error(TaskError::IO(_))));
            }

            TaskEvent::Error { error } => {
                error_event = true;
                assert!(matches!(error, TaskError::IO(_)));
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(stopped_event);
    assert!(error_event);
}

#[tokio::test]
async fn not_exist_dir() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "echo test"])
        .working_dir("/non/existent/directory");
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["test"])
        .working_dir("/non/existent/directory");

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    let result = executor.coordinate_start(tx).await;
    assert!(matches!(result, Err(TaskError::IO(_))));
    let mut error_event = false;
    let mut stopped_event = false;

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                panic!(
                    "Unexpected start event: pid {}, created at {:?}, running at {:?}",
                    process_id, created_at, running_at
                );
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
                assert_eq!(exit_code, None);
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert!(matches!(reason, TaskStopReason::Error(TaskError::IO(_))));
            }

            TaskEvent::Error { error } => {
                error_event = true;
                assert!(matches!(error, TaskError::IO(_)));
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(stopped_event);
    assert!(error_event);
}

#[tokio::test]
async fn zero_timeout() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);

    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "Start-Sleep -Seconds 1"]);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep").args(["1"]);

    let config = config.timeout_ms(0);
    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);
    let result = executor.coordinate_start(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));
    let mut error_event = false;
    let mut stopped_event = false;

    while let Ok(Some(event)) = timeout(Duration::from_secs(5), rx.recv()).await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                panic!(
                    "Unexpected start event: pid {}, created at {:?}, running at {:?}",
                    process_id, created_at, running_at
                );
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
                assert_eq!(exit_code, None);
                assert_eq!(exit_code, executor.get_exit_code());
                assert_eq!(finished_at, executor.get_finished_at().unwrap());
                assert!(matches!(
                    reason,
                    TaskStopReason::Error(TaskError::InvalidConfiguration(_))
                ));
            }

            TaskEvent::Error { error } => {
                error_event = true;
                assert!(matches!(error, TaskError::InvalidConfiguration(_)));
            }
            TaskEvent::Ready => {
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(stopped_event);
    assert!(error_event);
}

use tokio::sync::mpsc;

use crate::tasks::{
    async_tokio::spawner::TaskSpawner,
    config::{StreamSource, TaskConfig, TaskShell},
    error::TaskError,
    event::{TaskEvent, TaskEventStopReason},
    state::TaskTerminateReason,
};
#[tokio::test]
async fn start_direct_fn_echo_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let config = TaskConfig::new("echo")
        .args(["hello"])
        .shell(TaskShell::Auto);
    let mut spawner = TaskSpawner::new("echo_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started { task_name } => {
                assert_eq!(task_name, "echo_task");
                started = true;
            }
            TaskEvent::Output {
                task_name,
                line,
                src,
            } => {
                assert_eq!(task_name, "echo_task");
                assert_eq!(line, "hello");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                task_name,
                exit_code,
                reason: _,
            } => {
                assert_eq!(task_name, "echo_task");
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);
}
#[tokio::test]
async fn start_direct_fn_invalid_empty_command() {
    let (tx, _rx) = mpsc::channel::<TaskEvent>(1024);
    let config = TaskConfig::new(""); // invalid: empty command
    let mut spawner = TaskSpawner::new("bad_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));
}
#[tokio::test]
async fn start_direct_timeout_terminated_task() {
    let config = TaskConfig::new("sleep")
        .args(vec!["2"])
        .timeout_ms(500)
        .shell(TaskShell::Auto);
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let mut spawner = TaskSpawner::new("sleep_with_timeout_task".into(), config);

    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started { task_name } => {
                assert_eq!(task_name, "sleep_with_timeout_task");
                started = true;
            }

            TaskEvent::Stopped {
                task_name,
                exit_code,
                reason,
            } => {
                assert_eq!(task_name, "sleep_with_timeout_task");
                assert_eq!(exit_code, None);
                assert_eq!(
                    reason,
                    TaskEventStopReason::Terminated(TaskTerminateReason::Timeout)
                );
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);
}

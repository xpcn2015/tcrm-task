use tokio::sync::mpsc;

#[cfg(windows)]
use crate::tasks::config::TaskConfig;
use crate::tasks::{
    config::StreamSource,
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::select::executor::TaskExecutor,
};

#[tokio::test]
async fn echo_command_wait() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "echo hello"]);
    #[cfg(unix)]
    let config = TaskConfig::new("echo").args(["hello"]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let mut executor = TaskExecutor::new(config);

    let result = executor.start(tx).await;
    assert!(result.is_ok());

    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started => {
                started = true;
            }
            TaskEvent::Output { line, src } => {
                assert_eq!(line, "hello");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason: _,
            } => {
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);
    assert!(executor.finished_at.is_some());
    assert_eq!(executor.exit_code, Some(0));
    assert_eq!(executor.stop_reason, Some(TaskStopReason::Finished));
    assert!(executor.flags.stop);
    assert!(!executor.flags.ready);
    assert_eq!(executor.state, TaskState::Finished);
    assert!(executor.created_at <= executor.running_at.unwrap());
    assert!(executor.running_at.unwrap() <= executor.finished_at.unwrap());
}

#[tokio::test]
async fn echo_command_realtime() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(64);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "echo hello"]);
    #[cfg(unix)]
    let config = TaskConfig::new("echo").args(["hello"]);

    #[cfg(feature = "process-group")]
    let config = config.use_process_group(false);

    let executor = TaskExecutor::new(config);
    let handle = executor.spawn_start(tx).await;
    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started => {
                started = true;
            }
            TaskEvent::Output { line, src } => {
                assert_eq!(line, "hello");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason: _,
            } => {
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);

    let join_result = handle.await;
    assert!(
        join_result.is_ok(),
        "Spawned task panicked: {:?}",
        join_result
    );
    if let Ok(inner_result) = join_result {
        assert!(
            inner_result.is_ok(),
            "Task returned error: {:?}",
            inner_result.err()
        );
    }
}

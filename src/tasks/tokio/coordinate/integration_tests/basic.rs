use tokio::sync::mpsc;

#[cfg(windows)]
use crate::tasks::config::TaskConfig;
use crate::tasks::{
    config::StreamSource,
    control::{TaskInformation, TaskStatusInfo},
    event::{TaskEvent, TaskStopReason},
    state::TaskState,
    tokio::coordinate::executor::TaskExecutor,
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

    let mut task_info = executor.get_information();
    let result = executor.start(tx).await;
    assert!(result.is_ok());

    let mut started = false;
    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                task_info.process_id = Some(process_id);
                task_info.created_at = created_at;
                task_info.running_at = Some(running_at);
                task_info.state = TaskState::Running;
                started = true;
            }
            TaskEvent::Output { line, src } => {
                assert_eq!(line, "hello");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
            } => {
                assert_eq!(exit_code, Some(0));
                assert_eq!(reason, TaskStopReason::Finished);
                task_info.exit_code = exit_code;
                task_info.stop_reason = Some(reason);
                task_info.finished_at = Some(finished_at);
                task_info.state = TaskState::Finished;
                stopped = true;
            }
            TaskEvent::Error {
                error: _,
                finished_at,
            } => {
                task_info.exit_code = None;
                task_info.stop_reason = None;
                task_info.finished_at = Some(finished_at);
                task_info.state = TaskState::Finished;
                panic!("Task encountered an error");
            }
            TaskEvent::Ready => {
                // Not expected in this test
                panic!("Unexpected Ready event");
            }
        }
    }

    assert!(started);
    assert!(stopped);
    assert!(executor.get_finished_at().is_some());
    assert_eq!(executor.exit_code, Some(0));
    assert_eq!(executor.stop_reason, Some(TaskStopReason::Finished));
    assert!(executor.flags.stop);
    assert!(!executor.flags.ready);
    assert_eq!(executor.state, TaskState::Finished);
    assert!(executor.get_create_at() <= &executor.get_running_at().unwrap());
    assert!(executor.get_running_at().unwrap() <= executor.get_finished_at().unwrap());
    assert_eq!(task_info, executor.get_information());
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
    let mut task_info = executor.get_information();
    let handle = executor.spawn_start(tx).await;
    let mut started = false;
    let mut stopped = false;

    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                running_at,
            } => {
                started = true;
            }
            TaskEvent::Output { line, src } => {
                assert_eq!(line, "hello");
                assert_eq!(src, StreamSource::Stdout);
            }
            TaskEvent::Stopped {
                exit_code,
                reason: _,
                finished_at: _,
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

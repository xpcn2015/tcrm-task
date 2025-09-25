use tokio::sync::mpsc;

use crate::tasks::config::{StreamSource, TaskConfig};
use crate::tasks::error::TaskError;
use crate::tasks::{async_tokio::spawner::TaskSpawner, event::TaskEvent};

#[tokio::test]
async fn echo_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "echo hello"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["hello"])
        .use_process_group(false);

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
async fn process_id_returns_none_after_stopped() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "echo done"]);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["done"])
        .use_process_group(false);

    let mut spawner = TaskSpawner::new("pid_test_task".to_string(), config);
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut stopped = false;
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Stopped { task_name, .. } = event {
            assert_eq!(task_name, "pid_test_task");
            stopped = true;
            break;
        }
    }
    assert!(stopped, "Task should emit Stopped event");
    // process_id should be None after stopped
    let pid = spawner.get_process_id().await;
    assert!(
        pid.is_none(),
        "process_id should be None after task is stopped"
    );
}

#[tokio::test]
async fn process_id_returns_some_while_task_running() {
    use std::time::Duration;
    use tokio::time::sleep;
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell").args(["-Command", "Start-Sleep -Seconds 2"]);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep")
        .args(["2"])
        .use_process_group(false);

    let mut spawner = TaskSpawner::new("pid_running_task".to_string(), config);
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    // Wait a short time to ensure the process is running
    sleep(Duration::from_millis(500)).await;
    let pid = spawner.get_process_id().await;
    assert!(
        pid.is_some(),
        "process_id should be Some while task is running"
    );

    // Drain events to ensure clean test exit
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Stopped { .. } = event {
            break;
        }
    }
}
#[tokio::test]
async fn invalid_empty_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(10);
    let config = TaskConfig::new(""); // invalid: empty command
    let mut spawner = TaskSpawner::new("bad_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

    // Should receive an Error event
    let mut error_event = false;
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Error { task_name, error } = event {
            assert_eq!(task_name, "bad_task");
            assert!(matches!(error, TaskError::InvalidConfiguration(_)));
            error_event = true;
        }
    }
    assert!(error_event, "Should emit Error event for invalid config");

    // Ensure TaskState is Finished after error, not stalled at Initiating
    let state = spawner.get_state().await;
    assert_eq!(
        state,
        crate::tasks::state::TaskState::Finished,
        "TaskState should be Finished after error, not Initiating"
    );
}
#[tokio::test]
async fn command_not_found() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let config = TaskConfig::new("non_existent_command").use_process_group(false);
    let mut spawner = TaskSpawner::new("error_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::IO(_))));

    if let Some(TaskEvent::Error { task_name, error }) = rx.recv().await {
        assert_eq!(task_name, "error_task");
        assert!(matches!(error, TaskError::IO(_)));
        if let TaskError::IO(msg) = error {
            #[cfg(windows)]
            assert!(msg.contains("not found") || msg.contains("cannot find"));
            #[cfg(unix)]
            assert!(msg.contains("No such file or directory"));
        }
    } else {
        panic!("Expected TaskEvent::Error");
    }
}

#[tokio::test]
async fn invalid_working_directory() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let config = TaskConfig::new("echo").working_dir("/non/existent/directory");

    let mut spawner = TaskSpawner::new("working_dir_task".to_string(), config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

    if let Some(TaskEvent::Error { task_name, error }) = rx.recv().await {
        assert_eq!(task_name, "working_dir_task");
        assert!(matches!(error, TaskError::InvalidConfiguration(_)));
    } else {
        panic!("Expected TaskEvent::Error");
    }
}

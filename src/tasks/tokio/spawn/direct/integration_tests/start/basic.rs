use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::tasks::config::{StreamSource, TaskConfig};
use crate::tasks::error::TaskError;
use crate::tasks::state::TaskState;
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

    let mut spawner = TaskSpawner::new(config);

    let result = spawner.start_direct(tx).await;
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
}
#[tokio::test]
async fn env_echo() {
    let (tx, mut rx) = mpsc::channel(100);

    let mut env = std::collections::HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value_123".to_string());

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "echo %TEST_VAR%"])
        .env(env)
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("printenv")
        .args(["TEST_VAR"])
        .env(env)
        .use_process_group(false);

    let mut spawner = TaskSpawner::new(config);

    let result = spawner.start_direct(tx).await;
    assert!(
        result.is_ok(),
        "Task should start successfully: {}",
        result.unwrap_err()
    );

    // Collect output
    let mut output = String::new();
    while let Ok(event) = timeout(Duration::from_secs(5), rx.recv()).await {
        if let Some(event) = event {
            match event {
                TaskEvent::Output { line, .. } => {
                    output.push_str(&line);
                }
                TaskEvent::Stopped { .. } => break,
                _ => {}
            }
        } else {
            break;
        }
    }

    assert!(
        output.contains("test_value_123"),
        "Output should contain environment variable value: {}",
        output
    );
}
#[tokio::test]
async fn invalid_empty_command() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(10);
    let config = TaskConfig::new(""); // invalid: empty command
    let mut spawner = TaskSpawner::new(config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

    // Should receive an Error event
    let mut error_event = false;
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Error { error } = event {
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
    let mut spawner = TaskSpawner::new(config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::IO(_))));

    if let Some(TaskEvent::Error { error }) = rx.recv().await {
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

    let mut spawner = TaskSpawner::new(config);

    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

    if let Some(TaskEvent::Error { error }) = rx.recv().await {
        assert!(matches!(error, TaskError::InvalidConfiguration(_)));
    } else {
        panic!("Expected TaskEvent::Error");
    }
}
#[tokio::test]
async fn task_failure_handling() {
    let (tx, mut rx) = mpsc::channel(100);

    // Create a task that will fail
    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "exit 1"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("false").use_process_group(false);

    let mut spawner = TaskSpawner::new(config);

    let result = spawner.start_direct(tx).await;
    assert!(
        result.is_ok(),
        "Task should start successfully even if it will fail"
    );

    // Wait for completion
    let mut stopped_event = None;
    while let Ok(event) = timeout(Duration::from_secs(5), rx.recv()).await {
        if let Some(event) = event {
            if let TaskEvent::Stopped { exit_code, .. } = event {
                stopped_event = Some(exit_code);
                break;
            }
        } else {
            break;
        }
    }

    assert_eq!(stopped_event, Some(Some(1)), "Task should exit with code 1");
}

#[tokio::test]
async fn concurrent_task_execution() {
    // Test running multiple tasks concurrently
    let mut handles = Vec::new();

    for i in 0..3 {
        let handle = tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(100);

            #[cfg(windows)]
            let config = TaskConfig::new("cmd")
                .args(["/C", &format!("echo task_{}", i)])
                .use_process_group(false);
            #[cfg(unix)]
            let config = TaskConfig::new("echo")
                .args([&format!("task_{}", i)])
                .use_process_group(false);

            let mut spawner = TaskSpawner::new(config);

            let result = spawner.start_direct(tx).await;
            assert!(result.is_ok(), "Task {} should start successfully", i);

            // Wait for completion
            while let Ok(event) = timeout(Duration::from_secs(5), rx.recv()).await {
                if let Some(event) = event {
                    if matches!(event, TaskEvent::Stopped { .. }) {
                        return i;
                    }
                } else {
                    break;
                }
            }

            panic!("Task {} did not complete", i);
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results = futures::future::try_join_all(handles).await;
    assert!(
        results.is_ok(),
        "All concurrent tasks should complete successfully"
    );

    let task_ids: Vec<_> = results.unwrap();
    assert_eq!(task_ids.len(), 3, "Should complete 3 tasks");
    assert!(task_ids.contains(&0), "Should complete task 0");
    assert!(task_ids.contains(&1), "Should complete task 1");
    assert!(task_ids.contains(&2), "Should complete task 2");
}

#[tokio::test]
async fn test_task_state_transitions() {
    let (tx, _rx) = mpsc::channel(100);

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "echo test"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["test"])
        .use_process_group(false);

    let mut spawner = TaskSpawner::new(config);

    // Initial state should be Pending
    assert_eq!(spawner.get_state().await, TaskState::Pending);

    // Start the task
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok(), "Task should start successfully");

    // Wait a bit for state changes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // State should have changed from Pending
    let final_state = spawner.get_state().await;
    assert_ne!(
        final_state,
        TaskState::Pending,
        "State should have changed from Pending"
    );
}

use std::time::Duration;
use tokio::{sync::mpsc, time::timeout};

use crate::tasks::{
    async_tokio::spawner::TaskSpawner,
    config::{StreamSource, TaskConfig},
    event::TaskEvent,
    state::TaskState,
};

/// Integration tests that verify actual task execution behavior
/// These tests execute real processes and validate their behavior

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_actual_echo_command_execution() {
        let (tx, mut rx) = mpsc::channel(100);

        let config = if cfg!(windows) {
            TaskConfig::new("cmd").args(vec!["/C".to_string(), "echo hello_world".to_string()])
        } else {
            TaskConfig::new("echo").args(vec!["hello_world".to_string()])
        };

        let mut spawner = TaskSpawner::new("echo_test".to_string(), config);

        // Start the task
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok(), "Task should start successfully");

        // Collect all events
        let mut events = Vec::new();
        while let Ok(event) = timeout(Duration::from_secs(5), rx.recv()).await {
            if let Some(event) = event {
                let is_stopped = matches!(event, TaskEvent::Stopped { .. });
                events.push(event);
                // Stop collecting after we see the task stop
                if is_stopped {
                    break;
                }
            } else {
                break;
            }
        }

        // Verify we got the expected events
        assert!(!events.is_empty(), "Should receive events");

        // Should have Started event
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TaskEvent::Started { .. })),
            "Should receive Started event"
        );

        // Should have Output event with our text
        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                TaskEvent::Output { line, .. } => Some(line),
                _ => None,
            })
            .collect();

        assert!(!output_events.is_empty(), "Should receive output events");
        let all_output = output_events
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_output.contains("hello_world"),
            "Output should contain 'hello_world', got: {}",
            all_output
        );

        // Should have Stopped event with success
        assert!(
            events.iter().any(|e| matches!(
                e,
                TaskEvent::Stopped {
                    exit_code: Some(0),
                    ..
                }
            )),
            "Should receive Stopped event with exit code 0"
        );
    }

    #[tokio::test]
    async fn test_task_with_working_directory() {
        let (tx, mut rx) = mpsc::channel(100);

        // Create a task that outputs the current directory
        let config = if cfg!(windows) {
            TaskConfig::new("cmd")
                .args(vec!["/C".to_string(), "cd".to_string()])
                .working_dir(std::env::temp_dir().to_str().unwrap().to_string())
        } else {
            TaskConfig::new("pwd").working_dir(std::env::temp_dir().to_str().unwrap().to_string())
        };

        let mut spawner = TaskSpawner::new("pwd_test".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok(), "Task should start successfully");

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

        // Verify the working directory was set
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.to_str().unwrap();
        let temp_path_normalized = temp_path.trim_end_matches(['\\', '/']);
        assert!(
            output.contains(temp_path)
                || output.contains(&temp_path.replace('\\', "/"))
                || output.contains(temp_path_normalized),
            "Output should contain temp directory path: {}, got: {}",
            temp_path,
            output
        );
    }

    #[tokio::test]
    async fn test_task_with_environment_variables() {
        let (tx, mut rx) = mpsc::channel(100);

        let mut env = std::collections::HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value_123".to_string());

        let config = if cfg!(windows) {
            TaskConfig::new("cmd")
                .args(vec!["/C".to_string(), "echo %TEST_VAR%".to_string()])
                .env(env)
        } else {
            TaskConfig::new("sh")
                .args(vec!["-c".to_string(), "echo $TEST_VAR".to_string()])
                .env(env)
        };

        let mut spawner = TaskSpawner::new("env_test".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok(), "Task should start successfully");

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
    async fn test_task_failure_handling() {
        let (tx, mut rx) = mpsc::channel(100);

        // Create a task that will fail
        let config = if cfg!(windows) {
            TaskConfig::new("cmd").args(vec!["/C".to_string(), "exit 1".to_string()])
        } else {
            TaskConfig::new("sh").args(vec!["-c".to_string(), "exit 1".to_string()])
        };

        let mut spawner = TaskSpawner::new("fail_test".to_string(), config);

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
    async fn test_task_with_stdin() {
        let (tx, mut rx) = mpsc::channel(100);
        let (stdin_tx, stdin_rx) = mpsc::channel(100);

        let config = if cfg!(windows) {
            TaskConfig::new("cmd")
                .args(vec!["/C".to_string(), "more".to_string()])
                .enable_stdin(true)
        } else {
            TaskConfig::new("cat").enable_stdin(true)
        };

        let mut spawner = TaskSpawner::new("stdin_test".to_string(), config).set_stdin(stdin_rx);

        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok(), "Task should start successfully");

        // Send input
        let test_input = "hello from stdin\n";
        assert!(
            stdin_tx.send(test_input.to_string()).await.is_ok(),
            "Should send stdin"
        );
        drop(stdin_tx); // Close stdin

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
            output.contains("hello from stdin"),
            "Output should contain stdin input: {}",
            output
        );
    }

    #[tokio::test]
    async fn test_task_ready_indicator() {
        let (tx, mut rx) = mpsc::channel(100);

        let config = if cfg!(windows) {
            TaskConfig::new("cmd")
                .args(vec![
                    "/C".to_string(),
                    "echo READY && echo more_output".to_string(),
                ])
                .ready_indicator("READY".to_string())
                .ready_indicator_source(StreamSource::Stdout)
        } else {
            TaskConfig::new("sh")
                .args(vec![
                    "-c".to_string(),
                    "echo READY && echo more_output".to_string(),
                ])
                .ready_indicator("READY".to_string())
                .ready_indicator_source(StreamSource::Stdout)
        };

        let mut spawner = TaskSpawner::new("ready_test".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok(), "Task should start successfully");

        // Look for Ready event
        let mut found_ready = false;
        while let Ok(event) = timeout(Duration::from_secs(5), rx.recv()).await {
            if let Some(event) = event {
                match event {
                    TaskEvent::Ready { task_name } => {
                        assert_eq!(task_name, "ready_test");
                        found_ready = true;
                    }
                    TaskEvent::Stopped { .. } => break,
                    _ => {}
                }
            } else {
                break;
            }
        }

        assert!(
            found_ready,
            "Should receive Ready event when indicator is found"
        );
    }

    #[tokio::test]
    async fn test_concurrent_task_execution() {
        // Test running multiple tasks concurrently
        let mut handles = Vec::new();

        for i in 0..3 {
            let handle = tokio::spawn(async move {
                let (tx, mut rx) = mpsc::channel(100);

                let config = if cfg!(windows) {
                    TaskConfig::new("cmd").args(vec!["/C".to_string(), format!("echo task_{}", i)])
                } else {
                    TaskConfig::new("echo").args(vec![format!("task_{}", i)])
                };

                let mut spawner = TaskSpawner::new(format!("concurrent_test_{}", i), config);

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

        let config = if cfg!(windows) {
            TaskConfig::new("cmd").args(vec!["/C".to_string(), "echo test".to_string()])
        } else {
            TaskConfig::new("echo").args(vec!["test".to_string()])
        };

        let mut spawner = TaskSpawner::new("state_test".to_string(), config);

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
}

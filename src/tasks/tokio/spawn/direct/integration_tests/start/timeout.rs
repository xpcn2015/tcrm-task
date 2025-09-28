use tokio::sync::mpsc;

#[cfg(windows)]
use crate::tasks::config::TaskConfig;
use crate::tasks::{
    tokio::spawn::spawner::TaskSpawner,
    error::TaskError,
    event::{TaskEvent, TaskStopReason, TaskTerminateReason},
};

#[tokio::test]
async fn terminated_task() {
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "sleep 2"])
        .timeout_ms(1)
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep")
        .args(["2"])
        .timeout_ms(1)
        .use_process_group(false);

    let (tx, mut rx) = mpsc::channel::<TaskEvent>(12);
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

            TaskEvent::Stopped { exit_code, reason } => {
                assert_eq!(exit_code, None);
                assert_eq!(
                    reason,
                    TaskStopReason::Terminated(TaskTerminateReason::Timeout)
                );
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(stopped);
}

#[tokio::test]
async fn error_zero_timeout() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Start-Sleep -Seconds 1"])
        .timeout_ms(0);
    #[cfg(unix)]
    let config = TaskConfig::new("sleep").args(["1"]).timeout_ms(0);

    let mut spawner = TaskSpawner::new(config);

    // Zero timeout should be rejected as invalid configuration
    let result = spawner.start_direct(tx).await;
    assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

    // Should receive an error event
    if let Some(TaskEvent::Error { error }) = rx.recv().await {
        assert!(matches!(error, TaskError::InvalidConfiguration(_)));
    } else {
        panic!("Expected TaskEvent::Error with InvalidConfiguration");
    }
}

use tokio::sync::mpsc;

use crate::tasks::config::TaskConfig;
use crate::tasks::{async_tokio::spawner::TaskSpawner, event::TaskEvent};

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

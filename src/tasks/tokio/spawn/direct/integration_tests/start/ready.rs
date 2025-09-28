use tokio::sync::mpsc;

use crate::tasks::config::{StreamSource, TaskConfig};
use crate::tasks::{async_tokio::spawner::TaskSpawner, event::TaskEvent};

#[tokio::test]
async fn ready_indicator_on_stdout() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(15);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Write-Output 'READY_INDICATOR'"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stdout)
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["READY_INDICATOR"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stdout)
        .use_process_group(false);

    let mut spawner = TaskSpawner::new(config);
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut ready_event = false;
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Ready = event {
            ready_event = true;
        }
    }
    assert!(
        ready_event,
        "Should emit Ready event when indicator is in stdout"
    );
}
#[tokio::test]
async fn ready_indicator_source_stderr() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(15);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Write-Error 'READY_INDICATOR'"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr)
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("bash")
        .args(["-c", "echo READY_INDICATOR >&2"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr)
        .use_process_group(false);

    let mut spawner = TaskSpawner::new(config);
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut ready_event = false;
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Ready = event {
            ready_event = true;
        }
    }
    assert!(
        ready_event,
        "Should emit Ready event when indicator is in stderr"
    );
}

#[tokio::test]
async fn ready_indicator_source_mismatch() {
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "Write-Output 'READY_INDICATOR'"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr)
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["READY_INDICATOR"])
        .ready_indicator("READY_INDICATOR".to_string())
        .ready_indicator_source(StreamSource::Stderr)
        .use_process_group(false);

    let mut spawner = TaskSpawner::new(config);
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    let mut ready_event = false;
    while let Some(event) = rx.recv().await {
        if let TaskEvent::Ready { .. } = event {
            ready_event = true;
        }
    }
    assert!(
        !ready_event,
        "Should NOT emit Ready event if indicator is in wrong stream"
    );
}

use tokio::sync::mpsc;

use crate::tasks::{
    config::TaskConfig, error::TaskError, event::TaskTerminateReason,
    tokio::spawn::spawner::TaskSpawner,
};

#[tokio::test]
async fn stdin_disabled_ignores_channel_set() {
    let config = TaskConfig::new("echo").enable_stdin(false);
    let (_, rx) = mpsc::channel(100);

    let spawner = TaskSpawner::new(config).set_stdin(rx);
    assert!(spawner.stdin_rx.is_none());
}

#[tokio::test]
async fn reject_terminate_signal_on_no_channel_set() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);

    let result = spawner
        .send_terminate_signal(TaskTerminateReason::Cleanup)
        .await;
    assert!(result.is_err());
    match result {
        Err(TaskError::Channel(_)) => {} // pass
        _ => panic!("Expected Channel error"),
    }
}

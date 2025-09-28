use crate::tasks::{config::TaskConfig, tokio::spawn::spawner::TaskSpawner};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn created_at_increases_over_time() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    let create_at1 = spawner.get_task_info().await.created_at.elapsed();
    sleep(Duration::from_millis(20)).await;
    let create_at2 = spawner.get_task_info().await.created_at.elapsed();
    assert!(
        create_at2 > create_at1,
        "created_at should increase when time passes"
    );
}

#[tokio::test]
async fn zero_uptime_if_not_started() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    let uptime1 = spawner.get_task_info().await.uptime;
    sleep(Duration::from_millis(20)).await;
    let uptime2 = spawner.get_task_info().await.uptime;
    assert_eq!(uptime1, Duration::ZERO, "Initial uptime should be zero");
    assert_eq!(
        uptime2,
        Duration::ZERO,
        "Uptime should remain zero if task not started"
    );
}

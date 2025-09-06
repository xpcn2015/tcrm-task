use std::time::Duration;

use tokio::time::sleep;

use crate::tasks::{
    async_tokio::spawner::{TaskInfo, TaskSpawner},
    config::TaskConfig,
    state::TaskState,
};

#[tokio::test]
async fn initial_task_state_should_be_pending() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new("test_task".to_string(), config);

    let state = spawner.get_state().await;
    assert_eq!(state, TaskState::Pending);
}
#[tokio::test]
async fn update_state_fn_should_be_able_to_update_state() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new("test_task".to_string(), config);

    spawner.update_state(TaskState::Running).await;
    let state = spawner.get_state().await;
    assert_eq!(state, TaskState::Running);
}
#[tokio::test]
async fn is_running_fn_should_return_true_when_state_is_running_or_ready() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new("test_task".to_string(), config);

    assert!(!spawner.is_running().await);
    spawner.update_state(TaskState::Running).await;
    assert!(spawner.is_running().await);
    spawner.update_state(TaskState::Ready).await;
    assert!(spawner.is_running().await);
}

#[tokio::test]
async fn uptime_fn_should_increases_when_time_pass() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new("test_task".to_string(), config);

    let uptime1 = spawner.uptime();
    sleep(Duration::from_millis(20)).await;
    let uptime2 = spawner.uptime();
    assert!(uptime2 > uptime1);
}

#[tokio::test]
async fn get_task_info_fn_should_return_correct_imformation() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new("test_task".to_string(), config);

    let info: TaskInfo = spawner.get_task_info().await;
    assert_eq!(info.name, "test_task");
    assert_eq!(info.state, TaskState::Pending);
}

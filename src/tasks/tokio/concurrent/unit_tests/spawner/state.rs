use crate::tasks::{config::TaskConfig, state::TaskState, tokio::spawn::spawner::TaskSpawner};

#[tokio::test]
async fn fn_is_running_returns_true_when_state_running() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    assert!(
        !spawner.is_running().await,
        "Should not be running initially"
    );
    spawner.update_state(TaskState::Running).await;
    assert!(spawner.is_running().await, "Should be running after update");
}

#[tokio::test]
async fn fn_is_running_false_for_non_running_states() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    for state in [TaskState::Pending, TaskState::Ready, TaskState::Finished] {
        spawner.update_state(state.clone()).await;
        assert!(
            !spawner.is_running().await,
            "Should not be running for state: {:?}",
            state
        );
    }
}

#[tokio::test]
async fn fn_is_ready_returns_true_when_state_ready() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    assert!(!spawner.is_ready().await, "Should not be ready initially");
    spawner.update_state(TaskState::Ready).await;
    assert!(spawner.is_ready().await, "Should be ready after update");
}

#[tokio::test]
async fn fn_is_ready_false_for_non_ready_states() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    for state in [TaskState::Pending, TaskState::Running, TaskState::Finished] {
        spawner.update_state(state.clone()).await;
        assert!(
            !spawner.is_ready().await,
            "Should not be ready for state: {:?}",
            state
        );
    }
}

#[tokio::test]
async fn initial_state_is_pending() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    let state = spawner.get_state().await;
    assert_eq!(state, TaskState::Pending, "Initial state should be Pending");
}

#[tokio::test]
async fn state_transitions() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    let states = [
        TaskState::Pending,
        TaskState::Running,
        TaskState::Ready,
        TaskState::Finished,
    ];
    for state in states.iter() {
        spawner.update_state(state.clone()).await;
        let current = spawner.get_state().await;
        assert_eq!(current, *state, "State should transition to {:?}", state);
    }
}

#[tokio::test]
async fn update_state_changes_state() {
    let config = TaskConfig::new("echo");
    let spawner = TaskSpawner::new(config);
    spawner.update_state(TaskState::Running).await;
    let state = spawner.get_state().await;
    assert_eq!(
        state,
        TaskState::Running,
        "State should be Running after update"
    );
}

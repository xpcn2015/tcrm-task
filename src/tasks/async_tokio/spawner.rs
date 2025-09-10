use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout};

use crate::tasks::error::TaskError;
use crate::tasks::state::TaskTerminateReason;
use crate::tasks::{config::TaskConfig, state::TaskState};

// TODO: Consider adding serde support for TaskInfo, not skipping Instant fields
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub name: String,
    pub state: TaskState,
    pub uptime: Duration,
    #[cfg_attr(feature = "serde", serde(skip, default = "default_instant"))]
    pub created_at: Instant,
    #[cfg_attr(feature = "serde", serde(skip, default))]
    pub finished_at: Option<Instant>,
}

#[cfg(feature = "serde")]
fn default_instant() -> Instant {
    Instant::now()
}
/// Spawns and manages the lifecycle of a task
#[derive(Debug)]
pub struct TaskSpawner {
    pub(crate) config: TaskConfig,
    pub(crate) task_name: String,
    pub(crate) state: Arc<RwLock<TaskState>>,
    pub(crate) terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    pub(crate) process_id: Arc<RwLock<Option<u32>>>,
    pub(crate) created_at: Instant,
    pub(crate) finished_at: Arc<RwLock<Option<Instant>>>,
    pub(crate) stdin_rx: Option<mpsc::Receiver<String>>,
}

impl TaskSpawner {
    /// Create a new task spawner for the given task name and configuration
    pub fn new(task_name: String, config: TaskConfig) -> Self {
        Self {
            task_name,
            config,
            state: Arc::new(RwLock::new(TaskState::Pending)),
            terminate_tx: Arc::new(Mutex::new(None)),
            process_id: Arc::new(RwLock::new(None)),
            created_at: Instant::now(),
            finished_at: Arc::new(RwLock::new(None)),
            stdin_rx: None,
        }
    }

    /// Set the stdin receiver for the task, enabling asynchronous input
    ///
    /// Has no effect if `enable_stdin` is false in the configuration
    pub fn set_stdin(mut self, stdin_rx: mpsc::Receiver<String>) -> Self {
        if self.config.enable_stdin.unwrap_or_default() {
            self.stdin_rx = Some(stdin_rx);
        }
        self
    }

    /// Get the current state of the task
    pub async fn get_state(&self) -> TaskState {
        self.state.read().await.clone()
    }

    /// Check if the task is currently running
    pub async fn is_running(&self) -> bool {
        let state = self.state.read().await.clone();
        state == TaskState::Running
    }
    /// Check if the task is currently ready
    pub async fn is_ready(&self) -> bool {
        let state = self.state.read().await.clone();
        state == TaskState::Ready
    }

    /// Get the uptime of the task since creation
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get information about the task, including name, state, and uptime
    pub async fn get_task_info(&self) -> TaskInfo {
        TaskInfo {
            name: self.task_name.clone(),
            state: self.get_state().await,
            uptime: self.uptime(),
            created_at: self.created_at,
            finished_at: self.finished_at.read().await.clone(),
        }
    }

    /// Get the process ID of the running task (if any)
    pub async fn get_process_id(&self) -> Option<u32> {
        self.process_id.read().await.clone()
    }

    /// Update the state of the task
    pub(crate) async fn update_state(&self, new_state: TaskState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    /// Send a termination signal to the running task
    ///
    /// Returns an error if the signal cannot be sent
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn send_terminate_signal(
        &self,
        reason: TaskTerminateReason,
    ) -> Result<(), TaskError> {
        if let Some(tx) = self.terminate_tx.lock().await.take() {
            if tx.send(reason.clone()).is_err() {
                let msg = "Terminate channel closed while sending signal";
                #[cfg(feature = "tracing")]
                tracing::warn!(terminate_reason=?reason, msg);
                return Err(TaskError::Channel(msg.to_string()));
            }
        } else {
            let msg = "Terminate signal already sent or channel missing";
            #[cfg(feature = "tracing")]
            tracing::warn!(msg);
            return Err(TaskError::Channel(msg.to_string()));
        }

        Ok(())
    }
}

/// Waits for all spawned task handles to complete, with a timeout
///
/// Returns an error if any handle fails or times out
pub(crate) async fn join_all_handles(
    task_handles: &mut Vec<JoinHandle<()>>,
) -> Result<(), TaskError> {
    if task_handles.is_empty() {
        return Ok(());
    }

    let handles = std::mem::take(task_handles);
    let mut errors = Vec::new();

    for (_index, mut handle) in handles.into_iter().enumerate() {
        match timeout(Duration::from_secs(5), &mut handle).await {
            Ok(Ok(())) => {}
            Ok(Err(join_err)) => {
                let err_msg = format!("Handle [{}] join failed: {:?}", handle.id(), join_err);

                errors.push(err_msg);
            }
            Err(_) => {
                let err_msg = format!("Handle [{}] join timeout, aborting", handle.id());
                handle.abort(); // ensure it’s killed
                errors.push(err_msg);
            }
        }
    }

    if !errors.is_empty() {
        return Err(TaskError::Handle(format!(
            "Multiple task handles join failures: {}",
            errors.join("; ")
        )));
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio::time::sleep;

    use crate::tasks::{
        async_tokio::spawner::{TaskInfo, TaskSpawner},
        config::TaskConfig,
        error::TaskError,
        state::{TaskState, TaskTerminateReason},
    };

    #[tokio::test]
    async fn task_spawner_is_running_returns_true_when_state_running() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("running_task".to_string(), config);
        assert!(
            !spawner.is_running().await,
            "Should not be running initially"
        );
        spawner.update_state(TaskState::Running).await;
        assert!(spawner.is_running().await, "Should be running after update");
    }

    #[tokio::test]
    async fn task_spawner_is_ready_returns_true_when_state_ready() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("ready_task".to_string(), config);
        assert!(!spawner.is_ready().await, "Should not be ready initially");
        spawner.update_state(TaskState::Ready).await;
        assert!(spawner.is_ready().await, "Should be ready after update");
    }

    #[tokio::test]
    async fn task_spawner_initial_state_is_pending() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("pending_task".to_string(), config);
        let state = spawner.get_state().await;
        assert_eq!(state, TaskState::Pending, "Initial state should be Pending");
    }

    #[tokio::test]
    async fn task_spawner_update_state_changes_state() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("update_task".to_string(), config);
        spawner.update_state(TaskState::Running).await;
        let state = spawner.get_state().await;
        assert_eq!(
            state,
            TaskState::Running,
            "State should be Running after update"
        );
    }

    #[tokio::test]
    async fn task_spawner_uptime_increases_over_time() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("uptime_task".to_string(), config);
        let uptime1 = spawner.uptime();
        sleep(Duration::from_millis(20)).await;
        let uptime2 = spawner.uptime();
        assert!(uptime2 > uptime1, "Uptime should increase after sleep");
    }

    #[tokio::test]
    async fn task_spawner_get_task_info_returns_correct_info() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("info_task".to_string(), config);
        let info: TaskInfo = spawner.get_task_info().await;
        assert_eq!(info.name, "info_task");
        assert_eq!(info.state, TaskState::Pending);
        assert!(info.uptime >= Duration::ZERO);
    }

    #[tokio::test]
    async fn task_spawner_process_id_initially_none() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("process_id_task".to_string(), config);
        assert_eq!(spawner.get_process_id().await, None);
    }

    #[tokio::test]
    async fn task_spawner_stdin_disabled_ignores_channel() {
        let config = TaskConfig::new("echo").enable_stdin(false);
        let (_, rx) = mpsc::channel(100);

        let spawner = TaskSpawner::new("no_stdin".to_string(), config).set_stdin(rx);
        assert!(spawner.stdin_rx.is_none());
    }

    #[tokio::test]
    async fn task_spawner_send_terminate_signal_no_channel() {
        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("no_channel".to_string(), config);

        let result = spawner
            .send_terminate_signal(TaskTerminateReason::Cleanup)
            .await;
        assert!(result.is_err());
        if let Err(TaskError::Channel(msg)) = result {
            assert_eq!(msg, "Terminate signal already sent or channel missing");
        } else {
            panic!("Expected Channel error");
        }
    }

    // TODO: Consider adding serde support for TaskInfo, not skipping Instant fields
    #[cfg(feature = "serde")]
    #[tokio::test]
    async fn task_info_serde() {
        use serde_json;

        let config = TaskConfig::new("echo");
        let spawner = TaskSpawner::new("serde_task".to_string(), config);
        let info = spawner.get_task_info().await;

        // This should work even with Instant fields skipped
        let serialized = serde_json::to_string(&info).unwrap();
        println!("Serialized TaskInfo: {}", serialized);
        assert!(serialized.contains("serde_task"));
        assert!(serialized.contains("Pending"));
    }
}

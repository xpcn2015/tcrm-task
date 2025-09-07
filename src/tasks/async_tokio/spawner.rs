use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout};

use crate::tasks::error::TaskError;
use crate::tasks::state::TaskTerminateReason;
use crate::tasks::{config::TaskConfig, state::TaskState};

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub name: String,
    pub state: TaskState,
    pub uptime: Duration,
    pub created_at: Instant,
    pub finished_at: Option<Instant>,
}
/// Spawns and manages the lifecycle of a task
#[derive(Debug)]
pub struct TaskSpawner {
    pub(crate) config: TaskConfig,
    pub(crate) task_name: String,
    pub(crate) state: Arc<RwLock<TaskState>>,
    pub(crate) terminate_tx: Arc<Mutex<Option<oneshot::Sender<TaskTerminateReason>>>>,
    pub(crate) process_id: Option<u32>,
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
            process_id: None,
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

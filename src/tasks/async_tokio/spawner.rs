use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout};
use tracing::{instrument, warn};

use crate::tasks::error::TaskError;
use crate::tasks::state::TaskTerminateReason;
use crate::tasks::{config::TaskConfig, state::TaskState};

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub name: String,
    pub state: TaskState,
    pub uptime: Duration,
}
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

    /// Sets the stdin receiver for the task.
    ///
    /// This allows the task to receive input asynchronously.
    ///
    /// Has no effect if `enable_stdin` in the task configuration is `false`.
    pub fn set_stdin(mut self, stdin_rx: mpsc::Receiver<String>) -> Self {
        if self.config.enable_stdin.unwrap_or_default() {
            self.stdin_rx = Some(stdin_rx);
        }
        self
    }

    /// Get task current state
    pub async fn get_state(&self) -> TaskState {
        self.state.read().await.clone()
    }

    /// Check if task is still running
    pub async fn is_running(&self) -> bool {
        let state = self.state.read().await.clone();
        matches!(state, TaskState::Running | TaskState::Ready)
    }

    /// Get task uptime
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub async fn get_task_info(&self) -> TaskInfo {
        TaskInfo {
            name: self.task_name.clone(),
            state: self.get_state().await,
            uptime: self.uptime(),
        }
    }

    pub async fn update_state(&self, new_state: TaskState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }
    #[instrument[skip_all]]
    pub async fn send_terminate_signal(
        &self,
        reason: TaskTerminateReason,
    ) -> Result<(), TaskError> {
        if let Some(tx) = self.terminate_tx.lock().await.take() {
            if tx.send(reason.clone()).is_err() {
                warn!(terminate_reason=?reason, "Terminate channel closed while sending signal");
                return Err(TaskError::Thread(
                    "Terminate channel closed while sending signal".to_string(),
                ));
            }
        } else {
            warn!("Terminate signal already sent or channel missing");
            return Err(TaskError::Thread(
                "Terminate signal already sent or channel missing".to_string(),
            ));
        }

        Ok(())
    }
}

/// Wait for all spawned threads to complete
pub async fn join_all_handles(task_handles: &mut Vec<JoinHandle<()>>) -> Result<(), TaskError> {
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
        return Err(TaskError::Thread(format!(
            "Multiple task join failures: {}",
            errors.join("; ")
        )));
    }

    Ok(())
}

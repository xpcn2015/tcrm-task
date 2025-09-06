use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
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
    pub(crate) terminate_tx: Option<mpsc::UnboundedSender<TaskTerminateReason>>,
    pub(crate) process_id: Option<u32>,
    pub(crate) created_at: Instant,
    pub(crate) finished_at: Arc<RwLock<Option<Instant>>>,
}

impl TaskSpawner {
    pub fn new(task_name: String, config: TaskConfig) -> Self {
        Self {
            task_name,
            config,
            state: Arc::new(RwLock::new(TaskState::Pending)),
            terminate_tx: None,
            process_id: None,
            created_at: Instant::now(),
            finished_at: Arc::new(RwLock::new(None)),
        }
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
        let tx = match &self.terminate_tx {
            Some(tx) => tx,
            None => {
                warn!(
                    task_name = self.task_name,
                    "No terminate tx, task might not started"
                );
                return Err(TaskError::MPSC(format!(
                    "No terminate tx on task [{}], task might not started",
                    self.task_name
                )));
            }
        };
        // Task already stopped if there is error
        if let Err(_) = tx.send(reason.clone()) {
            warn!(reason=?reason, "Terminate channel closed while sending signal");
        };
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

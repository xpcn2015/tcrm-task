use tokio::sync::mpsc;
use tracing::{error, instrument, warn};

use crate::tasks::async_tokio::direct::command::{setup_command, shell_command};
use crate::tasks::async_tokio::direct::watchers::output::spawn_output_watchers;
use crate::tasks::async_tokio::direct::watchers::result::spawn_result_watcher;
use crate::tasks::async_tokio::direct::watchers::timeout::spawn_timeout_watcher;
use crate::tasks::async_tokio::direct::watchers::wait::spawn_wait_watcher;
use crate::tasks::async_tokio::spawner::TaskSpawner;
use crate::tasks::error::TaskError;
use crate::tasks::event::{TaskEvent, TaskEventStopReason};
use crate::tasks::state::{TaskState, TaskTerminateReason};

impl TaskSpawner {
    #[instrument(skip(self, event_tx), fields(task_name = %self.task_name))]
    pub async fn start_direct(
        &mut self,
        event_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<u32, TaskError> {
        self.update_state(TaskState::Initiating).await;

        self.config.validate()?;

        let mut cmd = shell_command(&self.config)?;
        let mut cmd = cmd.kill_on_drop(true);

        setup_command(&mut cmd, &self.config)?;
        let mut child = cmd.spawn()?;
        let child_id = match child.id() {
            Some(id) => id,
            None => {
                error!("Failed to get process id");
                return Err(TaskError::Custom("Failed to get process id".to_string()));
            }
        };
        self.process_id = Some(child_id);
        let mut task_handles = vec![];
        self.update_state(TaskState::Running).await;
        if let Err(_) = event_tx
            .send(TaskEvent::Started {
                task_name: self.task_name.clone(),
            })
            .await
        {
            warn!("Event channel closed while sending TaskEvent::Started");
        }

        // Handle stdout and stderr
        let handles = spawn_output_watchers(self.task_name.clone(), event_tx.clone(), &mut child);
        task_handles.extend(handles);

        let (result_tx, result_rx) = mpsc::channel::<(Option<i32>, TaskEventStopReason)>(1);
        let (terminate_tx, terminate_rx) = mpsc::unbounded_channel::<TaskTerminateReason>();
        self.terminate_tx = Some(terminate_tx.clone());
        let handle = spawn_wait_watcher(
            self.task_name.clone(),
            self.state.clone(),
            child,
            terminate_rx,
            result_tx,
        );
        task_handles.push(handle);

        // Spawn timeout watcher if configured
        if let Some(timeout_ms) = self.config.timeout_ms {
            let handle = spawn_timeout_watcher(terminate_tx.clone(), timeout_ms);
            task_handles.push(handle);
        }

        // Spawn result watcher
        let _handle = spawn_result_watcher(
            self.task_name.clone(),
            self.state.clone(),
            self.finished_at.clone(),
            event_tx,
            result_rx,
            task_handles,
        );

        Ok(child_id)
    }
}

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout};

use crate::tasks::error::TaskError;
use crate::tasks::state::TaskTerminateReason;
use crate::tasks::{config::TaskConfig, state::TaskState};

/// Information about a running or completed task
///
/// Provides metadata about the task execution including timing, state, and lifecycle information.
///
/// # Examples
///
/// ```rust
/// use tcrm_task::tasks::async_tokio::spawner::{TaskSpawner, TaskInfo};
/// use tcrm_task::tasks::config::TaskConfig;
///
/// #[tokio::main]
/// async fn main() {
///     let config = TaskConfig::new("cmd").args(["/C", "echo", "hello"]);
///     let spawner = TaskSpawner::new("test".to_string(), config);
///     
///     let info: TaskInfo = spawner.get_task_info().await;
///     println!("Task {} is in state {:?}", info.name, info.state);
/// }
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Name of the task
    pub name: String,
    /// Current execution state
    pub state: TaskState,
    /// How long the task has been running
    pub uptime: Duration,
    /// When the task was created
    #[cfg_attr(feature = "serde", serde(skip, default = "default_instant"))]
    pub created_at: Instant,
    /// When the task finished (if completed)
    #[cfg_attr(feature = "serde", serde(skip, default))]
    pub finished_at: Option<Instant>,
}

#[cfg(feature = "serde")]
/// Returns the current instant for serde default value.
fn default_instant() -> Instant {
    Instant::now()
}

/// Spawns and manages the lifecycle of a task
///
/// `TaskSpawner` handles the execution of system processes with comprehensive
/// monitoring, state management, and event emission. It provides both
/// synchronous and asynchronous interfaces for process management.
///
/// # Features
///
/// - **State Management**: Track task execution through Pending, Running, Ready, and Finished states
/// - **Event Emission**: Real-time events for output, state changes, and lifecycle events
/// - **Timeout Handling**: Automatic termination when tasks exceed configured timeouts
/// - **Stdin Support**: Send input to running processes when enabled
/// - **Ready Detection**: Automatic detection when long-running processes are ready
/// - **Process Control**: Start, stop, and terminate processes with proper cleanup
///
/// # Examples
///
/// ## Simple Command Execution
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd")
///         .args(["/C", "echo", "Hello World"]);
///
///     let (tx, mut rx) = mpsc::channel(100);
///     let mut spawner = TaskSpawner::new("hello".to_string(), config);
///     
///     spawner.start_direct(tx).await?;
///
///     // Process events
///     while let Some(event) = rx.recv().await {
///         println!("Event: {:?}", event);
///         if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
///             break;
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Long-running Process with Ready Detection
/// ```rust
/// use tcrm_task::tasks::{config::{TaskConfig, StreamSource}, async_tokio::spawner::TaskSpawner};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd")
///         .args(["/C", "echo", "Server listening"])
///         .ready_indicator("Server listening")
///         .ready_indicator_source(StreamSource::Stdout)
///         .timeout_ms(30000);
///
///     let (tx, mut rx) = mpsc::channel(100);
///     let mut spawner = TaskSpawner::new("server".to_string(), config);
///     
///     spawner.start_direct(tx).await?;
///
///     // Wait for ready event
///     while let Some(event) = rx.recv().await {
///         if matches!(event, tcrm_task::tasks::event::TaskEvent::Ready { .. }) {
///             println!("Server is ready to accept requests!");
///             // Server is now ready, can start sending requests
///             break;
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Interactive Process with Stdin
/// ```rust
/// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskConfig::new("cmd")
///         .args(["/C", "echo", "Hello"])
///         .enable_stdin(true);
///
///     let (tx, mut rx) = mpsc::channel(100);
///     let (stdin_tx, stdin_rx) = mpsc::channel(10);
///     let mut spawner = TaskSpawner::new("cmd".to_string(), config);
///     
///     // Set up stdin channel - note: set_stdin consumes self and returns Self
///     spawner = spawner.set_stdin(stdin_rx);
///     
///     spawner.start_direct(tx).await?;
///
///     // Send input to the process
///     stdin_tx.send("print('Hello from stdin!')".to_string()).await?;
///     stdin_tx.send("exit()".to_string()).await?;
///
///     // Process events
///     while let Some(event) = rx.recv().await {
///         match event {
///             tcrm_task::tasks::event::TaskEvent::Output { line, .. } => {
///                 println!("Output: {}", line);
///             }
///             tcrm_task::tasks::event::TaskEvent::Stopped { .. } => break,
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
/// ```
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
    ///
    /// Creates a new `TaskSpawner` instance in the Pending state. The configuration
    /// is not validated until `start_direct` is called.
    ///
    /// # Arguments
    ///
    /// * `task_name` - Unique identifier for this task instance
    /// * `config` - Task configuration defining command, arguments, environment, etc.
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
    ///
    /// let config = TaskConfig::new("echo").args(["hello"]);
    /// let spawner = TaskSpawner::new("my-task".to_string(), config);
    /// ```
    #[must_use] pub fn new(task_name: String, config: TaskConfig) -> Self {
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
    /// Configures a channel for sending input to the process stdin. This method
    /// has no effect if `enable_stdin` is false in the task configuration.
    ///
    /// # Arguments
    ///
    /// * `stdin_rx` - Receiver channel for stdin input strings
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
    /// use tokio::sync::mpsc;
    ///
    /// let config = TaskConfig::new("python")
    ///     .args(["-i"])
    ///     .enable_stdin(true);
    ///
    /// let (stdin_tx, stdin_rx) = mpsc::channel(10);
    /// let spawner = TaskSpawner::new("interactive".to_string(), config)
    ///     .set_stdin(stdin_rx);
    /// ```
    #[must_use] pub fn set_stdin(mut self, stdin_rx: mpsc::Receiver<String>) -> Self {
        if self.config.enable_stdin.unwrap_or_default() {
            self.stdin_rx = Some(stdin_rx);
        }
        self
    }

    /// Get the current state of the task
    ///
    /// Returns the current execution state of the task. States progress through:
    /// Pending → Initiating → Running → (Ready) → Finished
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner, state::TaskState};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = TaskConfig::new("echo");
    ///     let spawner = TaskSpawner::new("test".to_string(), config);
    ///     
    ///     assert_eq!(spawner.get_state().await, TaskState::Pending);
    /// }
    /// ```
    pub async fn get_state(&self) -> TaskState {
        self.state.read().await.clone()
    }

    /// Check if the task is currently running
    ///
    /// Returns true if the task state is Running, false otherwise.
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = TaskConfig::new("echo");
    ///     let spawner = TaskSpawner::new("test".to_string(), config);
    ///     
    ///     assert!(!spawner.is_running().await); // Not running initially
    /// }
    /// ```
    pub async fn is_running(&self) -> bool {
        let state = self.state.read().await.clone();
        state == TaskState::Running
    }

    /// Check if the task is currently ready
    ///
    /// Returns true if the task state is Ready, false otherwise.
    /// The Ready state indicates a long-running process has signaled it's
    /// ready to accept requests (via the ready indicator).
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::{TaskConfig, StreamSource}, async_tokio::spawner::TaskSpawner};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = TaskConfig::new("my-server")
    ///         .ready_indicator("Server ready")
    ///         .ready_indicator_source(StreamSource::Stdout);
    ///     let spawner = TaskSpawner::new("server".to_string(), config);
    ///     
    ///     assert!(!spawner.is_ready().await); // Not ready initially
    /// }
    /// ```
    pub async fn is_ready(&self) -> bool {
        let state = self.state.read().await.clone();
        state == TaskState::Ready
    }

    /// Get the uptime of the task since creation
    ///
    /// Returns the duration since the `TaskSpawner` was created, regardless
    /// of the current execution state.
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = TaskConfig::new("echo");
    ///     let spawner = TaskSpawner::new("test".to_string(), config);
    ///     
    ///     let uptime = spawner.uptime();
    ///     assert!(uptime < Duration::from_secs(1)); // Just created
    /// }
    /// ```
    #[must_use] pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get comprehensive information about the task
    ///
    /// Returns a `TaskInfo` struct containing the task name, current state,
    /// uptime, creation time, and completion time (if finished).
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = TaskConfig::new("echo").args(["hello"]);
    ///     let spawner = TaskSpawner::new("info-test".to_string(), config);
    ///     
    ///     let info = spawner.get_task_info().await;
    ///     println!("Task '{}' has been running for {:?}", info.name, info.uptime);
    /// }
    /// ```
    pub async fn get_task_info(&self) -> TaskInfo {
        TaskInfo {
            name: self.task_name.clone(),
            state: self.get_state().await,
            uptime: self.uptime(),
            created_at: self.created_at,
            finished_at: *self.finished_at.read().await,
        }
    }

    /// Get the process ID of the running task (if any)
    ///
    /// Returns the system process ID if the task is currently running,
    /// or None if the task hasn't started or has finished.
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = TaskConfig::new("cmd").args(["/C", "ping", "127.0.0.1", "-n", "2"]);
    ///     let mut spawner = TaskSpawner::new("pid-test".to_string(), config);
    ///     
    ///     assert_eq!(spawner.get_process_id().await, None); // Not started yet
    ///     
    ///     let (tx, _rx) = mpsc::channel(100);
    ///     spawner.start_direct(tx).await?;
    ///     
    ///     // Now should have a process ID
    ///     let pid = spawner.get_process_id().await;
    ///     assert!(pid.is_some());
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_process_id(&self) -> Option<u32> {
        *self.process_id.read().await
    }

    /// Update the state of the task
    ///
    /// Internal method used by the spawner to update task state during execution.
    pub(crate) async fn update_state(&self, new_state: TaskState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    /// Send a termination signal to the running task
    ///
    /// Requests graceful termination of the running process with the specified reason.
    /// The process may take some time to respond to the termination signal.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for termination (Timeout, Cleanup, etc.)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the termination signal was sent successfully
    /// - `Err(TaskError::Channel)` if the signal could not be sent
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::{
    ///     config::TaskConfig,
    ///     async_tokio::spawner::TaskSpawner,
    ///     state::TaskTerminateReason
    /// };
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = TaskConfig::new("cmd").args(["/C", "ping", "127.0.0.1", "-n", "10"]); // Long-running task
    ///     let mut spawner = TaskSpawner::new("terminate-test".to_string(), config);
    ///     
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///     spawner.start_direct(tx).await?;
    ///     
    ///     // Wait a bit, then terminate
    ///     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    ///     spawner.send_terminate_signal(TaskTerminateReason::Cleanup).await?;
    ///     
    ///     // Process events until stopped
    ///     while let Some(event) = rx.recv().await {
    ///         if matches!(event, tcrm_task::tasks::event::TaskEvent::Stopped { .. }) {
    ///             break;
    ///         }
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
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

    for mut handle in handles {
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

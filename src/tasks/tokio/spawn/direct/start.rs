use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

use crate::tasks::error::TaskError;
use crate::tasks::event::{TaskEvent, TaskStopReason, TaskTerminateReason};
use crate::tasks::state::TaskState;
use crate::tasks::tokio::spawn::direct::command::setup_command;
use crate::tasks::tokio::spawn::direct::watchers::input::spawn_stdin_watcher;
use crate::tasks::tokio::spawn::direct::watchers::output::spawn_output_watchers;
use crate::tasks::tokio::spawn::direct::watchers::result::spawn_result_watcher;
use crate::tasks::tokio::spawn::direct::watchers::timeout::spawn_timeout_watcher;
use crate::tasks::tokio::spawn::direct::watchers::wait::spawn_wait_watcher;
use crate::tasks::tokio::spawn::process_group::ProcessGroup;
use crate::tasks::tokio::spawn::spawner::TaskSpawner;

impl TaskSpawner {
    /// Start the task and execute it directly with real-time event monitoring
    ///
    /// Validates the configuration, spawns the process, and sets up comprehensive monitoring
    /// including output capture, timeout handling, stdin support, and ready detection.
    /// Events are sent through the provided channel as the task executes.
    ///
    /// # Process Lifecycle
    ///
    /// 1. **Validation**: Configuration is validated for security and correctness
    /// 2. **Process Spawn**: System process is created with configured parameters
    /// 3. **Monitoring Setup**: Watchers are spawned for stdout/stderr, stdin, timeouts, and process completion
    /// 4. **Event Emission**: Real-time events are sent as the process executes
    /// 5. **Cleanup**: Process and resources are cleaned up when execution completes
    ///
    /// # Arguments
    ///
    /// * `event_tx` - Channel sender for receiving task events in real-time
    ///
    /// # Returns
    ///
    /// - `Ok(process_id)` - The system process ID if the task was started successfully
    /// - `Err(TaskError)` - Configuration validation error, spawn failure, or other issues
    ///
    /// # Events Emitted
    ///
    /// - `TaskEvent::Started` - Process has been spawned and is running
    /// - `TaskEvent::Output` - Output line received from stdout/stderr
    /// - `TaskEvent::Ready` - Ready indicator detected (for long-running processes)
    /// - `TaskEvent::Stopped` - Process has completed with exit code and reason
    /// - `TaskEvent::Error` - An error occurred during execution
    ///
    /// # Examples
    ///
    /// ## Simple Command
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, tokio::spawn::spawner::TaskSpawner};
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = TaskConfig::new("cmd").args(["/C", "echo", "Hello, World!"]);
    ///     let mut spawner = TaskSpawner::new("greeting".to_string(), config);
    ///     
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///     let process_id = spawner.start_direct(tx).await?;
    ///     println!("Started process with ID: {}", process_id);
    ///
    ///     // Process all events until completion
    ///     while let Some(event) = rx.recv().await {
    ///         match event {
    ///             tcrm_task::tasks::event::TaskEvent::Output { line, .. } => {
    ///                 println!("Output: {}", line);
    ///             }
    ///             tcrm_task::tasks::event::TaskEvent::Stopped { exit_code, .. } => {
    ///                 println!("Process finished with exit code: {:?}", exit_code);
    ///                 break;
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## Long-running Process with Ready Detection
    /// ```rust
    /// use tcrm_task::tasks::{
    ///     config::{TaskConfig, StreamSource},
    ///     tokio::spawn::spawner::TaskSpawner,
    ///     event::TaskEvent
    /// };
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = TaskConfig::new("cmd")
    ///         .args(["/C", "echo", "Server listening on"])
    ///         .ready_indicator("Server listening on")
    ///         .ready_indicator_source(StreamSource::Stdout)
    ///         .timeout_ms(30000); // 30 second timeout
    ///
    ///     let mut spawner = TaskSpawner::new("web-server".to_string(), config);
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///     
    ///     spawner.start_direct(tx).await?;
    ///
    ///     // Wait for the server to be ready
    ///     while let Some(event) = rx.recv().await {
    ///         match event {
    ///             TaskEvent::Ready { task_name } => {
    ///                 println!("Server '{}' is ready to accept requests!", task_name);
    ///                 // Now you can start sending requests to the server
    ///                 break;
    ///             }
    ///             TaskEvent::Error { error, .. } => {
    ///                 eprintln!("Server failed to start: {}", error);
    ///                 return Err(error.into());
    ///             }
    ///             TaskEvent::Output { line, .. } => {
    ///                 println!("Server log: {}", line);
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## Interactive Process with Stdin
    /// ```rust
    /// use tcrm_task::tasks::{config::TaskConfig, tokio::spawn::spawner::TaskSpawner};
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = TaskConfig::new("python")
    ///         .args(["-i"])  // Interactive mode
    ///         .enable_stdin(true);
    ///
    ///     let (stdin_tx, stdin_rx) = mpsc::channel(10);
    ///     let mut spawner = TaskSpawner::new("python-repl".to_string(), config)
    ///         .set_stdin(stdin_rx);
    ///     
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///     spawner.start_direct(tx).await?;
    ///
    ///     // Send some Python commands
    ///     stdin_tx.send("print('Hello from Python!')".to_string()).await?;
    ///     stdin_tx.send("2 + 2".to_string()).await?;
    ///     stdin_tx.send("exit()".to_string()).await?;
    ///
    ///     // Process output
    ///     while let Some(event) = rx.recv().await {
    ///         match event {
    ///             tcrm_task::tasks::event::TaskEvent::Output { line, .. } => {
    ///                 println!("Python: {}", line);
    ///             }
    ///             tcrm_task::tasks::event::TaskEvent::Stopped { .. } => break,
    ///             _ => {}
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Validation
    ///
    /// This method validates the configuration before execution
    ///
    /// # Watchers
    ///
    /// The method spawns multiple async watchers for different aspects of process monitoring:
    /// - Output watchers (stdout/stderr)
    /// - Stdin watcher (if enabled)
    /// - Timeout watcher (if configured)
    /// - Process completion watcher
    /// - Result aggregation watcher
    ///
    /// All watchers run concurrently for responsiveness.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError`] if:
    /// - Task configuration validation fails
    /// - Process fails to start due to invalid command or working directory
    /// - Unable to obtain process ID from started child process
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, event_tx)))]
    #[allow(clippy::too_many_lines)]
    pub async fn start_direct(
        &mut self,
        event_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<u32, TaskError> {
        self.update_state(TaskState::Initiating).await;

        match self.config.validate() {
            Ok(()) => {}
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Invalid task configuration");

                self.update_state(TaskState::Finished).await;
                let error_event = TaskEvent::Error { error: e.clone() };

                if (event_tx.send(error_event).await).is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Event channel closed while sending TaskEvent::Error");
                }
                return Err(e);
            }
        }

        let mut cmd = Command::new(&self.config.command);
        cmd.kill_on_drop(true);

        setup_command(&mut cmd, &self.config);

        // Conditionally create process group for cross-platform process tree management
        let (mut configured_cmd, process_group) = if self
            .config
            .use_process_group
            .unwrap_or_default()
        {
            match ProcessGroup::create_with_command(cmd) {
                Ok((cmd, group)) => (cmd, Some(group)),
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %e, "Failed to create process group");

                    self.update_state(TaskState::Finished).await;
                    let error_event = TaskEvent::Error {
                        error: TaskError::Handle(format!("Failed to create process group: {}", e)),
                    };

                    if (event_tx.send(error_event).await).is_err() {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("Event channel closed while sending TaskEvent::Error");
                    }

                    return Err(TaskError::Handle(format!(
                        "Failed to create process group: {}",
                        e
                    )));
                }
            }
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Process group management disabled by configuration");
            (cmd, None)
        };

        let mut child = match configured_cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Failed to spawn child process");

                self.update_state(TaskState::Finished).await;
                let error_event = TaskEvent::Error {
                    error: TaskError::IO(e.to_string()),
                };

                if (event_tx.send(error_event).await).is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Event channel closed while sending TaskEvent::Error");
                }

                return Err(TaskError::IO(e.to_string()));
            }
        };
        self.running_at = Some(Instant::now());

        // Assign the child process to the process group if enabled
        if let Some(ref pg) = process_group {
            if let Err(e) = pg.assign_child(&child).await {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "Failed to assign child to process group");

                self.update_state(TaskState::Finished).await;
                let error_event = TaskEvent::Error {
                    error: TaskError::Handle(format!(
                        "Failed to assign child to process group: {}",
                        e
                    )),
                };

                if (event_tx.send(error_event).await).is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Event channel closed while sending TaskEvent::Error");
                }

                return Err(TaskError::Handle(format!(
                    "Failed to assign child to process group: {}",
                    e
                )));
            }
        }
        let Some(child_id) = child.id() else {
            let msg = "Failed to get process id";

            #[cfg(feature = "tracing")]
            tracing::error!(msg);

            self.update_state(TaskState::Finished).await;
            let error_event = TaskEvent::Error {
                error: TaskError::Handle(msg.to_string()),
            };

            if (event_tx.send(error_event).await).is_err() {
                #[cfg(feature = "tracing")]
                tracing::warn!("Event channel closed while sending TaskEvent::Error");
            }

            return Err(TaskError::Handle(msg.to_string()));
        };
        *self.process_id.write().await = Some(child_id);
        let mut task_handles = vec![];
        self.update_state(TaskState::Running).await;
        if (event_tx.send(TaskEvent::Started).await).is_err() {
            #[cfg(feature = "tracing")]
            tracing::warn!("Event channel closed while sending TaskEvent::Started");
        }

        let (result_tx, result_rx) = oneshot::channel::<(Option<i32>, TaskStopReason)>();
        let (terminate_tx, terminate_rx) = oneshot::channel::<TaskTerminateReason>();
        let (handle_terminator_tx, handle_terminator_rx) = watch::channel(false);

        // Spawn stdout and stderr watchers
        let handles = spawn_output_watchers(
            self.state.clone(),
            event_tx.clone(),
            &mut child,
            handle_terminator_rx.clone(),
            self.config.ready_indicator.clone(),
            self.config.ready_indicator_source.clone(),
        );
        task_handles.extend(handles);

        // Spawn stdin watcher if configured
        if let Some((stdin, stdin_rx)) = child.stdin.take().zip(self.stdin_rx.take()) {
            let handle = spawn_stdin_watcher(stdin, stdin_rx, handle_terminator_rx.clone());
            task_handles.push(handle);
        }

        // Spawn child wait watcher
        *self.terminate_tx.lock().await = Some(terminate_tx);

        let handle = spawn_wait_watcher(
            self.state.clone(),
            child,
            process_group,
            terminate_rx,
            handle_terminator_tx.clone(),
            result_tx,
            self.process_id.clone(),
        );
        task_handles.push(handle);

        // Spawn timeout watcher if configured
        if let Some(timeout_ms) = self.config.timeout_ms {
            let handle =
                spawn_timeout_watcher(self.terminate_tx.clone(), timeout_ms, handle_terminator_rx);
            task_handles.push(handle);
        }

        // Spawn result watcher
        let _handle = spawn_result_watcher(
            self.state.clone(),
            self.finished_at.clone(),
            event_tx,
            result_rx,
            task_handles,
        );

        Ok(child_id)
    }
}

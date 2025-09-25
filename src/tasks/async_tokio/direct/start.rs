use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

use crate::tasks::async_tokio::direct::command::setup_command;
use crate::tasks::async_tokio::direct::watchers::input::spawn_stdin_watcher;
use crate::tasks::async_tokio::direct::watchers::output::spawn_output_watchers;
use crate::tasks::async_tokio::direct::watchers::result::spawn_result_watcher;
use crate::tasks::async_tokio::direct::watchers::timeout::spawn_timeout_watcher;
use crate::tasks::async_tokio::direct::watchers::wait::spawn_wait_watcher;
use crate::tasks::async_tokio::process_group::ProcessGroup;
use crate::tasks::async_tokio::spawner::TaskSpawner;
use crate::tasks::error::TaskError;
use crate::tasks::event::{TaskEvent, TaskEventStopReason, TaskTerminateReason};
use crate::tasks::state::TaskState;

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
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
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
    ///     async_tokio::spawner::TaskSpawner,
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
    /// use tcrm_task::tasks::{config::TaskConfig, async_tokio::spawner::TaskSpawner};
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
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, event_tx), fields(task_name = %self.task_name)))]
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
                let error_event = TaskEvent::Error {
                    task_name: self.task_name.clone(),
                    error: e.clone(),
                };

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
        let (mut configured_cmd, process_group) = if self.config.is_process_group_enabled() {
            match ProcessGroup::create_with_command(cmd) {
                Ok((cmd, group)) => (cmd, Some(group)),
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %e, "Failed to create process group");

                    self.update_state(TaskState::Finished).await;
                    let error_event = TaskEvent::Error {
                        task_name: self.task_name.clone(),
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
                    task_name: self.task_name.clone(),
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
                    task_name: self.task_name.clone(),
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
                task_name: self.task_name.clone(),
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
        if (event_tx
            .send(TaskEvent::Started {
                task_name: self.task_name.clone(),
            })
            .await)
            .is_err()
        {
            #[cfg(feature = "tracing")]
            tracing::warn!("Event channel closed while sending TaskEvent::Started");
        }

        let (result_tx, result_rx) = oneshot::channel::<(Option<i32>, TaskEventStopReason)>();
        let (terminate_tx, terminate_rx) = oneshot::channel::<TaskTerminateReason>();
        let (handle_terminator_tx, handle_terminator_rx) = watch::channel(false);

        // Spawn stdout and stderr watchers
        let handles = spawn_output_watchers(
            self.task_name.clone(),
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
            self.task_name.clone(),
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

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn start_direct_ready_indicator_source_stdout() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "Write-Output 'READY_INDICATOR'"])
            .ready_indicator("READY_INDICATOR".to_string())
            .ready_indicator_source(StreamSource::Stdout);
        #[cfg(unix)]
        let config = TaskConfig::new("echo")
            .args(["READY_INDICATOR"])
            .ready_indicator("READY_INDICATOR".to_string())
            .ready_indicator_source(StreamSource::Stdout)
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("ready_stdout_task".to_string(), config);
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        let mut ready_event = false;
        while let Some(event) = rx.recv().await {
            if let TaskEvent::Ready { task_name } = event {
                assert_eq!(task_name, "ready_stdout_task");
                ready_event = true;
            }
        }
        assert!(
            ready_event,
            "Should emit Ready event when indicator is in stdout"
        );
    }

    #[tokio::test]
    async fn start_direct_ready_indicator_source_stderr() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "Write-Error 'READY_INDICATOR'"])
            .ready_indicator("READY_INDICATOR".to_string())
            .ready_indicator_source(StreamSource::Stderr);
        #[cfg(unix)]
        let config = TaskConfig::new("bash")
            .args(["-c", "echo READY_INDICATOR >&2"])
            .ready_indicator("READY_INDICATOR".to_string())
            .ready_indicator_source(StreamSource::Stderr)
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("ready_stderr_task".to_string(), config);
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        let mut ready_event = false;
        while let Some(event) = rx.recv().await {
            if let TaskEvent::Ready { task_name } = event {
                assert_eq!(task_name, "ready_stderr_task");
                ready_event = true;
            }
        }
        assert!(
            ready_event,
            "Should emit Ready event when indicator is in stderr"
        );
    }

    #[tokio::test]
    async fn start_direct_ready_indicator_source_mismatch() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "Write-Output 'READY_INDICATOR'"])
            .ready_indicator("READY_INDICATOR".to_string())
            .ready_indicator_source(StreamSource::Stderr);
        #[cfg(unix)]
        let config = TaskConfig::new("echo")
            .args(["READY_INDICATOR"])
            .ready_indicator("READY_INDICATOR".to_string())
            .ready_indicator_source(StreamSource::Stderr)
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("ready_mismatch_task".to_string(), config);
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        let mut ready_event = false;
        while let Some(event) = rx.recv().await {
            if let TaskEvent::Ready { .. } = event {
                ready_event = true;
            }
        }
        assert!(
            !ready_event,
            "Should NOT emit Ready event if indicator is in wrong stream"
        );
    }
    use tokio::sync::mpsc;

    use crate::tasks::{
        async_tokio::spawner::TaskSpawner,
        config::{StreamSource, TaskConfig},
        error::TaskError,
        event::{TaskEvent, TaskEventStopReason, TaskTerminateReason},
    };
    #[tokio::test]
    async fn start_direct_fn_echo_command() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell").args(["-Command", "echo hello"]);
        #[cfg(unix)]
        let config = TaskConfig::new("echo")
            .args(["hello"])
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("echo_task".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        let mut started = false;
        let mut stopped = false;
        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Started { task_name } => {
                    assert_eq!(task_name, "echo_task");
                    started = true;
                }
                TaskEvent::Output {
                    task_name,
                    line,
                    src,
                } => {
                    assert_eq!(task_name, "echo_task");
                    assert_eq!(line, "hello");
                    assert_eq!(src, StreamSource::Stdout);
                }
                TaskEvent::Stopped {
                    task_name,
                    exit_code,
                    reason: _,
                } => {
                    assert_eq!(task_name, "echo_task");
                    assert_eq!(exit_code, Some(0));
                    stopped = true;
                }
                _ => {}
            }
        }

        assert!(started);
        assert!(stopped);
    }
    #[tokio::test]
    async fn start_direct_timeout_terminated_task() {
        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "sleep 2"])
            .timeout_ms(1);
        #[cfg(unix)]
        let config = TaskConfig::new("sleep")
            .args(["2"])
            .timeout_ms(1)
            .use_process_group(false);

        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        let mut spawner = TaskSpawner::new("sleep_with_timeout_task".into(), config);

        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        let mut started = false;
        let mut stopped = false;
        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Started { task_name } => {
                    assert_eq!(task_name, "sleep_with_timeout_task");
                    started = true;
                }

                TaskEvent::Stopped {
                    task_name,
                    exit_code,
                    reason,
                } => {
                    assert_eq!(task_name, "sleep_with_timeout_task");
                    assert_eq!(exit_code, None);
                    assert_eq!(
                        reason,
                        TaskEventStopReason::Terminated(TaskTerminateReason::Timeout)
                    );
                    stopped = true;
                }
                _ => {}
            }
        }

        assert!(started);
        assert!(stopped);
    }

    #[tokio::test]
    async fn start_direct_fn_invalid_empty_command() {
        let (tx, _rx) = mpsc::channel::<TaskEvent>(1024);
        let config = TaskConfig::new(""); // invalid: empty command
        let mut spawner = TaskSpawner::new("bad_task".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

        // Ensure TaskState is Finished after error, not stalled at Initiating
        let state = spawner.get_state().await;
        assert_eq!(
            state,
            crate::tasks::state::TaskState::Finished,
            "TaskState should be Finished after error, not Initiating"
        );
    }

    #[tokio::test]
    async fn start_direct_fn_stdin_valid() {
        // Channel for receiving task events
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        let (stdin_tx, stdin_rx) = mpsc::channel::<String>(1024);

        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "$line = Read-Host; Write-Output $line"])
            .enable_stdin(true);
        #[cfg(unix)]
        let config = TaskConfig::new("head")
            .args(["-n", "1"])
            .enable_stdin(true)
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("stdin_task".to_string(), config).set_stdin(stdin_rx);

        // Spawn the task
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        // Send input via stdin if enabled
        stdin_tx.send("hello world".to_string()).await.unwrap();

        let mut started = false;
        let mut output_ok = false;
        let mut stopped = false;

        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Started { task_name } => {
                    assert_eq!(task_name, "stdin_task");
                    started = true;
                }
                TaskEvent::Output {
                    task_name,
                    line,
                    src,
                } => {
                    assert_eq!(task_name, "stdin_task");
                    assert_eq!(line, "hello world");
                    assert_eq!(src, StreamSource::Stdout);
                    output_ok = true;
                }
                TaskEvent::Stopped {
                    task_name,
                    exit_code,
                    ..
                } => {
                    assert_eq!(task_name, "stdin_task");
                    assert_eq!(exit_code, Some(0));
                    stopped = true;
                }
                _ => {}
            }
        }

        assert!(started);
        assert!(output_ok);
        assert!(stopped);
    }

    #[tokio::test]
    async fn start_direct_fn_stdin_ignore() {
        // Channel for receiving task events
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        let (stdin_tx, stdin_rx) = mpsc::channel::<String>(1024);

        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "$line = Read-Host; Write-Output $line"]);
        #[cfg(unix)]
        let config = TaskConfig::new("head")
            .args(["-n", "1"])
            .use_process_group(false);

        // Note: stdin is not enabled in config, so stdin should be ignored
        let mut spawner = TaskSpawner::new("stdin_task".to_string(), config).set_stdin(stdin_rx);

        // Spawn the task
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        // Send input, but it should be ignored (receiver will be dropped, so this should error)
        let send_result = stdin_tx.send("hello world".to_string()).await;
        assert!(
            send_result.is_err(),
            "Sending to stdin_tx should error because receiver is dropped"
        );

        let mut started = false;
        let mut output_found = false;
        let mut stopped = false;

        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Started { task_name } => {
                    assert_eq!(task_name, "stdin_task");
                    started = true;
                }
                TaskEvent::Output { .. } => {
                    // Should NOT receive output from stdin
                    output_found = true;
                }
                TaskEvent::Stopped {
                    task_name,
                    exit_code,
                    ..
                } => {
                    assert_eq!(task_name, "stdin_task");
                    assert_eq!(exit_code, Some(0));
                    stopped = true;
                }
                _ => {}
            }
        }

        assert!(started);
        assert!(
            !output_found,
            "Should not receive output from stdin when not enabled"
        );
        assert!(stopped);
    }

    // Error scenario tests
    #[tokio::test]
    async fn start_direct_command_not_found() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        let config = TaskConfig::new("non_existent_command").use_process_group(false);
        let mut spawner = TaskSpawner::new("error_task".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(matches!(result, Err(TaskError::IO(_))));

        if let Some(TaskEvent::Error { task_name, error }) = rx.recv().await {
            assert_eq!(task_name, "error_task");
            assert!(matches!(error, TaskError::IO(_)));
            if let TaskError::IO(msg) = error {
                #[cfg(windows)]
                assert!(msg.contains("not found") || msg.contains("cannot find"));
                #[cfg(unix)]
                assert!(msg.contains("No such file or directory"));
            }
        } else {
            panic!("Expected TaskEvent::Error");
        }
    }

    #[tokio::test]
    async fn start_direct_invalid_working_directory() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        let config = TaskConfig::new("echo").working_dir("/non/existent/directory");

        let mut spawner = TaskSpawner::new("working_dir_task".to_string(), config);

        let result = spawner.start_direct(tx).await;
        assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

        if let Some(TaskEvent::Error { task_name, error }) = rx.recv().await {
            assert_eq!(task_name, "working_dir_task");
            assert!(matches!(error, TaskError::InvalidConfiguration(_)));
        } else {
            panic!("Expected TaskEvent::Error");
        }
    }

    #[tokio::test]
    async fn start_direct_zero_timeout() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell")
            .args(["-Command", "Start-Sleep -Seconds 1"])
            .timeout_ms(0);
        #[cfg(unix)]
        let config = TaskConfig::new("sleep").args(["1"]).timeout_ms(0);

        let mut spawner = TaskSpawner::new("timeout_task".to_string(), config);

        // Zero timeout should be rejected as invalid configuration
        let result = spawner.start_direct(tx).await;
        assert!(matches!(result, Err(TaskError::InvalidConfiguration(_))));

        // Should receive an error event
        if let Some(TaskEvent::Error { task_name, error }) = rx.recv().await {
            assert_eq!(task_name, "timeout_task");
            assert!(matches!(error, TaskError::InvalidConfiguration(_)));
        } else {
            panic!("Expected TaskEvent::Error with InvalidConfiguration");
        }
    }

    #[tokio::test]
    async fn process_id_is_none_after_task_stopped() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell").args(["-Command", "echo done"]);
        #[cfg(unix)]
        let config = TaskConfig::new("echo")
            .args(["done"])
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("pid_test_task".to_string(), config);
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        let mut stopped = false;
        while let Some(event) = rx.recv().await {
            if let TaskEvent::Stopped { task_name, .. } = event {
                assert_eq!(task_name, "pid_test_task");
                stopped = true;
                break;
            }
        }
        assert!(stopped, "Task should emit Stopped event");
        // process_id should be None after stopped
        let pid = spawner.get_process_id().await;
        assert!(
            pid.is_none(),
            "process_id should be None after task is stopped"
        );
    }

    #[tokio::test]
    async fn process_id_is_some_while_task_running() {
        use std::time::Duration;
        use tokio::time::sleep;
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
        #[cfg(windows)]
        let config = TaskConfig::new("powershell").args(["-Command", "Start-Sleep -Seconds 2"]);
        #[cfg(unix)]
        let config = TaskConfig::new("sleep")
            .args(["2"])
            .use_process_group(false);

        let mut spawner = TaskSpawner::new("pid_running_task".to_string(), config);
        let result = spawner.start_direct(tx).await;
        assert!(result.is_ok());

        // Wait a short time to ensure the process is running
        sleep(Duration::from_millis(500)).await;
        let pid = spawner.get_process_id().await;
        assert!(
            pid.is_some(),
            "process_id should be Some while task is running"
        );

        // Drain events to ensure clean test exit
        while let Some(event) = rx.recv().await {
            if let TaskEvent::Stopped { .. } = event {
                break;
            }
        }
    }
}

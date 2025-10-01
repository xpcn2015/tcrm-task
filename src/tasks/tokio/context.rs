use std::{
    sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicU64},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::{Mutex, oneshot};

#[cfg(feature = "process-group")]
use crate::tasks::process::group::builder::ProcessGroup;
use crate::tasks::{
    config::TaskConfig,
    event::{TaskStopReason, TaskTerminateReason},
    state::TaskState,
};

/// Shared context for task execution state and configuration.
///
/// This structure holds all the runtime state information for a task,
/// including process information, timestamps, and synchronization primitives.
/// It's designed to be thread-safe and shared between async tasks.
#[derive(Debug)]
pub(crate) struct TaskExecutorContext {
    pub(crate) config: TaskConfig,
    state: AtomicU8,
    process_id: AtomicU32,
    created_at: AtomicU64,
    running_at: AtomicU64,
    finished_at: AtomicU64,
    exit_code: AtomicI32,
    exit_code_set: AtomicBool,
    stop_reason: Mutex<Option<TaskStopReason>>,
    ready_flag: AtomicBool,
    internal_terminate_tx: Mutex<Option<oneshot::Sender<TaskTerminateReason>>>,

    #[cfg(unix)]
    terminate_signal_code: AtomicI32,

    #[cfg(feature = "process-group")]
    pub(crate) group: Mutex<ProcessGroup>,
}
impl TaskExecutorContext {
    /// Creates a new task execution context.
    ///
    /// Initializes all state fields to their default values and records
    /// the current time as the creation timestamp.
    ///
    /// # Arguments
    ///
    /// * `config` - The task configuration to use
    ///
    /// # Returns
    ///
    /// A new `TaskExecutorContext` instance
    pub(crate) fn new(config: TaskConfig) -> Self {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self {
            config,
            state: AtomicU8::new(0),
            process_id: AtomicU32::new(0),
            created_at: AtomicU64::new(now_nanos),
            running_at: AtomicU64::new(0),
            finished_at: AtomicU64::new(0),
            exit_code: AtomicI32::new(0),
            exit_code_set: AtomicBool::new(false),
            stop_reason: Mutex::new(None),
            ready_flag: AtomicBool::new(false),
            internal_terminate_tx: Mutex::new(None),

            #[cfg(unix)]
            terminate_signal_code: AtomicI32::new(0),

            #[cfg(feature = "process-group")]
            group: Mutex::new(ProcessGroup::new()),
        }
    }

    /// Gets the termination signal code for Unix systems.
    ///
    /// Returns the signal code that was used to terminate the process,
    /// or None if no signal was received.
    ///
    /// # Returns
    ///
    /// * `Some(i32)` - The signal code that terminated the process
    /// * `None` - If no signal was received or signal code is 0
    #[cfg(unix)]
    pub(crate) fn get_terminate_signal_code(&self) -> Option<i32> {
        let code = self
            .terminate_signal_code
            .load(std::sync::atomic::Ordering::SeqCst);
        if code == 0 { None } else { Some(code) }
    }
    /// Sets the termination signal code for Unix systems.
    ///
    /// Stores the signal code that was used to terminate the process.
    ///
    /// # Arguments
    ///
    /// * `code` - The signal code to store, or None to clear it
    #[cfg(unix)]
    pub(crate) fn set_terminate_signal_code(&self, code: Option<i32>) {
        self.terminate_signal_code
            .store(code.unwrap_or(0), std::sync::atomic::Ordering::SeqCst);
    }

    /// Sends a termination signal through the internal oneshot channel.
    ///
    /// Attempts to send a termination reason through the internal oneshot
    /// channel if it exists and hasn't been used yet.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for termination
    pub(crate) async fn send_terminate_oneshot(&self, reason: TaskTerminateReason) {
        let mut guard = self.internal_terminate_tx.lock().await;
        if let Some(tx) = guard.take() {
            if tx.send(reason.clone()).is_err() {
                #[cfg(feature = "tracing")]
                tracing::warn!(terminate_reason=?reason, "Internal Terminate channel closed while sending signal");
            }
        } else {
            #[cfg(feature = "tracing")]
            tracing::warn!("Terminate signal already sent or channel missing");
        }
    }
    /// Sets the internal termination signal sender.
    ///
    /// Stores the oneshot sender that will be used to signal task termination.
    /// This is typically called during task setup.
    ///
    /// # Arguments
    ///
    /// * `tx` - The oneshot sender for termination signals
    pub(crate) async fn set_internal_terminate_tx(&self, tx: oneshot::Sender<TaskTerminateReason>) {
        let mut guard = self.internal_terminate_tx.lock().await;
        *guard = Some(tx);
    }

    /// Gets the ready flag indicating if the task has reached a ready state.
    ///
    /// Returns true if the ready indicator has been detected in the output.
    ///
    /// # Returns
    ///
    /// * `true` - If the ready indicator has been detected
    /// * `false` - If the task is not ready yet
    pub(crate) fn get_ready_flag(&self) -> bool {
        self.ready_flag.load(std::sync::atomic::Ordering::SeqCst)
    }
    /// Sets the ready flag indicating the task has reached a ready state.
    ///
    /// Called when the ready indicator is detected in the task output.
    ///
    /// # Arguments
    ///
    /// * `ready` - Whether the task is ready or not
    pub(crate) fn set_ready_flag(&self, ready: bool) {
        self.ready_flag
            .store(ready, std::sync::atomic::Ordering::SeqCst);
    }

    /// Gets the stop reason for the task.
    ///
    /// Returns the reason why the task was stopped, if it has been stopped.
    ///
    /// # Returns
    ///
    /// * `Some(TaskStopReason)` - The reason the task stopped if it has been stopped
    /// * `None` - If the task has not been stopped yet
    pub(crate) async fn get_stop_reason(&self) -> Option<TaskStopReason> {
        self.stop_reason.lock().await.clone()
    }

    /// Sets the stop reason for the task.
    ///
    /// Records the reason why the task is being stopped for future retrieval.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for stopping the task
    pub(crate) async fn set_stop_reason(&self, reason: TaskStopReason) {
        let mut guard = self.stop_reason.lock().await;
        *guard = Some(reason);
    }

    /// Gets the exit code of the finished task.
    ///
    /// Returns the process exit code if the task has finished execution.
    /// For terminated processes (timeout, manual termination), this returns `None`
    /// to avoid race conditions between termination and natural completion.
    ///
    /// # Exit Code Behavior
    ///
    /// - **Natural completion**: Returns `Some(exit_code)`
    /// - **Terminated processes**: Returns `None` (timeout, user termination, etc.)
    /// - **Running/Not started**: Returns `None`
    ///
    /// # Returns
    ///
    /// * `Some(i32)` - The exit code if the task completed naturally
    /// * `None` - If the task is not finished, was terminated, or exit code was not captured
    pub(crate) fn get_exit_code(&self) -> Option<i32> {
        let state = self.get_state();
        if state != TaskState::Finished {
            return None;
        }
        if self.exit_code_set.load(std::sync::atomic::Ordering::SeqCst) {
            let code = self.exit_code.load(std::sync::atomic::Ordering::SeqCst);
            Some(code)
        } else {
            None
        }
    }

    /// Sets the exit code for the task.
    ///
    /// Stores the process exit code when the task finishes execution.
    /// Setting `None` indicates the process was terminated and should not
    /// provide an exit code in the final `TaskEvent::Stopped` event.
    ///
    /// # Arguments
    ///
    /// * `code` - The exit code to store, or None if the process was terminated
    pub(crate) fn set_exit_code(&self, code: Option<i32>) {
        match code {
            Some(c) => {
                self.exit_code.store(c, std::sync::atomic::Ordering::SeqCst);
                self.exit_code_set
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            None => {
                self.exit_code_set
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    /// Gets the timestamp when the task started running.
    ///
    /// Returns the time when the task transitioned to the running state.
    ///
    /// # Returns
    ///
    /// * `Some(SystemTime)` - When the task started running
    /// * `None` - If the task hasn't started running yet
    pub(crate) fn get_running_at(&self) -> Option<SystemTime> {
        Self::get_time(&self.running_at)
    }

    /// Sets the running timestamp to the current time.
    ///
    /// Records the current time as when the task started running.
    ///
    /// # Returns
    ///
    /// The timestamp that was recorded
    pub(crate) fn set_running_at(&self) -> SystemTime {
        Self::set_time(&self.running_at)
    }

    /// Gets the timestamp when the task finished execution.
    ///
    /// Returns the time when the task completed or was terminated.
    ///
    /// # Returns
    ///
    /// * `Some(SystemTime)` - When the task finished
    /// * `None` - If the task hasn't finished yet
    pub(crate) fn get_finished_at(&self) -> Option<SystemTime> {
        Self::get_time(&self.finished_at)
    }

    /// Sets the finished timestamp to the current time.
    ///
    /// Records the current time as when the task finished execution.
    ///
    /// # Returns
    ///
    /// The timestamp that was recorded
    pub(crate) fn set_finished_at(&self) -> SystemTime {
        Self::set_time(&self.finished_at)
    }

    /// Gets the creation timestamp of the task context.
    ///
    /// Returns when this task context was initially created.
    ///
    /// # Returns
    ///
    /// The timestamp when the task context was created
    pub(crate) fn get_create_at(&self) -> SystemTime {
        let nanos = self.created_at.load(std::sync::atomic::Ordering::SeqCst);
        UNIX_EPOCH + std::time::Duration::from_nanos(nanos)
    }

    /// Gets the time value from an atomic storage.
    ///
    /// Reads a timestamp from atomic storage and converts it to SystemTime.
    /// Used internally for retrieving running_at and finished_at timestamps.
    ///
    /// # Arguments
    ///
    /// * `store` - Reference to the atomic storage containing the timestamp
    ///
    /// # Returns
    ///
    /// * `Some(SystemTime)` - The stored timestamp if it has been set
    /// * `None` - If no timestamp has been stored (value is 0)
    fn get_time(store: &AtomicU64) -> Option<SystemTime> {
        let nanos = store.load(std::sync::atomic::Ordering::SeqCst);
        if nanos == 0 {
            None
        } else {
            Some(UNIX_EPOCH + std::time::Duration::from_nanos(nanos))
        }
    }

    /// Sets the current time in atomic storage.
    ///
    /// Stores the current timestamp in atomic storage for thread-safe access.
    /// Used internally for setting running_at and finished_at timestamps.
    ///
    /// # Arguments
    ///
    /// * `store` - Reference to the atomic storage to update
    ///
    /// # Returns
    ///
    /// The current `SystemTime` that was stored
    fn set_time(store: &AtomicU64) -> SystemTime {
        let now = SystemTime::now();
        let nanos = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        store.store(nanos, std::sync::atomic::Ordering::SeqCst);
        now
    }

    /// Gets the process ID of the running task.
    ///
    /// Returns the operating system process ID if a process has been spawned.
    ///
    /// # Returns
    ///
    /// * `Some(u32)` - The process ID if a process is running
    /// * `None` - If no process has been spawned yet
    pub(crate) fn get_process_id(&self) -> Option<u32> {
        let process_id = self.process_id.load(std::sync::atomic::Ordering::SeqCst);
        if process_id == 0 {
            None
        } else {
            Some(process_id)
        }
    }

    /// Sets the process ID for the task.
    ///
    /// Stores the operating system process ID when a process is spawned.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID to store
    pub(crate) fn set_process_id(&self, pid: u32) {
        self.process_id
            .store(pid, std::sync::atomic::Ordering::SeqCst);
    }

    /// Gets the current state of the task.
    ///
    /// Returns the current execution state of the task.
    ///
    /// # Returns
    ///
    /// The current `TaskState` of the task
    pub(crate) fn get_state(&self) -> TaskState {
        self.state.load(std::sync::atomic::Ordering::SeqCst).into()
    }

    /// Sets the task state and returns the current timestamp.
    ///
    /// Updates the task's execution state and records when the change occurred.
    ///
    /// # Arguments
    ///
    /// * `new_state` - The new state to set
    ///
    /// # Returns
    ///
    /// The timestamp when the state change occurred
    pub(crate) fn set_state(&self, new_state: TaskState) -> SystemTime {
        self.state
            .store(new_state as u8, std::sync::atomic::Ordering::SeqCst);
        let now = SystemTime::now();
        let nanos = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        match new_state {
            TaskState::Running => {
                self.running_at
                    .store(nanos, std::sync::atomic::Ordering::SeqCst);
            }
            TaskState::Finished => {
                self.finished_at
                    .store(nanos, std::sync::atomic::Ordering::SeqCst);
            }
            _ => {}
        }
        now
    }
}

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::tasks::error::TaskError;
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct TaskConfig {
    /// The command or executable to run
    pub(crate) command: String,

    /// Arguments to pass to the command
    pub(crate) args: Option<Vec<String>>,

    /// Working directory for the command
    pub(crate) working_dir: Option<String>,

    /// Environment variables for the command
    pub(crate) env: Option<HashMap<String, String>>,

    /// Shell options if the command should run in a shell
    pub(crate) shell: Option<TaskShell>,

    /// Using pseudo-terminal to run this task
    pub(crate) pty: Option<bool>,

    /// Maximum allowed runtime in milliseconds
    pub(crate) timeout_ms: Option<u64>,

    /// Allow providing input to the task via stdin.
    pub(crate) enable_stdin: Option<bool>,
}

pub type SharedTaskConfig = Arc<TaskConfig>;
impl Default for TaskConfig {
    fn default() -> Self {
        TaskConfig {
            command: String::new(),
            args: None,
            working_dir: None,
            env: None,
            shell: Some(TaskShell::None),
            pty: Some(false),
            timeout_ms: None,
            enable_stdin: Some(false),
        }
    }
}
impl TaskConfig {
    pub fn new(command: impl Into<String>) -> Self {
        TaskConfig {
            command: command.into(),
            ..Default::default()
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = Some(args.into_iter().map(Into::into).collect());
        self
    }

    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn env<K, V, I>(mut self, env: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        self.env = Some(env.into_iter().map(|(k, v)| (k.into(), v.into())).collect());
        self
    }

    pub fn shell(mut self, shell: TaskShell) -> Self {
        self.shell = Some(shell);
        self
    }
    pub fn pty(mut self, pty: bool) -> Self {
        self.pty = Some(pty);
        self
    }

    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }
    pub fn enable_stdin(mut self, b: bool) -> Self {
        self.enable_stdin = Some(b);
        self
    }

    pub fn validate(&self) -> Result<(), TaskError> {
        if self.command.is_empty() {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot be empty".to_string(),
            ));
        }

        if let Some(timeout) = self.timeout_ms {
            if timeout == 0 {
                return Err(TaskError::InvalidConfiguration(
                    "Timeout must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StreamSource {
    Stdout = 0,
    Stderr = 1,
}
impl Default for StreamSource {
    fn default() -> Self {
        Self::Stdout
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskShell {
    None,
    Auto,
    #[cfg(windows)]
    Cmd,
    #[cfg(windows)]
    Powershell,
    #[cfg(unix)]
    Bash,
}
impl Default for TaskShell {
    fn default() -> Self {
        Self::None
    }
}

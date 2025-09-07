use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::tasks::error::TaskError;
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct TaskConfig {
    /// The command or executable to run
    pub command: String,

    /// Arguments to pass to the command
    pub args: Option<Vec<String>>,

    /// Working directory for the command
    pub working_dir: Option<String>,

    /// Environment variables for the command
    pub env: Option<HashMap<String, String>>,

    /// Maximum allowed runtime in milliseconds
    pub timeout_ms: Option<u64>,

    /// Allow providing input to the task via stdin.
    pub enable_stdin: Option<bool>,
}

pub type SharedTaskConfig = Arc<TaskConfig>;
impl Default for TaskConfig {
    fn default() -> Self {
        TaskConfig {
            command: String::new(),
            args: None,
            working_dir: None,
            env: None,
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

    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }
    pub fn enable_stdin(mut self, b: bool) -> Self {
        self.enable_stdin = Some(b);
        self
    }

    pub fn validate(&self) -> Result<(), TaskError> {
        const MAX_COMMAND_LEN: usize = 4096;
        const MAX_ARG_LEN: usize = 4096;
        const MAX_WORKING_DIR_LEN: usize = 4096;
        const MAX_ENV_KEY_LEN: usize = 1024;
        const MAX_ENV_VALUE_LEN: usize = 4096;

        // Validate command
        if self.command.is_empty() {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot be empty".to_string(),
            ));
        }
        if self.command.trim() != self.command {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot have leading or trailing whitespace".to_string(),
            ));
        }
        if self.command.len() > MAX_COMMAND_LEN {
            return Err(TaskError::InvalidConfiguration(
                "Command length exceeds maximum allowed length".to_string(),
            ));
        }

        // Validate arguments
        if let Some(args) = &self.args {
            for arg in args {
                if arg.is_empty() {
                    return Err(TaskError::InvalidConfiguration(
                        "Arguments cannot be empty".to_string(),
                    ));
                }
                if arg.trim() != arg {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Argument '{}' cannot have leading/trailing whitespace",
                        arg
                    )));
                }
                if arg.len() > MAX_ARG_LEN {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Argument '{}' exceeds maximum length",
                        arg
                    )));
                }
            }
        }

        // Validate working directory
        if let Some(dir) = &self.working_dir {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Working directory '{}' does not exist",
                    dir
                )));
            }
            if !path.is_dir() {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Working directory '{}' is not a directory",
                    dir
                )));
            }
            if dir.trim() != dir {
                return Err(TaskError::InvalidConfiguration(
                    "Working directory cannot have leading/trailing whitespace".to_string(),
                ));
            }
            if dir.len() > MAX_WORKING_DIR_LEN {
                return Err(TaskError::InvalidConfiguration(
                    "Working directory path exceeds maximum length".to_string(),
                ));
            }
        }

        // Validate environment variables
        if let Some(env) = &self.env {
            for (k, v) in env {
                if k.is_empty() {
                    return Err(TaskError::InvalidConfiguration(
                        "Environment variable key cannot be empty".to_string(),
                    ));
                }
                if k.contains('=') {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Environment variable key '{}' cannot contain '='",
                        k
                    )));
                }
                if k.contains(' ') {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Environment variable key '{}' cannot contain spaces",
                        k
                    )));
                }
                if k.len() > MAX_ENV_KEY_LEN {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Environment variable key '{}' exceeds maximum length",
                        k
                    )));
                }
                if v.trim() != v {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Environment variable '{}' value cannot have leading/trailing whitespace",
                        k
                    )));
                }
                if v.len() > MAX_ENV_VALUE_LEN {
                    return Err(TaskError::InvalidConfiguration(format!(
                        "Environment variable '{}' value exceeds maximum length",
                        k
                    )));
                }
            }
        }

        // Validate timeout
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

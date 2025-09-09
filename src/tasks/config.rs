use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::tasks::error::TaskError;

/// Configuration for a task to be executed
#[derive(Debug, Deserialize, Serialize, Clone)]
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

    /// Allow providing input to the task via stdin
    pub enable_stdin: Option<bool>,

    /// Optional string to indicate the task is ready (for long-running processes like servers)
    pub ready_indicator: Option<String>,

    /// Source of the ready indicator string (stdout/stderr)
    pub ready_indicator_source: Option<StreamSource>,
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
            ready_indicator: None,
            ready_indicator_source: Some(StreamSource::Stdout),
        }
    }
}

impl TaskConfig {
    /// Create a new task configuration with the given command
    ///
    /// # Example
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// let config = TaskConfig::new("echo");
    /// ```
    pub fn new(command: impl Into<String>) -> Self {
        TaskConfig {
            command: command.into(),
            ..Default::default()
        }
    }

    /// Set the arguments for the command
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = Some(args.into_iter().map(Into::into).collect());
        self
    }

    /// Set the working directory for the command
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set environment variables for the command
    pub fn env<K, V, I>(mut self, env: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        self.env = Some(env.into_iter().map(|(k, v)| (k.into(), v.into())).collect());
        self
    }

    /// Set the maximum allowed runtime in milliseconds
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }
    /// Enable or disable stdin for the task
    pub fn enable_stdin(mut self, b: bool) -> Self {
        self.enable_stdin = Some(b);
        self
    }
    /// Set the ready indicator for the task
    pub fn ready_indicator(mut self, indicator: impl Into<String>) -> Self {
        self.ready_indicator = Some(indicator.into());
        self
    }

    /// Set the source of the ready indicator
    pub fn ready_indicator_source(mut self, source: StreamSource) -> Self {
        self.ready_indicator_source = Some(source);
        self
    }

    /// Validate the configuration
    ///
    /// Returns `Ok(())` if valid, or `TaskError::InvalidConfiguration` describing the problem
    /// # Examples
    ///
    /// ```
    /// use tcrm_task::tasks::config::TaskConfig;
    /// // Valid config
    /// let config = TaskConfig::new("echo");
    /// assert!(config.validate().is_ok());
    ///
    /// // Invalid config (empty command)
    /// let config = TaskConfig::new("");
    /// assert!(config.validate().is_err());
    /// ```
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

        // Validate ready_indicator
        if let Some(indicator) = &self.ready_indicator {
            if indicator.is_empty() {
                return Err(TaskError::InvalidConfiguration(
                    "ready_indicator cannot be empty string".to_string(),
                ));
            }
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, env::temp_dir};

    use crate::tasks::{config::TaskConfig, error::TaskError};

    #[test]
    fn validation() {
        // Valid config
        let config = TaskConfig::new("echo").args(["hello"]);
        assert!(config.validate().is_ok());

        // Empty command should fail
        let config = TaskConfig::new("");
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Command with leading/trailing whitespace should fail
        let config = TaskConfig::new("  echo  ");
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Command exceeding max length should fail
        let long_cmd = "a".repeat(4097);
        let config = TaskConfig::new(long_cmd);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Zero timeout should fail
        let config = TaskConfig::new("echo").timeout_ms(0);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Valid timeout should pass
        let config = TaskConfig::new("echo").timeout_ms(30);
        assert!(config.validate().is_ok());

        // Arguments with empty string should fail
        let config = TaskConfig::new("echo").args([""]);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Argument with leading/trailing whitespace should fail
        let config = TaskConfig::new("echo").args([" hello "]);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Argument exceeding max length should fail
        let long_arg = "a".repeat(4097);
        let config = TaskConfig::new("echo").args([long_arg]);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Working directory that does not exist should fail
        let config = TaskConfig::new("echo").working_dir("/non/existent/dir");
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Working directory with temp dir should pass
        let dir = temp_dir();
        let config = TaskConfig::new("echo").working_dir(dir.as_path().to_str().unwrap());
        assert!(config.validate().is_ok());

        // Working directory with whitespace should fail
        let dir = temp_dir();
        let dir_str = format!(" {} ", dir.as_path().to_str().unwrap());
        let config = TaskConfig::new("echo").working_dir(&dir_str);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable with empty key should fail
        let mut env = HashMap::new();
        env.insert(String::new(), "value".to_string());
        let config = TaskConfig::new("echo").env(env);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable with space in key should fail
        let mut env = HashMap::new();
        env.insert("KEY WITH SPACE".to_string(), "value".to_string());
        let config = TaskConfig::new("echo").env(env);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable with '=' in key should fail
        let mut env = HashMap::new();
        env.insert("KEY=BAD".to_string(), "value".to_string());
        let config = TaskConfig::new("echo").env(env);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable key exceeding max length should fail
        let mut env = HashMap::new();
        env.insert("A".repeat(1025), "value".to_string());
        let config = TaskConfig::new("echo").env(env);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable value with whitespace should fail
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), " value ".to_string());
        let config = TaskConfig::new("echo").env(env);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable value exceeding max length should fail
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "A".repeat(4097));
        let config = TaskConfig::new("echo").env(env);
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // Environment variable key/value valid should pass
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "some value".to_string());
        let config = TaskConfig::new("echo").env(env);
        assert!(config.validate().is_ok());

        // ready_indicator: empty string should fail
        let mut config = TaskConfig::new("echo");
        config.ready_indicator = Some(String::new());
        assert!(matches!(
            config.validate(),
            Err(TaskError::InvalidConfiguration(_))
        ));

        // ready_indicator: leading/trailing spaces should pass
        let mut config = TaskConfig::new("echo");
        config.ready_indicator = Some("  READY  ".to_string());
        assert!(config.validate().is_ok());

        // ready_indicator: normal string should pass
        let mut config = TaskConfig::new("echo");
        config.ready_indicator = Some("READY".to_string());
        assert!(config.validate().is_ok());
    }
    #[test]
    fn config_builder() {
        let config = TaskConfig::new("cargo")
            .args(["build", "--release"])
            .working_dir("/home/user/project")
            .env([("RUST_LOG", "debug"), ("CARGO_TARGET_DIR", "target")])
            .timeout_ms(300)
            .enable_stdin(true);

        assert_eq!(config.command, "cargo");
        assert_eq!(
            config.args,
            Some(vec!["build".to_string(), "--release".to_string()])
        );
        assert_eq!(config.working_dir, Some("/home/user/project".to_string()));
        assert!(config.env.is_some());
        assert_eq!(config.timeout_ms, Some(300));
        assert_eq!(config.enable_stdin, Some(true));
    }
}

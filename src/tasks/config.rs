use std::{collections::HashMap, sync::Arc};

use crate::tasks::{error::TaskError, validator::ConfigValidator};

/// Configuration for a task to be executed.
///
/// `TaskConfig` defines all parameters needed to execute a system process securely.
/// It includes the command, arguments, environment setup, timeouts, and monitoring options.
///
/// # Examples
///
/// ## Basic Command
/// ```rust
/// use tcrm_task::tasks::config::TaskConfig;
///
/// let config = TaskConfig::new("cmd")
///     .args(["/C", "dir", "C:\\"]);
/// ```
///
/// ## Complex Configuration
/// ```rust
/// use tcrm_task::tasks::config::{TaskConfig, StreamSource};
/// use std::collections::HashMap;
///
/// let mut env = HashMap::new();
/// env.insert("PATH".to_string(), "C:\\Windows\\System32".to_string());
///
/// let config = TaskConfig::new("cmd")
///     .args(["/C", "echo", "Server started"])
///     .working_dir("C:\\")
///     .env(env)
///     .timeout_ms(30000)
///     .enable_stdin(true)
///     .ready_indicator("Server started")
///     .ready_indicator_source(StreamSource::Stdout);
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
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

    /// Enable process group management for child process termination (default: true)
    ///
    /// When enabled, creates process groups (Unix) or Job Objects (Windows) to ensure
    /// all child processes and their descendants are terminated when the main process is killed.
    pub use_process_group: Option<bool>,
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
            use_process_group: Some(true),
        }
    }
}

impl TaskConfig {
    /// Create a new task configuration with the given command
    ///
    /// # Arguments
    ///
    /// * `command` - The executable command to run (e.g., "ls", "node", "python")
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// let config = TaskConfig::new("echo");
    /// let config2 = TaskConfig::new("node".to_string());
    /// ```
    pub fn new(command: impl Into<String>) -> Self {
        TaskConfig {
            command: command.into(),
            ..Default::default()
        }
    }

    /// Set the arguments for the command
    ///
    /// # Arguments
    ///
    /// * `args` - Iterator of arguments to pass to the command
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// let config = TaskConfig::new("ls")
    ///     .args(["-la", "/tmp"]);
    ///     
    /// let config2 = TaskConfig::new("cargo")
    ///     .args(vec!["build", "--release"]);
    /// ```
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = Some(args.into_iter().map(Into::into).collect());
        self
    }

    /// Set the working directory for the command
    ///
    /// The working directory must exist when the task is executed.
    ///
    /// # Arguments
    ///
    /// * `dir` - Path to the working directory
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// let config = TaskConfig::new("ls")
    ///     .working_dir("/tmp");
    ///     
    /// let config2 = TaskConfig::new("cargo")
    ///     .working_dir("/path/to/project");
    /// ```
    #[must_use]
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set environment variables for the command
    ///
    /// # Arguments
    ///
    /// * `env` - Iterator of (key, value) pairs for environment variables
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    /// use std::collections::HashMap;
    ///
    /// // Using tuples
    /// let config = TaskConfig::new("node")
    ///     .env([("NODE_ENV", "production"), ("PORT", "3000")]);
    ///
    /// // Using HashMap
    /// let mut env = HashMap::new();
    /// env.insert("RUST_LOG".to_string(), "debug".to_string());
    /// let config2 = TaskConfig::new("cargo")
    ///     .env(env);
    /// ```
    #[must_use]
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
    ///
    /// If the task runs longer than this timeout, it will be terminated.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds (must be > 0)
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// // 30 second timeout
    /// let config = TaskConfig::new("long-running-task")
    ///     .timeout_ms(30000);
    ///
    /// // 5 minute timeout
    /// let config2 = TaskConfig::new("build-script")
    ///     .timeout_ms(300000);
    /// ```
    #[must_use]
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }

    /// Enable or disable stdin for the task
    ///
    /// When enabled, you can send input to the process via the stdin channel.
    ///
    /// # Arguments
    ///
    /// * `b` - Whether to enable stdin input
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// // Interactive command that needs input
    /// let config = TaskConfig::new("python")
    ///     .args(["-i"])
    ///     .enable_stdin(true);
    /// ```
    #[must_use]
    pub fn enable_stdin(mut self, b: bool) -> Self {
        self.enable_stdin = Some(b);
        self
    }

    /// Set the ready indicator for the task
    ///
    /// For long-running processes (like servers), this string indicates when
    /// the process is ready to accept requests. When this string appears in
    /// the process output, a Ready event will be emitted.
    ///
    /// # Arguments
    ///
    /// * `indicator` - String to look for in process output
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// let config = TaskConfig::new("my-server")
    ///     .ready_indicator("Server listening on port");
    ///
    /// let config2 = TaskConfig::new("database")
    ///     .ready_indicator("Database ready for connections");
    /// ```
    #[must_use]
    pub fn ready_indicator(mut self, indicator: impl Into<String>) -> Self {
        self.ready_indicator = Some(indicator.into());
        self
    }

    /// Set the source of the ready indicator
    ///
    /// Specifies whether to look for the ready indicator in stdout or stderr.
    ///
    /// # Arguments
    ///
    /// * `source` - Stream source (Stdout or Stderr)
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::{TaskConfig, StreamSource};
    ///
    /// let config = TaskConfig::new("my-server")
    ///     .ready_indicator("Ready")
    ///     .ready_indicator_source(StreamSource::Stderr);
    /// ```
    #[must_use]
    pub fn ready_indicator_source(mut self, source: StreamSource) -> Self {
        self.ready_indicator_source = Some(source);
        self
    }

    /// Enable or disable process group management
    ///
    /// When enabled (default), creates process groups on Unix or Job Objects on Windows
    /// to ensure all child processes and their descendants are terminated when the main
    /// process is killed. This prevents orphaned processes.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to use process group management
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// // Disable process group management
    /// let config = TaskConfig::new("cmd")
    ///     .use_process_group(false);
    ///     
    /// // Explicitly enable (though it's enabled by default)
    /// let config2 = TaskConfig::new("node")
    ///     .use_process_group(true);
    /// ```
    #[must_use]
    pub fn use_process_group(mut self, enabled: bool) -> Self {
        self.use_process_group = Some(enabled);
        self
    }

    /// Validate the configuration
    ///
    /// Validates all configuration parameters.
    /// This method should be called before executing the task to ensure
    /// safe operation.
    ///
    /// # Validation Checks
    /// - all fields length limits
    /// - **Command**: Must not be empty, contain shell injection patterns
    /// - **Arguments**: Must not contain null bytes or shell injection patterns  
    /// - **Working Directory**: Must exist and be a valid directory
    /// - **Environment Variables**: Keys must not contain spaces, '=', or null bytes
    /// - **Timeout**: Must be greater than 0 if specified
    /// - **Ready Indicator**: Must not be empty if specified
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the configuration is valid
    /// - `Err(TaskError::InvalidConfiguration)` with details if validation fails
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError`] if any validation check fails:
    /// - [`TaskError::InvalidConfiguration`] for configuration errors
    /// - [`TaskError::IO`] for working directory validation failures
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// // Valid config
    /// let config = TaskConfig::new("echo")
    ///     .args(["hello", "world"]);
    /// assert!(config.validate().is_ok());
    ///
    /// // Invalid config (empty command)
    /// let config = TaskConfig::new("");
    /// assert!(config.validate().is_err());
    ///
    /// // Invalid config (zero timeout)
    /// let config = TaskConfig::new("sleep")
    ///     .timeout_ms(0);
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), TaskError> {
        ConfigValidator::validate_command(&self.command)?;
        if let Some(ready_indicator) = &self.ready_indicator {
            ConfigValidator::validate_ready_indicator(ready_indicator)?;
        }
        if let Some(args) = &self.args {
            ConfigValidator::validate_args(args)?;
        }
        if let Some(dir) = &self.working_dir {
            ConfigValidator::validate_working_dir(dir)?;
        }
        if let Some(env) = &self.env {
            ConfigValidator::validate_env_vars(env)?;
        }
        if let Some(timeout) = &self.timeout_ms {
            ConfigValidator::validate_timeout(timeout)?;
        }
        Ok(())
    }

    /// Check if process group management is enabled
    ///
    /// Returns true if process group management should be used, false otherwise.
    /// Defaults to true if not explicitly set.
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::config::TaskConfig;
    ///
    /// let config = TaskConfig::new("cmd");
    /// assert!(config.is_process_group_enabled()); // Default is true
    ///
    /// let config2 = TaskConfig::new("cmd").use_process_group(false);
    /// assert!(!config2.is_process_group_enabled());
    /// ```
    pub fn is_process_group_enabled(&self) -> bool {
        self.use_process_group.unwrap_or(true)
    }
}

/// Specifies the source stream for output monitoring
///
/// Used with ready indicators to specify whether to monitor stdout or stderr
/// for the ready signal from long-running processes.
///
/// # Examples
///
/// ```rust
/// use tcrm_task::tasks::config::{TaskConfig, StreamSource};
///
/// // Monitor stdout for ready signal
/// let config = TaskConfig::new("web-server")
///     .ready_indicator("Server ready")
///     .ready_indicator_source(StreamSource::Stdout);
///
/// // Monitor stderr for ready signal  
/// let config2 = TaskConfig::new("database")
///     .ready_indicator("Ready for connections")
///     .ready_indicator_source(StreamSource::Stderr);
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[derive(Debug, Clone, PartialEq)]
pub enum StreamSource {
    /// Standard output stream
    Stdout = 0,
    /// Standard error stream  
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

use std::collections::HashMap;
use std::path::Path;

use crate::tasks::error::TaskError;
const MAX_COMMAND_LEN: usize = 4096;
const MAX_ARG_LEN: usize = 4096;
const MAX_WORKING_DIR_LEN: usize = 4096;
const MAX_ENV_KEY_LEN: usize = 1024;
const MAX_ENV_VALUE_LEN: usize = 4096;
/// Security validation utilities for task configuration
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validates command name.
    ///
    /// # Arguments
    ///
    /// * `command` - The command string to validate.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the command is valid.
    /// - `Err(TaskError::InvalidConfiguration)` if the command is invalid.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError::InvalidConfiguration`] if:
    /// - Command is empty or contains only whitespace
    /// - Command has leading or trailing whitespace
    /// - Command length exceeds maximum allowed length
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    ///
    /// let valid_command = "echo";
    /// assert!(ConfigValidator::validate_command(valid_command).is_ok());
    ///
    /// let invalid_command = "";
    /// assert!(ConfigValidator::validate_command(invalid_command).is_err());
    /// ```
    pub fn validate_command(command: &str) -> Result<(), TaskError> {
        // Check for empty or whitespace-only commands
        if command.trim().is_empty() {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot be empty".to_string(),
            ));
        }

        // Only check for obvious injection attempts, not shell features
        // if Self::contains_obvious_injection(command) {
        //     return Err(TaskError::InvalidConfiguration(
        //         "Command contains potentially dangerous injection patterns".to_string(),
        //     ));
        // }

        if command.trim() != command {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot have leading or trailing whitespace".to_string(),
            ));
        }
        if command.len() > MAX_COMMAND_LEN {
            return Err(TaskError::InvalidConfiguration(
                "Command length exceeds maximum allowed length".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - A slice of argument strings to validate.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all arguments are valid.
    /// - `Err(TaskError::InvalidConfiguration)` if any argument is invalid.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError::InvalidConfiguration`] if:
    /// - any argument contains null bytes
    /// - any argument is an empty string
    /// - any argument has leading or trailing whitespace
    /// - any argument exceeds maximum length
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    ///
    /// let valid_args = vec!["arg1".to_string(), "arg2".to_string()];
    /// assert!(ConfigValidator::validate_args(&valid_args).is_ok());
    ///
    /// let invalid_args = vec!["arg1".to_string(), "\0".to_string()];
    /// assert!(ConfigValidator::validate_args(&invalid_args).is_err());
    /// ```
    pub fn validate_args(args: &[String]) -> Result<(), TaskError> {
        for arg in args {
            // Only check for null bytes
            if arg.contains('\0') {
                return Err(TaskError::InvalidConfiguration(
                    "Argument contains null characters".to_string(),
                ));
            }
            if arg.is_empty() {
                return Err(TaskError::InvalidConfiguration(
                    "Arguments cannot be empty".to_string(),
                ));
            }
            if arg.trim() != arg {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Argument '{arg}' cannot have leading/trailing whitespace"
                )));
            }
            if arg.len() > MAX_ARG_LEN {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Argument '{arg}' exceeds maximum length"
                )));
            }
        }
        Ok(())
    }

    /// Validates the working directory path.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory path to validate.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the directory is valid.
    /// - `Err(TaskError::InvalidConfiguration)` if the directory is invalid.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError::InvalidConfiguration`] if:
    /// - Directory does not exist
    /// - Path exists but is not a directory
    /// - Directory has leading or trailing whitespace
    /// - Directory path exceeds maximum length
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    /// use std::env;
    ///
    /// // Test with current directory (should exist)
    /// let current_dir = env::current_dir().unwrap();
    /// let valid_dir = current_dir.to_str().unwrap();
    /// assert!(ConfigValidator::validate_working_dir(valid_dir).is_ok());
    ///
    /// // Test with nonexistent directory
    /// let invalid_dir = "nonexistent_dir_12345";
    /// assert!(ConfigValidator::validate_working_dir(invalid_dir).is_err());
    /// ```
    pub fn validate_working_dir(dir: &str) -> Result<(), TaskError> {
        let path = Path::new(dir);

        // Check if path exists
        if !path.exists() {
            return Err(TaskError::IO(format!(
                "Working directory does not exist: {dir}"
            )));
        }

        // Check if it's actually a directory
        if !path.is_dir() {
            return Err(TaskError::InvalidConfiguration(format!(
                "Working directory is not a directory: {dir}"
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

        Ok(())
    }

    /// Validates environment variables.
    ///
    /// # Arguments
    ///
    /// * `env` - A hashmap of environment variable key-value pairs to validate.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all environment variables are valid.
    /// - `Err(TaskError::InvalidConfiguration)` if any environment variable is invalid.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError::InvalidConfiguration`] if:
    /// - Any environment variable key contains spaces, '=', or null bytes
    /// - Any environment variable value contains null bytes
    /// - Any environment variable key/value exceeds maximum length
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    /// use std::collections::HashMap;
    ///
    /// let mut valid_env = HashMap::new();
    /// valid_env.insert("KEY".to_string(), "VALUE".to_string());
    /// assert!(ConfigValidator::validate_env_vars(&valid_env).is_ok());
    ///
    /// let mut invalid_env = HashMap::new();
    /// invalid_env.insert("KEY\0".to_string(), "VALUE".to_string());
    /// assert!(ConfigValidator::validate_env_vars(&invalid_env).is_err());
    /// ```
    pub fn validate_env_vars(env: &HashMap<String, String>) -> Result<(), TaskError> {
        for (key, value) in env {
            // Validate key
            if key.trim().is_empty() {
                return Err(TaskError::InvalidConfiguration(
                    "Environment variable key cannot be empty".to_string(),
                ));
            }
            if key.contains('=') || key.contains('\0') || key.contains('\t') || key.contains('\n') {
                return Err(TaskError::InvalidConfiguration(
                    "Environment variable key contains invalid characters".to_string(),
                ));
            }

            if key.contains(' ') {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable key '{key}' cannot contain spaces"
                )));
            }

            if key.len() > MAX_ENV_KEY_LEN {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable key '{key}' exceeds maximum length"
                )));
            }

            // Validate value
            if value.contains('\0') {
                return Err(TaskError::InvalidConfiguration(
                    "Environment variable value contains null characters".to_string(),
                ));
            }

            if value.trim() != value {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable '{key}' value cannot have leading/trailing whitespace"
                )));
            }
            if value.len() > MAX_ENV_VALUE_LEN {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable '{key}' value exceeds maximum length"
                )));
            }
        }
        Ok(())
    }

    /// Validates ready indicator string
    ///
    /// # Arguments
    ///
    /// * `indicator` - The ready indicator string to validate
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the indicator is valid
    /// - `Err(TaskError::InvalidConfiguration)` if the indicator is invalid
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::InvalidConfiguration`] if:
    /// - Indicator is empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    ///
    /// let valid_indicator = "ready";
    /// assert!(ConfigValidator::validate_ready_indicator(valid_indicator).is_ok());
    ///
    /// let invalid_indicator = "";
    /// assert!(ConfigValidator::validate_ready_indicator(invalid_indicator).is_err());
    /// ```
    pub fn validate_ready_indicator(indicator: &str) -> Result<(), TaskError> {
        if indicator.is_empty() {
            return Err(TaskError::InvalidConfiguration(
                "ready_indicator cannot be empty string".to_string(),
            ));
        }
        Ok(())
    }

    /// Validates timeout value.
    ///
    /// Ensures that the timeout value is greater than 0. A timeout of 0 would mean
    /// immediate timeout, which is not useful for task execution.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout value in milliseconds to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If timeout is valid (greater than 0)
    /// * `Err(TaskError)` - If timeout is 0
    ///
    /// # Example
    ///
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    ///
    /// // Valid timeout
    /// assert!(ConfigValidator::validate_timeout(&5000).is_ok());
    ///
    /// // Invalid timeout (0)
    /// assert!(ConfigValidator::validate_timeout(&0).is_err());
    /// ```
    pub fn validate_timeout(timeout: &u64) -> Result<(), TaskError> {
        if *timeout == 0 {
            return Err(TaskError::InvalidConfiguration(
                "Timeout must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }

    /// Checks for obvious injection attempts while allowing normal shell features.
    ///
    /// This method identifies clearly malicious patterns without blocking
    /// legitimate shell functionality. It focuses on patterns that are rarely used
    /// in normal command execution.
    ///
    /// # Arguments
    ///
    /// * `input` - The command string to check for injection patterns.
    ///
    /// # Returns
    ///
    /// `true` if obvious injection patterns are detected, `false` otherwise.
    pub fn contains_obvious_injection(input: &str) -> bool {
        // Only block patterns that are clearly malicious, not functional shell features
        let obvious_injection_patterns = [
            "\0",         // Null bytes
            "\x00",       // Null bytes (hex)
            "\r\n",       // CRLF injection
            "eval(",      // Direct eval calls
            "exec(",      // Direct exec calls
            "os.system(", // Direct Python code execution
        ];

        obvious_injection_patterns
            .iter()
            .any(|pattern| input.contains(pattern))
    }

    /// Validates command with strict security rules for untrusted input sources.
    ///
    /// This is an alternative to `validate_command` that blocks all shell features
    ///
    /// # Arguments
    ///
    /// * `command` - The command string to validate strictly.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the command passes strict validation.
    /// - `Err(TaskError::InvalidConfiguration)` if the command contains potentially dangerous patterns.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskError::InvalidConfiguration`] if:
    /// - Command is empty or contains only whitespace
    /// - Command contains shell metacharacters or redirection operators
    ///
    /// # Examples
    /// ```rust
    /// use tcrm_task::tasks::validator::ConfigValidator;
    ///
    /// // Simple command should pass
    /// let simple_command = "echo";
    /// assert!(ConfigValidator::validate_command_strict(simple_command).is_ok());
    ///
    /// // Command with shell features should fail
    /// let shell_command = "echo hello; rm -rf /";
    /// assert!(ConfigValidator::validate_command_strict(shell_command).is_err());
    /// ```
    #[allow(dead_code)]
    pub fn validate_command_strict(command: &str) -> Result<(), TaskError> {
        // Check for empty or whitespace-only commands
        if command.trim().is_empty() {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot be empty".to_string(),
            ));
        }

        // Strict validation - blocks shell features
        let dangerous_patterns = [
            ";", "&", "|", "`", "#", "$", "(", ")", "{", "}", "[", "]", "<", ">", "&&", "||", ">>",
            "<<", "\n", "\r",
        ];

        if dangerous_patterns
            .iter()
            .any(|pattern| command.contains(pattern))
        {
            return Err(TaskError::InvalidConfiguration(
                "Command contains shell metacharacters (use validate_command for developer tools)"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

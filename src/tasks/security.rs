use std::collections::HashMap;
use std::path::Path;

use crate::tasks::error::TaskError;
const MAX_COMMAND_LEN: usize = 4096;
const MAX_ARG_LEN: usize = 4096;
const MAX_WORKING_DIR_LEN: usize = 4096;
const MAX_ENV_KEY_LEN: usize = 1024;
const MAX_ENV_VALUE_LEN: usize = 4096;
/// Security validation utilities for task configuration
pub struct SecurityValidator;

impl SecurityValidator {
    /// Validates command name
    pub fn validate_command(command: &str) -> Result<(), TaskError> {
        // Check for empty or whitespace-only commands
        if command.trim().is_empty() {
            return Err(TaskError::InvalidConfiguration(
                "Command cannot be empty".to_string(),
            ));
        }

        // Only check for obvious injection attempts, not shell features
        if Self::contains_obvious_injection(command) {
            return Err(TaskError::InvalidConfiguration(
                "Command contains potentially dangerous injection patterns".to_string(),
            ));
        }

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

    /// Validates arguments for security issues
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
        Ok(())
    }

    /// Validates working directory
    pub fn validate_working_dir(dir: &str) -> Result<(), TaskError> {
        let path = Path::new(dir);

        // Check if path exists
        if !path.exists() {
            return Err(TaskError::InvalidConfiguration(format!(
                "Working directory does not exist: {}",
                dir
            )));
        }

        // Check if it's actually a directory
        if !path.is_dir() {
            return Err(TaskError::InvalidConfiguration(format!(
                "Working directory is not a directory: {}",
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

        Ok(())
    }

    /// Validates environment variables
    pub fn validate_env_vars(env: &HashMap<String, String>) -> Result<(), TaskError> {
        for (key, value) in env {
            // Validate key
            if key.trim().is_empty() {
                return Err(TaskError::InvalidConfiguration(
                    "Environment variable key cannot be empty".to_string(),
                ));
            }
            if key.contains('=') {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable key '{}' cannot contain '='",
                    key
                )));
            }

            if key.contains('=') || key.contains('\0') {
                return Err(TaskError::InvalidConfiguration(
                    "Environment variable key contains invalid characters".to_string(),
                ));
            }

            if key.contains(' ') {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable key '{}' cannot contain spaces",
                    key
                )));
            }

            if key.len() > MAX_ENV_KEY_LEN {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable key '{}' exceeds maximum length",
                    key
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
                    "Environment variable '{}' value cannot have leading/trailing whitespace",
                    key
                )));
            }
            if value.len() > MAX_ENV_VALUE_LEN {
                return Err(TaskError::InvalidConfiguration(format!(
                    "Environment variable '{}' value exceeds maximum length",
                    key
                )));
            }
        }
        Ok(())
    }

    /// Check for obvious injection attempts (but allow normal shell features)
    fn contains_obvious_injection(input: &str) -> bool {
        // Only block patterns that are clearly malicious, not functional shell features
        let obvious_injection_patterns = [
            "\0",    // Null bytes
            "\x00",  // Null bytes (hex)
            "\r\n",  // CRLF injection
            "eval(", // Direct eval calls
            "exec(", // Direct exec calls
        ];

        obvious_injection_patterns
            .iter()
            .any(|pattern| input.contains(pattern))
    }

    /// Alternative: Strict validation for untrusted input sources
    /// Use this when TaskConfig comes from external/untrusted sources
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
            ";", "&", "|", "`", "$", "(", ")", "{", "}", "[", "]", "<", ">", "&&", "||", ">>",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_command_rejects_empty() {
        assert!(SecurityValidator::validate_command("").is_err());
        assert!(SecurityValidator::validate_command("   ").is_err());
    }

    #[test]
    fn validate_command_accepts_shell_features() {
        // These should be allowed for developer tools
        let shell_commands = [
            "ls | grep pattern",
            "echo hello > output.txt",
            "make && npm test",
            "command1; command2",
            "echo $PATH",
            "ls $(pwd)",
            "cat file.txt | head -n 10",
        ];

        for cmd in &shell_commands {
            assert!(
                SecurityValidator::validate_command(cmd).is_ok(),
                "Should accept shell feature: {}",
                cmd
            );
        }
    }

    #[test]
    fn validate_command_rejects_obvious_injection() {
        let dangerous_commands = [
            "command\0with\0nulls",
            "eval(malicious_code)",
            "exec(rm -rf /)",
            "command\r\necho injected",
        ];

        for cmd in &dangerous_commands {
            assert!(
                SecurityValidator::validate_command(cmd).is_err(),
                "Should reject obvious injection: {}",
                cmd
            );
        }
    }

    #[test]
    fn validate_command_strict_blocks_shell_features() {
        let shell_commands = [
            "ls | grep pattern",
            "echo hello > output.txt",
            "make && npm test",
        ];

        for cmd in &shell_commands {
            assert!(
                SecurityValidator::validate_command_strict(cmd).is_err(),
                "Strict validation should reject: {}",
                cmd
            );
        }
    }

    #[test]
    fn validate_command_accepts_safe_commands() {
        let safe_commands = [
            "echo",
            "ls",
            "cat",
            "grep",
            "node",
            "python",
            "ls -la",
            "grep pattern file.txt",
            "node script.js",
        ];

        for cmd in &safe_commands {
            assert!(
                SecurityValidator::validate_command(cmd).is_ok(),
                "Should accept: {}",
                cmd
            );
        }
    }

    #[test]
    fn validate_args_accepts_normal_args() {
        let normal_args = vec![
            "arg1".to_string(),
            "--flag".to_string(),
            "file.txt".to_string(),
            "path/to/file".to_string(),
        ];

        assert!(SecurityValidator::validate_args(&normal_args).is_ok());
    }

    #[test]
    fn validate_args_rejects_null_bytes() {
        let dangerous_args = vec!["arg\0with\0nulls".to_string()];

        assert!(SecurityValidator::validate_args(&dangerous_args).is_err());
    }

    #[test]
    fn validate_working_dir_accepts_relative_paths() {
        // Should accept relative paths including .. for developer use
        let current_dir = std::env::current_dir().unwrap();
        assert!(SecurityValidator::validate_working_dir(current_dir.to_str().unwrap()).is_ok());
    }

    #[test]
    fn validate_working_dir_rejects_nonexistent() {
        assert!(SecurityValidator::validate_working_dir("/nonexistent/path").is_err());
    }

    #[test]
    fn validate_env_vars_accepts_spaces_in_keys() {
        // For developer tools, we're more lenient with env vars
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env.insert(
            "CUSTOM_VAR".to_string(),
            "some value with spaces".to_string(),
        );
        assert!(SecurityValidator::validate_env_vars(&env).is_ok());
    }

    #[test]
    fn validate_env_vars_rejects_invalid_keys() {
        let mut env = HashMap::new();
        env.insert("KEY=BAD".to_string(), "value".to_string());
        assert!(SecurityValidator::validate_env_vars(&env).is_err());
    }

    #[test]
    fn validate_env_vars_rejects_null_chars() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value\0with\0nulls".to_string());
        assert!(SecurityValidator::validate_env_vars(&env).is_err());
    }
}

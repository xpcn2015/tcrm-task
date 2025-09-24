#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::tasks::validator::ConfigValidator;

    #[test]
    fn reject_empty_commands() {
        assert!(ConfigValidator::validate_command("").is_err());
        assert!(ConfigValidator::validate_command("   ").is_err());
    }

    #[test]
    fn accepts_shell_features() {
        // These should be allowed for developer tools
        let shell_commands = [
            "ls | grep pattern",
            "echo hello > output.txt",
            "make && npm test",
            "command1; command2",
            "echo $PATH",
            "ls $(pwd)",
            "cat file.txt | head -n 10",
            "sleep 1 & echo done",
            "(cd /tmp && ls)",
            "echo 'hello world'",
            "echo \"hello world\"",
            "echo hello\\ world",
            "echo hello # comment",
            "cat <<EOF\nhello\nEOF",
            "false || echo fallback",
            "echo hello >> file.txt",
            "echo `date`",
        ];

        for cmd in &shell_commands {
            assert!(
                ConfigValidator::validate_command(cmd).is_ok(),
                "Should accept shell feature: {}",
                cmd
            );
        }
    }

    #[test]
    fn rejects_obvious_injection() {
        let dangerous_commands = [
            "command\0with\0nulls",
            "eval(malicious_code)",
            "command\r\necho injected",
            "command\x00with\x00nulls",
            "cmd\r\n\r\nanother",
            "cmd\u{0000}",
            "eval(exec('malicious'))",
            "os.system('rm -rf /')",
            "exec(rm -rf /)",
        ];

        for cmd in &dangerous_commands {
            assert!(
                ConfigValidator::validate_command(cmd).is_err(),
                "Should reject obvious injection: {}",
                cmd
            );
        }
    }

    #[test]
    fn strict_mode_blocks_shell_features() {
        let shell_commands = [
            "ls | grep pattern",
            "echo hello > output.txt",
            "make && npm test",
            "command1; command2",
            "echo $PATH",
            "ls $(pwd)",
            "cat file.txt | head -n 10",
            "sleep 1 & echo done",
            "(cd /tmp && ls)",
            "cat <<EOF\nhello\nEOF",
            "false || echo fallback",
            "echo hello >> file.txt",
            "echo `date`",
            "echo hello # comment",
        ];

        for cmd in &shell_commands {
            assert!(
                ConfigValidator::validate_command_strict(cmd).is_err(),
                "Strict validation should reject: {}",
                cmd
            );
        }
    }

    #[test]
    fn accepts_safe_commands() {
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
            "python main.py --input data.txt",
            "cargo build --release",
            "npm install",
            "git status",
            "git commit -m 'update'",
            "echo 'hello world'",
            "echo \"hello world\"",
            "ls /usr/local/bin",
            "cat README.md",
            "grep -i error log.txt",
            "python -m venv .env",
            "node --version",
            "ls -lh /tmp",
            "cat file.txt | head -n 10",
            "echo $PATH",
        ];

        for cmd in &safe_commands {
            assert!(
                ConfigValidator::validate_command(cmd).is_ok(),
                "Should accept: {}",
                cmd
            );
        }
    }

    #[test]
    fn accepts_normal_args() {
        let normal_args = vec![
            "arg1".to_string(),
            "--flag".to_string(),
            "file.txt".to_string(),
            "path/to/file".to_string(),
            "--input=data.txt".to_string(),
            "-v".to_string(),
            "--output".to_string(),
            "123".to_string(),
            "'quoted arg'".to_string(),
            "\"double quoted arg\"".to_string(),
            "C:\\Program Files\\App".to_string(),
            "./script.sh".to_string(),
            "--env=PROD".to_string(),
            "--threads=4".to_string(),
            "--config=path/config.yaml".to_string(),
            "--user=admin".to_string(),
            "--password=secret".to_string(),
            "--dry-run".to_string(),
            "--verbose".to_string(),
        ];

        assert!(ConfigValidator::validate_args(&normal_args).is_ok());
    }

    #[test]
    fn rejects_null_bytes() {
        let dangerous_args = vec!["arg\0with\0nulls".to_string()];

        assert!(ConfigValidator::validate_args(&dangerous_args).is_err());
    }

    #[test]
    fn validate_working_dir_accepts_relative_paths() {
        // Should accept relative paths including .. for developer use
        let current_dir = std::env::current_dir().unwrap();
        assert!(ConfigValidator::validate_working_dir(current_dir.to_str().unwrap()).is_ok());
    }

    #[test]
    fn validate_working_dir_rejects_nonexistent() {
        assert!(ConfigValidator::validate_working_dir("/nonexistent/path").is_err());
    }

    #[test]
    fn validate_env_vars_rejects_spaces_in_keys() {
        // Environment variable keys should not contain spaces
        let mut env = HashMap::new();
        env.insert("KEY WITH SPACE".to_string(), "value".to_string());
        assert!(ConfigValidator::validate_env_vars(&env).is_err());
    }

    #[test]
    fn validate_env_vars_accepts_normal_vars() {
        // Normal environment variables should be accepted
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env.insert(
            "CUSTOM_VAR".to_string(),
            "some value with spaces".to_string(),
        );
        assert!(ConfigValidator::validate_env_vars(&env).is_ok());
    }

    #[test]
    fn validate_env_vars_rejects_invalid_keys() {
        let mut env = HashMap::new();
        env.insert("KEY=BAD".to_string(), "value".to_string());
        assert!(ConfigValidator::validate_env_vars(&env).is_err());
    }

    #[test]
    fn validate_env_vars_rejects_null_chars() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value\0with\0nulls".to_string());
        assert!(ConfigValidator::validate_env_vars(&env).is_err());
    }
}

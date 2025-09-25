use std::process::Stdio;

use tokio::process::Command;

use crate::tasks::config::TaskConfig;

/// Configures a `tokio::process::Command` based on the provided `TaskConfig`.
///
/// Sets arguments, working directory, environment, and stdio options.
///
/// # Arguments
///
/// * `cmd` - The command to configure.
/// * `config` - The task configuration to apply.
pub(crate) fn setup_command(cmd: &mut Command, config: &TaskConfig) {
    // Setup additional arguments
    if let Some(args) = &config.args {
        cmd.args(args);
    }

    // Setup working directory with validation
    if let Some(dir) = &config.working_dir {
        cmd.current_dir(dir);
    }

    // Setup environment variables
    if let Some(envs) = &config.env {
        cmd.envs(envs);
    }

    // Setup stdio
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(
        if config.enable_stdin.unwrap_or(false) {
            Stdio::piped()
        } else {
            Stdio::null()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::process::Command;

    #[tokio::test]
    async fn setup_command_applies_args_and_runs_echo() {
        let mut cmd = if cfg!(windows) {
            Command::new("powershell")
        } else {
            Command::new("echo")
        };
        let config = if cfg!(windows) {
            let mut c = TaskConfig::new("powershell");
            c.args = Some(vec!["-Command".to_string(), "echo hello_test".to_string()]);
            c
        } else {
            let mut c = TaskConfig::new("echo");
            c.args = Some(vec!["hello_test".to_string()]);
            c.use_process_group = Some(false);
            c
        };
        setup_command(&mut cmd, &config);
        let output = cmd.output().await.expect("Failed to run echo");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("hello_test"),
            "stdout should contain argument"
        );
    }

    #[tokio::test]
    async fn setup_command_sets_working_dir_and_runs_pwd() {
        let mut cmd = if cfg!(windows) {
            Command::new("cmd")
        } else {
            Command::new("pwd")
        };
        let mut config = TaskConfig::new(if cfg!(windows) { "cmd" } else { "pwd" });
        let cwd = std::env::current_dir().unwrap();
        config.working_dir = Some(cwd.to_string_lossy().to_string());
        setup_command(&mut cmd, &config);
        if cfg!(windows) {
            cmd.args(["/C", "cd"]);
        }
        let output = cmd.output().await.expect("Failed to run pwd/cd");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected = cwd.to_string_lossy();
        assert!(
            stdout.contains(expected.as_ref()),
            "stdout should contain working dir"
        );
    }

    #[tokio::test]
    async fn setup_command_sets_env_vars_and_echoes() {
        let mut cmd = if cfg!(windows) {
            Command::new("cmd")
        } else {
            Command::new("sh")
        };
        let mut config = TaskConfig::new(if cfg!(windows) { "cmd" } else { "sh" });
        let mut envs = HashMap::new();
        envs.insert("FOO".to_string(), "BAR_TEST".to_string());
        config.env = Some(envs);
        setup_command(&mut cmd, &config);
        if cfg!(windows) {
            cmd.args(["/C", "echo %FOO%"]);
        } else {
            cmd.args(["-c", "echo $FOO"]);
        }
        let output = cmd.output().await.expect("Failed to run echo env");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("BAR_TEST"), "stdout should contain env var");
    }

    #[tokio::test]
    async fn setup_command_stdin_is_piped_if_enabled_and_accepts_input() {
        let mut cmd = if cfg!(windows) {
            Command::new("cmd")
        } else {
            Command::new("cat")
        };
        let mut config = TaskConfig::new(if cfg!(windows) { "cmd" } else { "cat" });
        config.enable_stdin = Some(true);
        setup_command(&mut cmd, &config);
        if cfg!(windows) {
            cmd.args(["/C", "more"]);
        }
        let mut child = cmd.spawn().expect("Failed to spawn");
        use tokio::io::AsyncWriteExt;
        let mut stdin = child.stdin.take().expect("No stdin");
        stdin
            .write_all(b"input_test\n")
            .await
            .expect("Failed to write");
        drop(stdin);
        let output = child
            .wait_with_output()
            .await
            .expect("Failed to get output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("input_test"), "stdout should contain input");
    }

    #[tokio::test]
    async fn setup_command_stdin_is_null_if_disabled_and_no_input() {
        let mut cmd = if cfg!(windows) {
            Command::new("cmd")
        } else {
            Command::new("cat")
        };
        let mut config = TaskConfig::new(if cfg!(windows) { "cmd" } else { "cat" });
        config.enable_stdin = Some(false);
        setup_command(&mut cmd, &config);
        if cfg!(windows) {
            cmd.args(["/C", "more"]);
        }
        let child = cmd.spawn().expect("Failed to spawn");
        // Should not have stdin available
        assert!(child.stdin.is_none(), "stdin should be None when disabled");
    }
}

use std::collections::HashMap;

use tokio::{io::AsyncWriteExt, process::Command};

use crate::tasks::{config::TaskConfig, tokio::spawn::direct::command::setup_command};

#[tokio::test]
async fn echo_with_args() {
    #[cfg(windows)]
    let mut cmd = Command::new("powershell");
    #[cfg(unix)]
    let mut cmd = Command::new("echo");

    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "echo hello_test"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["hello_test"])
        .use_process_group(false);

    setup_command(&mut cmd, &config);
    let output = cmd.output().await.expect("Failed to run echo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello_test"),
        "stdout should contain argument"
    );
}

#[tokio::test]
async fn echo_with_env() {
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(unix)]
    let mut cmd = Command::new("echo");

    let mut envs = HashMap::new();
    envs.insert("FOO".to_string(), "BAR_TEST".to_string());

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .env(envs)
        .args(["/C", "echo %FOO%"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .env(envs)
        .args(["$FOO"])
        .use_process_group(false);

    setup_command(&mut cmd, &config);

    let output = cmd.output().await.expect("Failed to run echo env");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BAR_TEST"), "stdout should contain env var");
}

#[tokio::test]
async fn stdin_enabled_accepts_input() {
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(unix)]
    let mut cmd = Command::new("cat");

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .enable_stdin(true)
        .args(["/C", "more"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("cat")
        .enable_stdin(true)
        .use_process_group(false);

    setup_command(&mut cmd, &config);

    let mut child = cmd.spawn().expect("Failed to spawn");
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
async fn stdin_disabled_no_input() {
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(unix)]
    let mut cmd = Command::new("cat");

    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .enable_stdin(false)
        .args(["/C", "more"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("cat")
        .enable_stdin(false)
        .use_process_group(false);

    setup_command(&mut cmd, &config);
    let child = cmd.spawn().expect("Failed to spawn");
    // Should not have stdin available
    assert!(child.stdin.is_none(), "stdin should be None when disabled");
}

use crate::tasks::{
    config::{TaskConfig, TaskShell},
    error::TaskError,
};

#[test]
fn validation() {
    // Valid config
    let config = TaskConfig::new("echo").args(["hello"]);
    assert!(config.validate().is_ok());

    // Empty command should fail
    let config = TaskConfig::new("");
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Zero timeout should fail
    let config = TaskConfig::new("echo").timeout_ms(0);
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Valid timeout should pass
    let config = TaskConfig::new("echo").timeout_ms(30);
    assert!(config.validate().is_ok());
}
#[test]
fn config_builder() {
    let config = TaskConfig::new("cargo")
        .args(["build", "--release"])
        .working_dir("/home/user/project")
        .env([("RUST_LOG", "debug"), ("CARGO_TARGET_DIR", "target")])
        .shell(TaskShell::Auto)
        .pty(true)
        .timeout_ms(300)
        .enable_stdin(true);

    assert_eq!(config.command, "cargo");
    assert_eq!(
        config.args,
        Some(vec!["build".to_string(), "--release".to_string()])
    );
    assert_eq!(config.working_dir, Some("/home/user/project".to_string()));
    assert!(config.env.is_some());
    assert_eq!(config.shell, Some(TaskShell::Auto));
    assert_eq!(config.pty, Some(true));
    assert_eq!(config.timeout_ms, Some(300));
    assert_eq!(config.enable_stdin, Some(true));
}

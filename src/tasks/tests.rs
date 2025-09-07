use std::{collections::HashMap, env::temp_dir};

use crate::tasks::{config::TaskConfig, error::TaskError};

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

    // Command with leading/trailing whitespace should fail
    let config = TaskConfig::new("  echo  ");
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

    // Arguments with empty string should fail
    let config = TaskConfig::new("echo").args([""]);
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Argument with leading/trailing whitespace should fail
    let config = TaskConfig::new("echo").args([" hello "]);
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Working directory that does not exist should fail
    let config = TaskConfig::new("echo").working_dir("/non/existent/dir");
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Working directory with temp dir should pass
    let dir = temp_dir();
    let config = TaskConfig::new("echo").working_dir(dir.as_path().to_str().unwrap());
    assert!(config.validate().is_ok());

    // Environment variable with empty key should fail
    let mut env = HashMap::new();
    env.insert(String::new(), "value".to_string());
    let config = TaskConfig::new("echo").env(env);
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Environment variable with space in key should fail
    let mut env = HashMap::new();
    env.insert("KEY WITH SPACE".to_string(), "value".to_string());
    let config = TaskConfig::new("echo").env(env);
    match config.validate() {
        Err(TaskError::InvalidConfiguration(_)) => {}
        other => panic!("unexpected result: {:?}", other),
    }

    // Environment variable key/value valid should pass
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "some value".to_string());
    let config = TaskConfig::new("echo").env(env);
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

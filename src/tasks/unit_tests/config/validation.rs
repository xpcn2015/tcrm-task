use std::{collections::HashMap, env::temp_dir};

use crate::tasks::{config::TaskConfig, error::TaskError};

#[test]
fn accept_basic_echo_command() {
    let config = TaskConfig::new("echo").args(["hello"]);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_command_with_args_and_working_dir() {
    let config = TaskConfig::new("ls")
        .args(["-la", "/tmp"])
        .working_dir(temp_dir().to_str().unwrap());
    assert!(config.validate().is_ok());
}

#[test]
fn accept_command_with_env_vars() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    let config = TaskConfig::new("env").env(env);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_command_with_timeout() {
    let config = TaskConfig::new("sleep").timeout_ms(1000);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_command_with_ready_indicator() {
    let config = TaskConfig::new("server")
        .ready_indicator("Server started")
        .ready_indicator_source(crate::tasks::config::StreamSource::Stdout);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_command_with_process_group_disabled() {
    let config = TaskConfig::new("cmd").use_process_group(false);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_command_with_stdin_enabled() {
    let config = TaskConfig::new("python").args(["-i"]).enable_stdin(true);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_valid_timeout() {
    let config = TaskConfig::new("echo").timeout_ms(30);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_existing_working_dir() {
    let dir = temp_dir();
    let config = TaskConfig::new("echo").working_dir(dir.as_path().to_str().unwrap());
    assert!(config.validate().is_ok());
}
#[test]
fn accept_valid_env_var_key_value() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "some value".to_string());
    let config = TaskConfig::new("echo").env(env);
    assert!(config.validate().is_ok());
}

#[test]
fn accept_ready_indicator_with_whitespace() {
    let mut config = TaskConfig::new("echo");
    config.ready_indicator = Some("  READY  ".to_string());
    assert!(config.validate().is_ok());
}

#[test]
fn accept_ready_indicator_normal() {
    let mut config = TaskConfig::new("echo");
    config.ready_indicator = Some("READY".to_string());
    assert!(config.validate().is_ok());
}

#[test]
fn reject_empty_command() {
    let config = TaskConfig::new("");
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_command_with_whitespace() {
    let config = TaskConfig::new("  echo  ");
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_command_exceeding_max_length() {
    let long_cmd = "a".repeat(4097);
    let config = TaskConfig::new(long_cmd);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_zero_timeout() {
    let config = TaskConfig::new("echo").timeout_ms(0);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_empty_argument() {
    let config = TaskConfig::new("echo").args([""]);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_argument_with_whitespace() {
    let config = TaskConfig::new("echo").args([" hello "]);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_argument_exceeding_max_length() {
    let long_arg = "a".repeat(4097);
    let config = TaskConfig::new("echo").args([long_arg]);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_nonexistent_working_dir() {
    let config = TaskConfig::new("echo").working_dir("/non/existent/dir");
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_working_dir_with_whitespace() {
    let dir = temp_dir();
    let dir_str = format!(" {} ", dir.as_path().to_str().unwrap());
    let config = TaskConfig::new("echo").working_dir(&dir_str);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_env_var_with_empty_key() {
    let mut env = HashMap::new();
    env.insert(String::new(), "value".to_string());
    let config = TaskConfig::new("echo").env(env);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_env_var_with_space_in_key() {
    let mut env = HashMap::new();
    env.insert("KEY WITH SPACE".to_string(), "value".to_string());
    let config = TaskConfig::new("echo").env(env);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_env_var_with_equal_in_key() {
    let mut env = HashMap::new();
    env.insert("KEY=BAD".to_string(), "value".to_string());
    let config = TaskConfig::new("echo").env(env);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_env_var_key_exceeding_max_length() {
    let mut env = HashMap::new();
    env.insert("A".repeat(1025), "value".to_string());
    let config = TaskConfig::new("echo").env(env);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_env_var_value_with_whitespace() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), " value ".to_string());
    let config = TaskConfig::new("echo").env(env);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_env_var_value_exceeding_max_length() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "A".repeat(4097));
    let config = TaskConfig::new("echo").env(env);
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

#[test]
fn reject_empty_ready_indicator() {
    let mut config = TaskConfig::new("echo");
    config.ready_indicator = Some(String::new());
    assert!(matches!(
        config.validate(),
        Err(TaskError::InvalidConfiguration(_))
    ));
}

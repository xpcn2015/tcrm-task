use crate::tasks::config::TaskConfig;

#[test]
fn basic() {
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

#[test]
fn env_hashmap() {
    use std::collections::HashMap;
    let mut env = HashMap::new();
    env.insert("FOO".to_string(), "bar".to_string());
    env.insert("BAZ".to_string(), "qux".to_string());
    let config = TaskConfig::new("env").env(env.clone());
    assert_eq!(config.env, Some(env));
}

#[test]
fn config_builder_timeout() {
    let config = TaskConfig::new("server").timeout_ms(5000);
    assert_eq!(config.timeout_ms, Some(5000));
}

#[test]
fn config_builder_ready_indicator_stdout() {
    let config = TaskConfig::new("server")
        .ready_indicator("Server started")
        .ready_indicator_source(crate::tasks::config::StreamSource::Stdout);
    assert_eq!(config.ready_indicator, Some("Server started".to_string()));
    assert_eq!(
        config.ready_indicator_source,
        Some(crate::tasks::config::StreamSource::Stdout)
    );
}
#[test]
fn config_builder_ready_indicator_stderr() {
    let config = TaskConfig::new("server")
        .ready_indicator("Server started")
        .ready_indicator_source(crate::tasks::config::StreamSource::Stderr);
    assert_eq!(config.ready_indicator, Some("Server started".to_string()));
    assert_eq!(
        config.ready_indicator_source,
        Some(crate::tasks::config::StreamSource::Stderr)
    );
}

#[test]
fn process_group_disabled() {
    let config = TaskConfig::new("cmd").use_process_group(false);
    assert_eq!(config.use_process_group, Some(false));
    assert!(!config.is_process_group_enabled());
}

#[test]
fn process_group_enabled_default() {
    let config = TaskConfig::new("cmd");
    assert_eq!(config.use_process_group, Some(true));
    assert!(config.is_process_group_enabled());
}

#[test]
fn disable_stdin_default() {
    let config = TaskConfig::new("python");
    assert_eq!(config.enable_stdin, Some(false));
}

#[test]
fn args_vec() {
    let config = TaskConfig::new("ls").args(vec!["-l", "/tmp"]);
    assert_eq!(
        config.args,
        Some(vec!["-l".to_string(), "/tmp".to_string()])
    );
}

#[test]
fn working_dir_none_by_default() {
    let config = TaskConfig::new("ls");
    assert_eq!(config.working_dir, None);
}

#[test]
fn env_none_by_default() {
    let config = TaskConfig::new("ls");
    assert_eq!(config.env, None);
}

#[test]
fn timeout_none_by_default() {
    let config = TaskConfig::new("ls");
    assert_eq!(config.timeout_ms, None);
}

#[test]
fn ready_indicator_none_by_default() {
    let config = TaskConfig::new("ls");
    assert_eq!(config.ready_indicator, None);
}

#[test]
fn ready_indicator_source_default() {
    let config = TaskConfig::new("ls");
    assert_eq!(
        config.ready_indicator_source,
        Some(crate::tasks::config::StreamSource::Stdout)
    );
}

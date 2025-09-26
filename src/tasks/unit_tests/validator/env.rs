use crate::tasks::validator::ConfigValidator;
use std::collections::HashMap;

#[test]
fn rejects_empty_key() {
    let mut env = HashMap::new();
    env.insert("".to_string(), "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_key_with_whitespace() {
    let mut env = HashMap::new();
    env.insert(" KEY ".to_string(), "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_key_exceeds_max_length() {
    let mut env = HashMap::new();
    let long_key = "K".repeat(1025); // MAX_ENV_KEY_LEN is 1024
    env.insert(long_key, "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_value_exceeds_max_length() {
    let mut env = HashMap::new();
    let long_value = "V".repeat(4097); // MAX_ENV_VALUE_LEN is 4096
    env.insert("KEY".to_string(), long_value);
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_key_with_null_byte() {
    let mut env = HashMap::new();
    env.insert("KEY\0BAD".to_string(), "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_key_with_tab_or_newline() {
    let mut env = HashMap::new();
    env.insert("KEY\tBAD".to_string(), "value".to_string());
    env.insert("KEY\nBAD".to_string(), "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn accepts_empty_value() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_ok());
}

#[test]
fn rejects_value_with_leading_trailing_whitespace() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), " value ".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_spaces_in_keys() {
    // Environment variable keys should not contain spaces
    let mut env = HashMap::new();
    env.insert("KEY WITH SPACE".to_string(), "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn accepts_normal_vars() {
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
fn rejects_invalid_keys() {
    let mut env = HashMap::new();
    env.insert("KEY=BAD".to_string(), "value".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

#[test]
fn rejects_null_chars() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "value\0with\0nulls".to_string());
    assert!(ConfigValidator::validate_env_vars(&env).is_err());
}

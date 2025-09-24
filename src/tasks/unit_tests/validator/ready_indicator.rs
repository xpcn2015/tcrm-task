use crate::tasks::validator::ConfigValidator;

#[test]
fn reject_empty() {
    assert!(ConfigValidator::validate_ready_indicator("").is_err());
}

#[test]
fn accept_whitespace() {
    assert!(ConfigValidator::validate_ready_indicator(" ").is_ok());
    assert!(ConfigValidator::validate_ready_indicator("   ").is_ok());
}

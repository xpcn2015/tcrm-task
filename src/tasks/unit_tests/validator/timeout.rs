use crate::tasks::validator::ConfigValidator;

#[test]
fn reject_0_timeout() {
    assert!(ConfigValidator::validate_timeout(&0).is_err());
}

#[test]
fn accept_positive_number_timeout() {
    assert!(ConfigValidator::validate_timeout(&1).is_ok());
}

use crate::tasks::validator::ConfigValidator;

#[test]
fn accepts_relative_paths() {
    let current_dir = std::env::current_dir().unwrap();
    assert!(ConfigValidator::validate_working_dir(current_dir.to_str().unwrap()).is_ok());

    // Relative path
    assert!(ConfigValidator::validate_working_dir(".").is_ok());
    assert!(ConfigValidator::validate_working_dir("..").is_ok());

    // Absolute path (platform-specific)
    #[cfg(unix)]
    {
        assert!(ConfigValidator::validate_working_dir("/tmp").is_ok());
        assert!(ConfigValidator::validate_working_dir("/").is_ok());
    }
    #[cfg(windows)]
    {
        use std::env;
        let system_root = env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        assert!(ConfigValidator::validate_working_dir(&system_root).is_ok());
    }
}

#[test]
fn rejects_nonexistent() {
    assert!(ConfigValidator::validate_working_dir("/nonexistent/path").is_err());
}

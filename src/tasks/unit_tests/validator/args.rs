use crate::tasks::validator::ConfigValidator;

#[test]
fn accepts_normal_args() {
    let normal_args = vec![
        "arg1".to_string(),
        "--flag".to_string(),
        "file.txt".to_string(),
        "path/to/file".to_string(),
        "--input=data.txt".to_string(),
        "-v".to_string(),
        "--output".to_string(),
        "123".to_string(),
        "'quoted arg'".to_string(),
        "\"double quoted arg\"".to_string(),
        "C:\\Program Files\\App".to_string(),
        "./script.sh".to_string(),
        "--env=PROD".to_string(),
        "--threads=4".to_string(),
        "--config=path/config.yaml".to_string(),
        "--user=admin".to_string(),
        "--password=secret".to_string(),
        "--dry-run".to_string(),
        "--verbose".to_string(),
    ];

    assert!(ConfigValidator::validate_args(&normal_args).is_ok());
}

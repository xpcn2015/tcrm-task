use crate::tasks::validator::ConfigValidator;

#[test]
fn reject_empty_commands() {
    assert!(ConfigValidator::validate_command("").is_err());
    assert!(ConfigValidator::validate_command("   ").is_err());
}

#[test]
fn accepts_shell_features() {
    // These should be allowed for developer tools
    let shell_commands = [
        "ls | grep pattern",
        "echo hello > output.txt",
        "make && npm test",
        "command1; command2",
        "echo $PATH",
        "ls $(pwd)",
        "cat file.txt | head -n 10",
        "sleep 1 & echo done",
        "(cd /tmp && ls)",
        "echo 'hello world'",
        "echo \"hello world\"",
        "echo hello\\ world",
        "echo hello # comment",
        "cat <<EOF\nhello\nEOF",
        "false || echo fallback",
        "echo hello >> file.txt",
        "echo `date`",
    ];

    for cmd in &shell_commands {
        assert!(
            ConfigValidator::validate_command(cmd).is_ok(),
            "Should accept shell feature: {}",
            cmd
        );
    }
}

#[test]
fn strict_mode_blocks_shell_features() {
    let shell_commands = [
        "ls | grep pattern",
        "echo hello > output.txt",
        "make && npm test",
        "command1; command2",
        "echo $PATH",
        "ls $(pwd)",
        "cat file.txt | head -n 10",
        "sleep 1 & echo done",
        "(cd /tmp && ls)",
        "cat <<EOF\nhello\nEOF",
        "false || echo fallback",
        "echo hello >> file.txt",
        "echo `date`",
        "echo hello # comment",
    ];

    for cmd in &shell_commands {
        assert!(
            ConfigValidator::validate_command_strict(cmd).is_err(),
            "Strict validation should reject: {}",
            cmd
        );
    }
}

#[test]
fn accepts_safe_commands() {
    let safe_commands = [
        "echo",
        "ls",
        "cat",
        "grep",
        "node",
        "python",
        "ls -la",
        "grep pattern file.txt",
        "node script.js",
        "python main.py --input data.txt",
        "cargo build --release",
        "npm install",
        "git status",
        "git commit -m 'update'",
        "echo 'hello world'",
        "echo \"hello world\"",
        "ls /usr/local/bin",
        "cat README.md",
        "grep -i error log.txt",
        "python -m venv .env",
        "node --version",
        "ls -lh /tmp",
        "cat file.txt | head -n 10",
        "echo $PATH",
    ];

    for cmd in &safe_commands {
        assert!(
            ConfigValidator::validate_command(cmd).is_ok(),
            "Should accept: {}",
            cmd
        );
    }
}

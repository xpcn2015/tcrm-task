use std::process::Stdio;

use tokio::process::Command;

use crate::tasks::config::TaskConfig;

/// Configures a `tokio::process::Command` based on the provided `TaskConfig`.
///
/// Sets arguments, working directory, environment, and stdio options.
pub(crate) fn setup_command(cmd: &mut Command, config: &TaskConfig) {
    // Setup additional arguments
    if let Some(args) = &config.args {
        cmd.args(args);
    }

    // Setup working directory with validation
    if let Some(dir) = &config.working_dir {
        cmd.current_dir(dir);
    }

    // Setup environment variables
    if let Some(envs) = &config.env {
        cmd.envs(envs);
    }

    // Setup stdio
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(
        if config.enable_stdin.unwrap_or(false) {
            Stdio::piped()
        } else {
            Stdio::null()
        },
    );

    // Kill child process on parent exit
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
}

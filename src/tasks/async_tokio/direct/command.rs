use std::process::Stdio;

use tokio::process::Command;
use tracing::error;

use crate::tasks::{config::TaskConfig, error::TaskError};

pub fn setup_command(cmd: &mut Command, config: &TaskConfig) -> Result<(), TaskError> {
    // Setup additional arguments
    if let Some(args) = &config.args {
        cmd.args(args);
    }

    // Setup working directory with validation
    if let Some(dir) = &config.working_dir {
        if !std::path::Path::new(dir).exists() {
            error!(dir, "Working directory does not exist");
            return Err(TaskError::InvalidConfiguration(
                "Working directory does not exist".to_string(),
            ));
        }
        cmd.current_dir(dir);
    }

    // Setup environment variables
    if let Some(envs) = &config.env {
        cmd.envs(envs);
    }

    // Setup stdio with better configuration
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

    Ok(())
}

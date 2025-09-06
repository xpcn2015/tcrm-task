use std::process::Stdio;

use tokio::process::Command;
use tracing::error;

use crate::tasks::{
    config::{TaskConfig, TaskShell},
    error::TaskError,
};

pub fn setup_command(cmd: &mut Command, config: &TaskConfig) -> Result<(), TaskError> {
    // Setup arguments for TaskShell::None
    if matches!(
        config.shell.as_ref().unwrap_or(&TaskShell::None),
        TaskShell::None
    ) {
        let parts: Vec<&str> = config.command.split_whitespace().collect();
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
    }

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
pub fn shell_command(config: &TaskConfig) -> Result<Command, TaskError> {
    let shell = config.shell.as_ref().unwrap_or(&TaskShell::None);

    let cmd = match shell {
        #[cfg(windows)]
        TaskShell::Cmd => {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&config.command);
            c
        }
        #[cfg(windows)]
        TaskShell::Powershell => {
            let mut c = Command::new("powershell");
            c.arg("-Command").arg(&config.command);
            c
        }
        #[cfg(unix)]
        TaskShell::Bash => {
            let mut c = Command::new("bash");
            c.arg("-c").arg(&config.command);
            c
        }
        TaskShell::None => {
            // Validate command exists
            let parts: Vec<&str> = config.command.split_whitespace().collect();
            if parts.is_empty() {
                return Err(TaskError::InvalidConfiguration("Empty command".to_string()));
            }
            Command::new(parts[0])
        }
        TaskShell::Auto => {
            #[cfg(windows)]
            {
                let mut c = Command::new("powershell");
                c.arg("-Command").arg(&config.command);
                c
            }
            #[cfg(unix)]
            {
                let mut c = Command::new("bash");
                c.arg("-c").arg(&config.command);
                c
            }
        }
    };
    Ok(cmd)
}

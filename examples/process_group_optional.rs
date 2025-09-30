//! Example demonstrating optional process group functionality.
//!
//! This example shows how to enable/disable process group management via TaskConfig

use tcrm_task::tasks::{config::TaskConfig, event::TaskEvent, tokio::executor::TaskExecutor};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Process Group Management Example");
    println!("================================\n");

    // Example 1: Process group ENABLED (default)
    println!("1. Testing with process group ENABLED (default):");

    #[cfg(windows)]
    let config_enabled = TaskConfig::new("cmd")
        .args(["/C", "echo", "Process group enabled"])
        .timeout_ms(5000);

    #[cfg(unix)]
    let config_enabled = TaskConfig::new("echo")
        .args(["Process group enabled"])
        .timeout_ms(5000);

    test_process_behavior("ProcessGroup ENABLED", config_enabled).await?;

    println!("\n{}\n", "=".repeat(50));

    // Example 2: Process group DISABLED
    println!("2. Testing with process group DISABLED:");

    #[cfg(windows)]
    let config_disabled = TaskConfig::new("cmd")
        .args(["/C", "echo", "Process group disabled"])
        .use_process_group(false)
        .timeout_ms(5000);

    #[cfg(unix)]
    let config_disabled = TaskConfig::new("echo")
        .args(["Process group disabled"])
        .use_process_group(false)
        .timeout_ms(5000);

    test_process_behavior("ProcessGroup DISABLED", config_disabled).await?;

    Ok(())
}

async fn test_process_behavior(
    test_name: &str,
    config: TaskConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running test: {}", test_name);

    config.validate()?;
    let mut executor = TaskExecutor::new(config);
    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);

    executor.coordinate_start(event_tx).await?;

    while let Some(event) = event_rx.recv().await {
        match event {
            TaskEvent::Started {
                process_id,
                created_at,
                ..
            } => {
                println!("  Started process {} at {:?}", process_id, created_at);
            }
            TaskEvent::Output { line, .. } => {
                println!("  Output: {}", line);
            }
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                ..
            } => {
                println!(
                    "  Stopped: {:?}, reason: {:?} at {:?}",
                    exit_code, reason, finished_at
                );
                break;
            }
            TaskEvent::Error { error, .. } => {
                eprintln!("  Error: {}", error);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

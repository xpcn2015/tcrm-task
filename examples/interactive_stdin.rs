//! Example: Simple process execution (stdin not yet implemented)
use tcrm_task::tasks::{config::TaskConfig, event::TaskEvent, tokio::executor::TaskExecutor};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "echo", "Hello from interactive example"])
        .timeout_ms(5000);
    #[cfg(unix)]
    let config = TaskConfig::new("echo")
        .args(["Hello from interactive example"])
        .timeout_ms(5000);

    config.validate()?;
    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);
    let mut executor = TaskExecutor::new(config, event_tx);
    executor.coordinate_start().await?;

    while let Some(event) = event_rx.recv().await {
        match event {
            TaskEvent::Output { line, .. } => println!("Output: {}", line),
            TaskEvent::Stopped {
                exit_code,
                reason,
                finished_at,
                ..
            } => {
                println!(
                    "Stopped: {:?}, reason: {:?} at {:?}",
                    exit_code, reason, finished_at
                );
                break;
            }
            TaskEvent::Error { error, .. } => eprintln!("Error: {}", error),
            _ => {}
        }
    }
    Ok(())
}

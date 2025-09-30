//! Basic example: Run a simple echo command and print output events
use tcrm_task::tasks::{config::TaskConfig, event::TaskEvent, tokio::executor::TaskExecutor};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let config = TaskConfig::new("cmd.exe")
        .args(["/C", "echo Hello!"])
        .timeout_ms(5000);
    #[cfg(unix)]
    let config = TaskConfig::new("bash")
        .args(["-c", "echo Hello!"])
        .timeout_ms(5000);

    let mut executor = TaskExecutor::new(config);
    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);
    executor.coordinate_start(event_tx).await?;
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

//! Example: Process with stdin interaction
use tcrm_task::tasks::{config::TaskConfig, event::TaskEvent, tokio::executor::TaskExecutor};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let config = TaskConfig::new("cmd")
        .args(["/C", "powershell", "-Command", "-"])
        .enable_stdin(true)
        .timeout_ms(10000);
    #[cfg(unix)]
    let config = TaskConfig::new("cat").enable_stdin(true).timeout_ms(10000);

    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);
    let mut executor = TaskExecutor::new(config, event_tx);

    // Start the process
    executor.coordinate_start().await?;

    // Send some input to stdin
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    if let Err(e) = executor.send_stdin("Hello from stdin!\n").await {
        eprintln!("Failed to send stdin: {}", e);
    }

    // Send another line
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    if let Err(e) = executor.send_stdin("Second line\n").await {
        eprintln!("Failed to send second stdin: {}", e);
    }

    // Process events
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
            TaskEvent::Started { process_id, .. } => {
                println!("Process started with PID: {}", process_id);
            }
            _ => {}
        }
    }
    Ok(())
}

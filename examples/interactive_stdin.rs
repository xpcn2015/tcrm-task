//! Example: Interactive process with stdin
use tcrm_task::tasks::{async_tokio::spawner::TaskSpawner, config::TaskConfig, event::TaskEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(10);
    // Use a native command for interactive stdin
    #[cfg(windows)]
    let config = TaskConfig::new("cmd.exe")
        .args(["/C", "more"])
        .enable_stdin(true)
        .timeout_ms(5000);
    #[cfg(unix)]
    let config = TaskConfig::new("bash")
        .args(["-c", "cat"])
        .enable_stdin(true)
        .timeout_ms(5000);
    let mut spawner = TaskSpawner::new(config).set_stdin(stdin_rx);
    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);
    let _pid = spawner.start_direct(event_tx).await?;
    // Send some lines to the process
    stdin_tx.send("Hello from Rust!".to_string()).await?;
    stdin_tx.send("Second line".to_string()).await?;
    while let Some(event) = event_rx.recv().await {
        match event {
            TaskEvent::Output { line, .. } => println!("Output: {}", line),
            TaskEvent::Stopped {
                exit_code, reason, ..
            } => {
                println!("Stopped: {:?}, reason: {:?}", exit_code, reason);
                break;
            }
            TaskEvent::Error { error, .. } => eprintln!("Error: {}", error),
            _ => {}
        }
    }
    Ok(())
}

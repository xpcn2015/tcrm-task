//! Example: Print tracing messages
use tcrm_task::tasks::{async_tokio::spawner::TaskSpawner, config::TaskConfig, event::TaskEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    #[cfg(windows)]
    let config = TaskConfig::new("cmd.exe")
        .args(["/C", "echo Hello!"])
        .timeout_ms(5000);
    #[cfg(unix)]
    let config = TaskConfig::new("bash")
        .args(["-c", "echo Hello!"])
        .timeout_ms(5000);

    let mut spawner = TaskSpawner::new(config);
    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);
    let _pid = spawner.start_direct(event_tx).await?;
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

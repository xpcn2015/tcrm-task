//! Example: Interactive process with stdin
use tcrm_task::tasks::{async_tokio::spawner::TaskSpawner, config::TaskConfig, event::TaskEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(10);
    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "'ready'; $host.UI.ReadLine()"])
        .enable_stdin(true)
        .ready_indicator("ready")
        .timeout_ms(5000);
    #[cfg(unix)]
    let config = TaskConfig::new("sh")
        .args(["-c", "echo 'ready'; read input; echo $input"])
        .enable_stdin(true)
        .ready_indicator("ready")
        .timeout_ms(5000);
    let mut spawner = TaskSpawner::new(config).set_stdin(stdin_rx);
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
            TaskEvent::Ready => {
                println!("Task is ready, sending input");
                stdin_tx.send("Hello from Rust!".to_string()).await?;
            }
            _ => {}
        }
    }
    Ok(())
}

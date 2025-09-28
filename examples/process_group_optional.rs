//! Example demonstrating optional process group functionality.
//!
//! This example shows:
//! 1. How to enable/disable process group management via TaskConfig
//! 2. The difference in behavior between process group enabled vs disabled
//! 3. How child processes are handled in both scenarios

use tcrm_task::tasks::{tokio::spawn::spawner::TaskSpawner, config::TaskConfig, event::TaskEvent};
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better visibility
    tracing_subscriber::fmt::init();

    println!("🔧 Process Group Management Example");
    println!("===================================\n");

    // Example 1: Process group ENABLED (default)
    println!("1️⃣  Testing with process group ENABLED (default):");

    #[cfg(windows)]
    let config_enabled = TaskConfig::new("powershell")
        .args(vec![
            "-Command".to_string(),
            "ping 127.0.0.1 -n 20".to_string(),
        ])
        // .use_process_group(true) - This is the default, so we don't need to set it
        .timeout_ms(5000);

    #[cfg(unix)]
    let config_enabled = TaskConfig::new("bash")
        .args(vec![
            "-c".to_string(),
            // Use ping to simulate a long-running process, similar to Windows
            "ping 127.0.0.1 -c 20".to_string(),
        ])
        // .use_process_group(true) - This is the default, so we don't need to set it
        .timeout_ms(5000);

    test_process_behavior("ProcessGroup ENABLED", config_enabled).await?;

    println!("\n{}\n", "=".repeat(50));

    // Example 2: Process group DISABLED
    println!("2️⃣  Testing with process group DISABLED:");

    #[cfg(windows)]
    let config_disabled = TaskConfig::new("powershell")
        .args(vec![
            "-Command".to_string(),
            "ping 127.0.0.1 -n 20".to_string(),
        ])
        .use_process_group(false) // Explicitly disable process group
        .timeout_ms(5000);

    #[cfg(unix)]
    let config_disabled = TaskConfig::new("bash")
        .args(vec![
            "-c".to_string(),
            // Use ping to simulate a long-running process, similar to Windows
            "ping 127.0.0.1 -c 20".to_string(),
        ])
        .use_process_group(false) // Explicitly disable process group
        .timeout_ms(5000);

    test_process_behavior("ProcessGroup DISABLED", config_disabled).await?;

    println!("\n✅ Process group configuration examples completed!");
    println!("\n📝 Key Differences:");
    println!(
        "   • You can change use_process_group to `true` in Example 2, and the terminal will not hang."
    );
    println!("   • ENABLED: Uses cross-platform process groups for child termination.");
    println!("   • DISABLED: Direct process management, which may leave orphaned processes.");

    Ok(())
}

async fn test_process_behavior(
    test_name: &str,
    config: TaskConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   Starting task: {}", test_name);

    let mut spawner = TaskSpawner::new(config);

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    // Start the task
    spawner.start_direct(event_tx).await?;

    // Monitor events
    let mut events_received = 0;
    while let Some(event) = event_rx.recv().await {
        events_received += 1;
        match event {
            TaskEvent::Started => {
                println!("   🚀 Task started",);
            }
            TaskEvent::Ready => {
                println!("   ✅ Task ready",);
            }
            TaskEvent::Output { line, src } => {
                println!("   📤 Output [{:?}]({})", src, line);
            }
            TaskEvent::Stopped { exit_code, reason } => {
                println!(
                    "   🛑 Task stopped - Exit: {:?}, Reason: {:?}",
                    exit_code, reason
                );
                break;
            }
            TaskEvent::Error { error } => {
                println!("   ❌ Task error: {}", error);
                break;
            }
        }
    }

    println!("   📊 Total events received: {}", events_received);

    // Give a moment for cleanup
    sleep(Duration::from_millis(100)).await;

    Ok(())
}

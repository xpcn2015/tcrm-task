use tokio::sync::mpsc;

#[cfg(windows)]
use crate::tasks::config::TaskConfig;
use crate::tasks::{async_tokio::spawner::TaskSpawner, config::StreamSource, event::TaskEvent};

#[tokio::test]
async fn stdin_valid() {
    // Channel for receiving task events
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(1024);

    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "$line = Read-Host; Write-Output $line"])
        .enable_stdin(true)
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("head")
        .args(["-n", "1"])
        .enable_stdin(true)
        .use_process_group(false);

    let mut spawner = TaskSpawner::new(config).set_stdin(stdin_rx);

    // Spawn the task
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    // Send input via stdin if enabled
    stdin_tx.send("hello world".to_string()).await.unwrap();

    let mut started = false;
    let mut output_ok = false;
    let mut stopped = false;

    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started => {
                started = true;
            }
            TaskEvent::Output { line, src } => {
                assert_eq!(line, "hello world");
                assert_eq!(src, StreamSource::Stdout);
                output_ok = true;
            }
            TaskEvent::Stopped { exit_code, .. } => {
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(output_ok);
    assert!(stopped);
}

#[tokio::test]
async fn stdin_ignore() {
    // Channel for receiving task events
    let (tx, mut rx) = mpsc::channel::<TaskEvent>(1024);
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(1024);

    #[cfg(windows)]
    let config = TaskConfig::new("powershell")
        .args(["-Command", "$line = Read-Host; Write-Output $line"])
        .use_process_group(false);
    #[cfg(unix)]
    let config = TaskConfig::new("head")
        .args(["-n", "1"])
        .use_process_group(false);

    // Note: stdin is not enabled in config, so stdin should be ignored
    let mut spawner = TaskSpawner::new(config).set_stdin(stdin_rx);

    // Spawn the task
    let result = spawner.start_direct(tx).await;
    assert!(result.is_ok());

    // Send input, but it should be ignored (receiver will be dropped, so this should error)
    let send_result = stdin_tx.send("hello world".to_string()).await;
    assert!(
        send_result.is_err(),
        "Sending to stdin_tx should error because receiver is dropped"
    );

    let mut started = false;
    let mut output_found = false;
    let mut stopped = false;

    while let Some(event) = rx.recv().await {
        match event {
            TaskEvent::Started => {
                started = true;
            }
            TaskEvent::Output { .. } => {
                // Should NOT receive output from stdin
                output_found = true;
            }
            TaskEvent::Stopped { exit_code, .. } => {
                assert_eq!(exit_code, Some(0));
                stopped = true;
            }
            _ => {}
        }
    }

    assert!(started);
    assert!(
        !output_found,
        "Should not receive output from stdin when not enabled"
    );
    assert!(stopped);
}

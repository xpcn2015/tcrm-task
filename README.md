# TCRM Task

Task execution unit for the TCRM project

## Features

- **Asynchronous Execution**: Built on Tokio for async task execution
- **Task Timeout**: Configurable execution timeout
- **Event System**: Real-time monitoring of task lifecycle and output
- **Optional Tracing/Logging**: Enable structured logging with the `tracing` Cargo feature

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tcrm-task = { version = "0.1.0" }

```

## Quick Start

### Basic Task Execution

```rust
use tcrm_task::tasks::{
    config::TaskConfig,
    async_tokio::spawner::TaskSpawner,
    event::TaskEvent,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = if cfg!(windows) {
        TaskConfig::new("powershell")
            .args(["-Command", "echo Hello from Windows!"])
            .timeout_ms(5000)
    } else {
        TaskConfig::new("bash")
            .args(["-c", "echo Hello from Unix!"])
            .timeout_ms(5000)
    };

    let mut spawner = TaskSpawner::new("hello_task".to_string(), config);

    // Create an event channel to receive task events
    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(100);

    // Start the task
    let process_id = spawner.start_direct(event_tx).await?;
    println!("Started process with ID: {}", process_id);

    // Listen for events
    while let Some(event) = event_rx.recv().await {
        match event {
            TaskEvent::Started { task_name } => println!("Task '{}' started", task_name),
            TaskEvent::Output { task_name, line, src } => println!("Task '{}' output ({:?}): {}", task_name, src, line),
            TaskEvent::Stopped { task_name, exit_code, reason } => {
                println!("Task '{}' stopped with exit code {:?}, reason: {:?}", task_name, exit_code, reason);
                break;
            }
            TaskEvent::Error { task_name, error } => eprintln!("Task '{}' error: {}", task_name, error),
            _ => {}
        }
    }
    Ok(())
}
```

### More Configuration

```rust
use tcrm_task::tasks::config::TaskConfig;
use std::collections::HashMap;

let config = TaskConfig::new("cargo")
    .args(["build", "--release"])
    .working_dir("/path/to/project")
    .env([
        ("RUST_LOG", "debug"),
        ("CARGO_TARGET_DIR", "target")
    ])
    .timeout_ms(30000)  // 30 seconds
    .enable_stdin(true);

// Validate configuration before use
config.validate()?;
```

### Task with Stdin Input

```rust
use tokio::sync::mpsc;

// Create stdin channel
let (stdin_tx, stdin_rx) = mpsc::channel::<String>(10);
let config = if cfg!(windows) {
    TaskConfig::new("powershell")
        .args(["-Command", "cat"])
        .enable_stdin(true)
} else {
    TaskConfig::new("cat")
        .enable_stdin(true)
};
let mut spawner = TaskSpawner::new("cat_task".to_string(), config)
    .set_stdin(stdin_rx);
stdin_tx.send("Hello from stdin!".to_string()).await?;
```

## Task States

Tasks progress through the following states:

- **Pending**: Task is created but not yet started
- **Initiating**: Task is being prepared for execution
- **Running**: Task is actively executing
- **Ready**: Task is running and ready (for long-running processes)
- **Finished**: Task has completed execution

## Event System

The library provides real-time events for task monitoring:

```rust
use tcrm_task::tasks::event::{TaskEvent, TaskEventStopReason};

match event {
    TaskEvent::Started { task_name } => {
        // Task has started
    }
    TaskEvent::Output { task_name, line, src } => {
        // New output line from stdout or stderr
    }
    TaskEvent::Ready { task_name } => {
        // Task is ready
    }
    TaskEvent::Stopped { task_name, exit_code, reason } => {
        // Task has stopped
        match reason {
            TaskEventStopReason::Finished => println!("Task completed normally"),
            TaskEventStopReason::Terminated(reason) => println!("Task terminated: {:?}", reason),
            TaskEventStopReason::Error(err) => println!("Task failed: {}", err),
        }
    }
    TaskEvent::Error { task_name, error } => {
        // Task encountered an error
    }
}
```

## Features

### Default Features

- `tokio`: Enables async functionality (enabled by default)

### Optional Features

- `flatbuffers`: Enables FlatBuffers serialization support
- `tracing`: Enables structured logging/tracing macros

## Examples

See the `examples/` directory for:
- Basic process execution
- Interactive process with stdin
- Configuration validation
- Tracing/logging output

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run tests with logging/tracing
RUST_LOG=debug cargo test --features tracing

# Run specific test module
cargo test tasks::tests
```

## License

This project is licensed under either the MIT or Apache-2.0 License, at your option.

See LICENSE file for details.

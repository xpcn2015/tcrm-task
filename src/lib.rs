//! # tcrm-task
//!
//! A process execution library.
//!
//! ## Features
//!
//! - **Real-time Events**: Monitor process output, state changes, and lifecycle events
//! - **Timeout**: Configurable process execution timeouts
//! - **Ready Indicators**: Detect when long-running processes are ready to accept requests via output matching
//! - **Process control**: Cross-platform signal sending and process control
//! - **Process Groups**: Optional feature for managing all child processes via groups/job objects
//!
//! ## Quick Start
//!
//! ```rust
//! use tcrm_task::tasks::{config::TaskConfig, tokio::executor::TaskExecutor};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and validate configuration
//!     #[cfg(windows)]
//!     let config = TaskConfig::new("cmd").args(["/C", "echo", "Hello, World!"]);
//!     #[cfg(unix)]
//!     let config = TaskConfig::new("echo").args(["Hello, World!"]);
//!
//!     // Create executor and event channel
//!     let mut executor = TaskExecutor::new(config);
//!     let (tx, mut rx) = mpsc::channel(100);
//!     
//!     // Spawns a new asynchronous task
//!     executor.coordinate_start(tx).await?;
//!
//!     // Process events
//!     while let Some(event) = rx.recv().await {
//!         match event {
//!             tcrm_task::tasks::event::TaskEvent::Started { process_id, .. } => {
//!                 println!("Process started with ID: {}", process_id);
//!             }
//!             tcrm_task::tasks::event::TaskEvent::Output { line, .. } => {
//!                 println!("Output: {}", line);
//!             }
//!             tcrm_task::tasks::event::TaskEvent::Stopped { exit_code, .. } => {
//!                 println!("Process finished with exit code: {:?}", exit_code);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Long-running Process with Ready Indicator
//!
//! ```rust
//! use tcrm_task::tasks::{
//!     config::{TaskConfig, StreamSource},
//!     tokio::executor::TaskExecutor,
//!     event::TaskEvent
//! };
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     #[cfg(windows)]
//!     let config = TaskConfig::new("cmd")
//!         .args(["/C", "echo", "Server listening on port 3000"])
//!         .ready_indicator("Server listening")
//!         .ready_indicator_source(StreamSource::Stdout)
//!         .timeout_ms(30000);
//!     
//!     #[cfg(unix)]
//!     let config = TaskConfig::new("echo")
//!         .args(["Server listening on port 3000"])
//!         .ready_indicator("Server listening")
//!         .ready_indicator_source(StreamSource::Stdout)
//!         .timeout_ms(30000);
//!
//!     let mut executor = TaskExecutor::new(config);
//!     let (tx, mut rx) = mpsc::channel(100);
//!     
//!     executor.coordinate_start(tx).await?;
//!
//!     // Wait for ready event
//!     while let Some(event) = rx.recv().await {
//!         match event {
//!             TaskEvent::Ready => {
//!                 println!("Server is ready for requests!");
//!                 break;
//!             }
//!             TaskEvent::Output { line, .. } => {
//!                 println!("Server log: {}", line);
//!             }
//!             TaskEvent::Error { error } => {
//!                 eprintln!("Error: {}", error);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Process Control and Termination
//!
//! ```rust
//! use tcrm_task::tasks::{
//!     config::TaskConfig,
//!     tokio::executor::TaskExecutor,
//!     control::TaskControl,
//!     event::TaskTerminateReason
//! };
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     #[cfg(windows)]
//!     let config = TaskConfig::new("cmd")
//!         .args(["/C", "timeout", "/t", "10"])
//!         .timeout_ms(5000); // 5 second timeout
//!     
//!     #[cfg(unix)]    
//!     let config = TaskConfig::new("sleep")
//!         .args(["10"])
//!         .timeout_ms(5000); // 5 second timeout
//!
//!     let mut executor = TaskExecutor::new(config);
//!     let (tx, mut rx) = mpsc::channel(100);
//!     
//!     executor.coordinate_start(tx).await?;
//!
//!     // Terminate after 2 seconds
//!     tokio::spawn(async move {
//!         tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
//!         let _ = executor.terminate_task(TaskTerminateReason::UserRequested);
//!     });
//!
//!     // Process events until completion
//!     while let Some(event) = rx.recv().await {
//!         match event {
//!             tcrm_task::tasks::event::TaskEvent::Stopped { reason, .. } => {
//!                 println!("Process stopped: {:?}", reason);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - `tokio`: Async runtime support (default)
//! - `tokio-coordinate`: Full coordination module (default)
//! - `process-group`: Process group management (default)
//! - `signal`: Sending signals to processes
//! - `serde`: Serialization support for all types
//! - `flatbuffers`: High-performance serialization
//! - `tracing`: Structured logging integration

#[cfg(feature = "flatbuffers")]
pub mod flatbuffers;
pub mod helper;
pub mod tasks;

//! # tcrm-task
//!
//! A Rust library for executing and managing system processes.
//! Built for developers who need process execution with
//! validation and real-time event monitoring.
//!
//! ## Features
//!
//! - **Real-time Events**: Monitor process output, state changes, and lifecycle events
//! - **Timeout Management**: Configurable timeouts for process execution
//! - **Stdin Support**: Send input to running processes
//! - **Ready Indicators**: Detect when long-running processes are ready to accept requests
//! - **Serialization**: Optional serde and flatbuffers support for persistence
//!
//! ## Quick Start
//!
//! ```rust
//! use tcrm_task::tasks::{config::TaskConfig, tokio::spawn::spawner::TaskSpawner};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a simple command configuration
//!     let config = TaskConfig::new("cmd")
//!         .args(["/C", "echo", "Hello, World!"]);
//!
//!     // Validate the configuration
//!     config.validate()?;
//!
//!     // Create a spawner and execute the task
//!     let (tx, mut rx) = mpsc::channel(100);
//!     let mut spawner = TaskSpawner::new("hello".to_string(), config);
//!     
//!     spawner.start_direct(tx).await?;
//!
//!     // Process events
//!     while let Some(event) = rx.recv().await {
//!         println!("Event: {:?}", event);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Advanced Usage
//!
//! ### Long-running Process with Ready Indicator
//!
//! ```rust
//! use tcrm_task::tasks::{config::{TaskConfig, StreamSource}, tokio::spawn::spawner::TaskSpawner};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = TaskConfig::new("cmd")
//!         .args(["/C", "echo", "Server listening on"])
//!         .ready_indicator("Server listening on")
//!         .ready_indicator_source(StreamSource::Stdout)
//!         .timeout_ms(30000);
//!
//!     let (tx, mut rx) = mpsc::channel(100);
//!     let mut spawner = TaskSpawner::new("server".to_string(), config);
//!     
//!     spawner.start_direct(tx).await?;
//!
//!     // Wait for ready event
//!     while let Some(event) = rx.recv().await {
//!         if matches!(event, tcrm_task::tasks::event::TaskEvent::Ready { .. }) {
//!             println!("Server is ready!");
//!             break;
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Process with Environment Variables and Working Directory
//!
//! ```rust
//! use tcrm_task::tasks::{config::TaskConfig, tokio::spawn::spawner::TaskSpawner};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut env = HashMap::new();
//!     env.insert("RUST_LOG".to_string(), "debug".to_string());
//!     env.insert("APP_ENV".to_string(), "production".to_string());
//!
//!     let config = TaskConfig::new("cmd")
//!         .args(["/C", "dir"])
//!         .working_dir("C:\\")
//!         .env(env)
//!         .timeout_ms(300000); // 5 minutes
//!
//!     config.validate()?;
//!     
//!     // ... execute task
//!     Ok(())
//! }
//! ```
//!
//! ## Validation
//!
//! This library includes validation to prevent:
//! - Path traversal  
//! - Null byte
//!
//! All configurations are validated before execution using the built-in validator.
//!
//! ## Optional Features
//!
//! - `serde`: Enable serialization support for all types
//! - `flatbuffers`: Enable `FlatBuffers` serialization for high-performance scenarios
//! - `tracing`: Enable structured logging integration

#[cfg(feature = "flatbuffers")]
pub mod flatbuffers;
pub mod helper;
pub mod tasks;

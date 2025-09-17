//! `FlatBuffers` serialization support for high-performance scenarios
//!
//! This module provides `FlatBuffers` serialization and deserialization for all
//! task-related types, enabling efficient data transfer and persistence in
//! performance-critical applications.
//!
//! `FlatBuffers` offers several advantages over JSON and other serialization formats:
//! - Zero-copy deserialization for optimal performance
//! - Compact binary format reducing storage and bandwidth requirements
//! - Schema evolution support for backwards/forwards compatibility
//! - Cross-platform compatibility with consistent byte ordering
//!
//! # Feature Requirements
//!
//! This module is only available when the `flatbuffers` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! tcrm-task = { version = "0.3", features = ["flatbuffers"] }
//! ```
//!
//! # Examples
//!
//! ## Basic Config Serialization
//! ```rust
//! # #[cfg(feature = "flatbuffers")]
//! # {
//! use tcrm_task::tasks::config::TaskConfig;
//! use flatbuffers::FlatBufferBuilder;
//!
//! let config = TaskConfig::new("cmd").args(["/C", "echo", "hello"]);
//!
//! // Convert to FlatBuffer
//! let mut builder = FlatBufferBuilder::new();
//! let fb_config = config.to_flatbuffers(&mut builder);
//! builder.finish(fb_config, None);
//! let bytes = builder.finished_data();
//! println!("Serialized {} bytes", bytes.len());
//! # }
//! ```
//!
//! ## State Serialization
//! ```rust
//! # #[cfg(feature = "flatbuffers")]
//! # {
//! use tcrm_task::tasks::event::TaskTerminateReason;
//! use flatbuffers::FlatBufferBuilder;
//!
//! let reason = TaskTerminateReason::Timeout;
//!
//! // Serialize terminate reason for logging
//! let mut builder = FlatBufferBuilder::new();
//! let (fb_reason, _offset) = reason.to_flatbuffers(&mut builder);
//! println!("Reason serialized: {:?}", fb_reason);
//! # }
//! ```

pub mod conversion;
#[allow(dead_code, unused_imports)]
#[path = "tcrm_task_generated.rs"]
pub mod tcrm_task_generated;

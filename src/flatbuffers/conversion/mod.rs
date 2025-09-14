//! Conversion traits and implementations for `FlatBuffers` serialization
//!
//! This module provides traits and implementations for converting between
//! Rust types and `FlatBuffers` representation. Each module corresponds to
//! a major type family in the task execution system.
//!
//! # Available Conversions
//!
//! - **config**: `TaskConfig` and related configuration types
//! - **error**: `TaskError` and error handling types  
//! - **event**: `TaskEvent` and all event variants
//! - **state**: `TaskState` and `TaskTerminateReason` types
//!
//! # Conversion Pattern
//!
//! All types implement a consistent pattern for `FlatBuffers` conversion:
//!
//! ```rust
//! use tcrm_task::tasks::config::TaskConfig;
//! use flatbuffers::FlatBufferBuilder;
//!
//! // Example of converting a TaskConfig to FlatBuffers
//! let config = TaskConfig::new("echo");
//! let mut builder = FlatBufferBuilder::new();
//! let fb_offset = config.to_flatbuffers(&mut builder);
//! builder.finish(fb_offset, None);
//! let bytes = builder.finished_data();
//!
//! // Example of converting bytes back to TaskConfig
//! let fb_config = flatbuffers::root::<tcrm_task::flatbuffers::tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
//! let restored_config = TaskConfig::try_from(fb_config).unwrap();
//! assert_eq!(config.command, restored_config.command);
//! ```
//!
//! This ensures type-safe, efficient serialization with proper error handling
//! for malformed data or version mismatches.

pub mod config;
pub mod error;
pub mod event;
pub mod state;

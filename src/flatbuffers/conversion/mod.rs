//! Conversion traits and implementations for FlatBuffers serialization
//!
//! This module provides traits and implementations for converting between
//! Rust types and FlatBuffers representation. Each module corresponds to
//! a major type family in the task execution system.
//!
//! # Available Conversions
//!
//! - **config**: TaskConfig and related configuration types
//! - **error**: TaskError and error handling types  
//! - **event**: TaskEvent and all event variants
//! - **state**: TaskState and TaskTerminateReason types
//!
//! # Conversion Pattern
//!
//! All types implement a consistent pattern:
//!
//! ```rust,ignore
//! trait FlatbufferConversion<T> {
//!     fn to_flatbuffer_bytes(&self) -> Result<Vec<u8>, ConversionError>;
//!     fn from_flatbuffer_bytes(bytes: &[u8]) -> Result<T, ConversionError>;
//! }
//! ```
//!
//! This ensures type-safe, efficient serialization with proper error handling
//! for malformed data or version mismatches.

pub mod config;
pub mod error;
pub mod event;
pub mod state;

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
//! use tcrm_task::flatbuffers::conversion::ToFlatbuffers;
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

#[cfg(test)]
mod unit_tests;

// Re-export the ConversionError for use in traits
pub use error::ConversionError;
// Re-export FlatBufferBuilder for trait definitions
pub use flatbuffers::FlatBufferBuilder;

/// Trait for converting from `FlatBuffers` format back to Rust types.
///
/// This trait provides a standardized interface for deserializing `FlatBuffers`
/// data back into Rust types. It handles validation and error reporting during
/// the conversion process.
///
/// # Examples
///
/// ```rust
/// use tcrm_task::flatbuffers::conversion::FromFlatbuffers;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Assuming you have FlatBuffers data and the appropriate generated type
/// // let fb_data = /* ... */;
/// // let rust_value = MyType::from_flatbuffers(fb_data)?;
/// # Ok(())
/// # }
/// ```
pub trait FromFlatbuffers<T> {
    /// Convert from `FlatBuffers` format to Rust type.
    ///
    /// # Parameters
    ///
    /// * `fb_data` - The `FlatBuffers` data to convert from
    ///
    /// # Returns
    ///
    /// Returns the Rust type, or a [`ConversionError`] if conversion fails.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError`] if the `FlatBuffers` data is invalid or corrupted.
    fn from_flatbuffers(fb_data: T) -> Result<Self, ConversionError>
    where
        Self: Sized;
}

/// Trait for converting Rust types to `FlatBuffers` format.
///
/// This trait provides a standardized interface for converting task monitor
/// configuration types into their `FlatBuffers` representation. It handles
/// serialization into a compact, cross-platform binary format.
///
/// # Type Parameters
///
/// * `'a` - Lifetime parameter for the `FlatBufferBuilder` reference
///
/// # Associated Types
///
/// * `Output` - The `FlatBuffers` type that this conversion produces
///
/// # Examples
///
/// ```rust
/// use tcrm_task::flatbuffers::conversion::ToFlatbuffers;
/// use tcrm_task::tasks::config::TaskConfig;
/// use flatbuffers::FlatBufferBuilder;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = TaskConfig::new("cargo").args(["test"]);
///
/// let mut fbb = FlatBufferBuilder::new();
/// let fb_config = config.to_flatbuffers(&mut fbb);
///
/// // The resulting fb_config can now be used to build the final FlatBuffer
/// # Ok(())
/// # }
/// ```
pub trait ToFlatbuffers<'a> {
    /// The `FlatBuffers` type produced by this conversion
    type Output;

    /// Convert this type to its `FlatBuffers` representation.
    ///
    /// # Parameters
    ///
    /// * `fbb` - Mutable reference to the `FlatBufferBuilder` for serialization
    ///
    /// # Returns
    ///
    /// Returns the `FlatBuffers` offset for the serialized data.
    fn to_flatbuffers(&self, fbb: &mut FlatBufferBuilder<'a>) -> Self::Output;
}

/// Trait for converting Rust types to `FlatBuffers` union format.
///
/// This trait handles the specific case of `FlatBuffers` unions, which require
/// both a discriminant value and the union data offset.
///
/// # Type Parameters
///
/// * `'a` - Lifetime parameter for the `FlatBufferBuilder` reference
/// * `Discriminant` - The union discriminant type
///
/// # Examples
///
/// ```rust
/// use tcrm_task::flatbuffers::conversion::ToFlatbuffersUnion;
/// use flatbuffers::FlatBufferBuilder;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Example with union type conversion
/// // let union_value = SomeUnionType::Variant(data);
/// // let mut fbb = FlatBufferBuilder::new();
/// // let (discriminant, offset) = union_value.to_flatbuffers_union(&mut fbb);
/// # Ok(())
/// # }
/// ```
pub trait ToFlatbuffersUnion<'a, Discriminant> {
    /// Convert this type to its `FlatBuffers` union representation.
    ///
    /// # Parameters
    ///
    /// * `fbb` - Mutable reference to the `FlatBufferBuilder` for serialization
    ///
    /// # Returns
    ///
    /// Returns a tuple of (discriminant, `union_offset`) for the serialized data.
    fn to_flatbuffers_union(
        &self,
        fbb: &mut FlatBufferBuilder<'a>,
    ) -> (
        Discriminant,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    );
}

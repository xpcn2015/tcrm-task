use std::future::Future;

#[cfg(feature = "tracing")]
use tracing::{Instrument, Level};

/// Conditionally instruments futures with tracing spans.
///
/// This trait provides a unified interface for adding tracing instrumentation
/// to async operations. When the "tracing" feature is enabled, futures are
/// wrapped with debug-level spans. When disabled, futures are returned unchanged.
pub trait MaybeInstrument: Future + Sized {
    /// Conditionally instruments the future with a tracing span.
    ///
    /// # Arguments
    ///
    /// * `name` - Static name for the tracing span.
    ///
    /// # Returns
    ///
    /// When tracing is enabled: An instrumented future with a debug-level span.
    /// When tracing is disabled: The original future unchanged.
    #[cfg(feature = "tracing")]
    fn maybe_instrument(self, name: &'static str) -> impl Future<Output = Self::Output> {
        let span = tracing::span!(Level::DEBUG, "async_op", name = name);
        self.instrument(span)
    }

    /// No-op implementation when tracing is disabled.
    ///
    /// # Arguments
    ///
    /// * `_name` - Ignored static name parameter.
    ///
    /// # Returns
    ///
    /// The original future unchanged.
    #[cfg(not(feature = "tracing"))]
    #[must_use]
    fn maybe_instrument(self, _name: &'static str) -> Self {
        self
    }
}

/// Blanket implementation for all Future types.
impl<F: Future> MaybeInstrument for F {}

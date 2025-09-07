use std::future::Future;

#[cfg(feature = "tracing")]
use tracing::{Instrument, Level};

pub trait MaybeInstrument: Future + Sized {
    #[cfg(feature = "tracing")]
    fn maybe_instrument(self, name: &'static str) -> impl Future<Output = Self::Output> {
        let span = tracing::span!(Level::DEBUG, "async_op", name = name);
        self.instrument(span)
    }

    #[cfg(not(feature = "tracing"))]
    fn maybe_instrument(self, _name: &'static str) -> Self {
        self
    }
}

impl<F: Future> MaybeInstrument for F {}

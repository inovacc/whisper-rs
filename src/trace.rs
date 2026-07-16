//! Internal zero-cost tracing facade.
//!
//! `trace_debug!` / `trace_info!` forward to the [`tracing`](https://docs.rs/tracing) crate when the
//! `tracing` feature is enabled, and compile to a no-op (that still type-checks its format args)
//! otherwise. Instrumentation call sites therefore carry no `#[cfg]` of their own, and the default
//! build pulls in no logging dependency and installs no subscriber.
//!
//! To get logs, a consumer enables `features = ["tracing"]` and installs any `tracing` subscriber
//! (e.g. `tracing_subscriber::fmt::init()`) in their application. Field syntax is intentionally not
//! used — call sites pass plain format strings so the same tokens type-check in the no-op branch.

#[cfg(feature = "tracing")]
macro_rules! trace_debug {
    ($($arg:tt)*) => { ::tracing::debug!($($arg)*) };
}
#[cfg(not(feature = "tracing"))]
macro_rules! trace_debug {
    ($($arg:tt)*) => {{ let _ = ::core::format_args!($($arg)*); }};
}

#[cfg(feature = "tracing")]
macro_rules! trace_info {
    ($($arg:tt)*) => { ::tracing::info!($($arg)*) };
}
#[cfg(not(feature = "tracing"))]
macro_rules! trace_info {
    ($($arg:tt)*) => {{ let _ = ::core::format_args!($($arg)*); }};
}

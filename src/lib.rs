//! whisper-rs — a feature-rich, safe Rust wrapper over whisper.cpp (local use).

// Internal tracing facade macros (`trace_debug!`/`trace_info!`). Declared first so the macros are in
// textual scope for every module below; no-op unless the `tracing` feature is enabled.
#[macro_use]
mod trace;

pub mod asr;
pub mod audio;
#[cfg(feature = "diarization")]
pub mod diarize;
pub mod error;
#[cfg(feature = "download")]
pub mod models;
pub mod output;
pub mod pipeline;
pub mod postprocess;
pub mod prelude;
#[cfg(feature = "streaming")]
pub mod stream;
pub mod timestamps;

// The raw whisper.cpp FFI layer (the only `unsafe` module). Hidden from docs by default; enable the
// `raw-api` feature to document it for power users who need the unwrapped bindings.
#[cfg_attr(not(feature = "raw-api"), doc(hidden))]
pub mod ffi;

pub use error::{Result, WhisperError};

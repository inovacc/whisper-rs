//! whisper-rs — a feature-rich, safe Rust wrapper over whisper.cpp (local use).

pub mod asr;
pub mod audio;
#[cfg(feature = "diarization")]
pub mod diarize;
pub mod error;
pub mod output;
pub mod pipeline;
pub mod postprocess;
pub mod prelude;
pub mod timestamps;

#[doc(hidden)]
pub mod ffi;

pub use error::{Result, WhisperError};

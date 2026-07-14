//! whisper-rs — a feature-rich, safe Rust wrapper over whisper.cpp (local use).

pub mod asr;
pub mod audio;
pub mod error;
pub mod output;

#[doc(hidden)]
pub mod ffi;

pub use error::{Result, WhisperError};

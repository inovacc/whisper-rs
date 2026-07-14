//! Crate-wide error type.
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind { Whisper, DiarizeSegmentation, DiarizeEmbedding }

impl std::fmt::Display for ModelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ModelKind::Whisper => "whisper",
            ModelKind::DiarizeSegmentation => "diarize-segmentation",
            ModelKind::DiarizeEmbedding => "diarize-embedding",
        };
        f.write_str(s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WhisperError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("audio decode error: {0}")]
    AudioDecode(String),
    #[error("resample error: {0}")]
    Resample(String),
    #[error("{kind} model not found at {path}")]
    ModelNotFound { kind: ModelKind, path: PathBuf },
    #[error("whisper.cpp returned non-zero code {0}")]
    Ffi(i32),
    #[error("invalid configuration: {0}")]
    Config(String),
    #[error("model download failed: {0}")]
    ModelDownload(String),
}

pub type Result<T> = std::result::Result<T, WhisperError>;

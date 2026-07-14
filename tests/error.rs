use std::path::PathBuf;
use whisper_rs::{error::ModelKind, WhisperError};

#[test]
fn model_not_found_displays_path_and_kind() {
    let e = WhisperError::ModelNotFound { kind: ModelKind::Whisper, path: PathBuf::from("/x/ggml.bin") };
    let s = e.to_string();
    assert!(s.contains("/x/ggml.bin"));
    assert!(s.contains("whisper"));
}

#[test]
fn io_error_converts_with_question_mark() {
    fn inner() -> whisper_rs::Result<()> {
        std::fs::File::open("/definitely/missing/file")?;
        Ok(())
    }
    assert!(matches!(inner(), Err(WhisperError::Io(_))));
}

//! Crate-wide error type. Full definition in Task 3.
#[derive(Debug)]
pub struct WhisperError;
pub type Result<T> = std::result::Result<T, WhisperError>;
impl std::fmt::Display for WhisperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "WhisperError") }
}
impl std::error::Error for WhisperError {}

//! Pure streaming-policy core: decides when tentative ASR hypotheses become committed text.
//!
//! This module contains no model, audio-capture, or threading code — only the data types and
//! policies that turn a stream of tentative token hypotheses into committed output. It is
//! model-independent and safe to unit test in isolation.

/// A single recognized token with its text and timing.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

/// The result of observing a new hypothesis: newly-committed text and the updated cursor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Committed {
    pub text: String,
    pub committed_upto: usize,
}

/// Events emitted by a streaming session as hypotheses are observed and committed.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    PartialText(String),
    CommittedSegment { text: String, start: f32, end: f32 },
    Error(String),
}

/// Decides when tentative hypotheses become committed text.
pub trait StreamPolicy {
    /// Observe a full-hypothesis token list; return the newly-committable prefix (beyond what was
    /// already committed).
    fn observe(&mut self, hypothesis: &[Token]) -> Committed;
}

pub mod local_agreement;
pub mod two_pass;
pub use local_agreement::LocalAgreement2;
pub use two_pass::TwoPass;

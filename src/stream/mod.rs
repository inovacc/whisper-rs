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

/// The result of observing a new hypothesis: newly-committed text and cursor bounds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Committed {
    pub text: String,
    /// Exclusive upper token index of the committed region (new cursor).
    pub committed_upto: usize,
    /// Inclusive lower token index the commit started at (cursor before advancing).
    pub committed_from: usize,
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
    /// Final signal (end of stream). Commit everything not yet committed.
    fn observe_final(&mut self, hypothesis: &[Token]) -> Committed {
        self.observe(hypothesis)
    }
}

/// Abstraction over a transcriber so streaming sessions can be unit-tested with a fake
/// (the concrete [`crate::asr::Transcriber`] needs a real model).
pub trait Transcribe {
    fn transcribe(
        &mut self,
        pcm: &[f32],
        opts: &crate::asr::AsrOptions,
    ) -> crate::error::Result<Vec<crate::output::Segment>>;
}

pub mod local_agreement;
pub mod session;
pub mod two_pass;
pub use local_agreement::LocalAgreement2;
pub use session::StreamSession;
pub use two_pass::TwoPass;

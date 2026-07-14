//! Synchronous streaming session over a [`Transcriber`]: push audio, poll for events.
use super::{StreamEvent, StreamPolicy, Token, Transcribe};
use crate::asr::{AsrOptions, Transcriber};

/// Synchronous streaming session: push 16 kHz mono f32 frames, poll for events.
/// Re-decodes the whole accumulated buffer each poll (simple LocalAgreement-friendly approach);
/// VAD-boundary incremental decoding is a later refinement.
///
/// Generic over the [`Transcribe`] seam (defaulting to the concrete [`Transcriber`]) so the
/// session logic can be unit-tested offline with a fake transcriber.
pub struct StreamSession<T: Transcribe = Transcriber> {
    transcriber: T,
    policy: Box<dyn StreamPolicy + Send>,
    opts: AsrOptions,
    buffer: Vec<f32>,
    dirty: bool,
}

impl<T: Transcribe> StreamSession<T> {
    pub fn new(transcriber: T, policy: Box<dyn StreamPolicy + Send>, opts: AsrOptions) -> Self {
        Self { transcriber, policy, opts, buffer: Vec::new(), dirty: false }
    }

    /// Append audio frames (16 kHz mono f32).
    pub fn push(&mut self, frames: &[f32]) {
        self.buffer.extend_from_slice(frames);
        self.dirty = true;
    }

    /// Re-decode the buffer, advance the policy, and return events (Committed + a Partial tail).
    pub fn poll(&mut self) -> Vec<StreamEvent> {
        if self.buffer.is_empty() || !self.dirty {
            return vec![];
        }
        self.dirty = false;
        self.decode_and_advance(false)
    }

    /// Final decode pass; commit everything remaining.
    pub fn finalize(&mut self) -> Vec<StreamEvent> {
        if self.buffer.is_empty() {
            return vec![];
        }
        self.decode_and_advance(true)
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.dirty = false;
    }

    fn decode_and_advance(&mut self, final_pass: bool) -> Vec<StreamEvent> {
        let segments = match self.transcriber.transcribe(&self.buffer, &self.opts) {
            Ok(s) => s,
            Err(e) => return vec![StreamEvent::Error(e.to_string())],
        };
        // Flatten words across segments into a Token hypothesis; fall back to segment text if no words.
        let tokens: Vec<Token> = segments
            .iter()
            .flat_map(|seg| {
                if seg.words.is_empty() {
                    vec![Token { text: seg.text.trim().to_string(), start: seg.start, end: seg.end }]
                } else {
                    seg.words.iter().map(|w| Token { text: w.text.clone(), start: w.start, end: w.end }).collect()
                }
            })
            .filter(|t| !t.text.is_empty())
            .collect();

        let committed = if final_pass { self.policy.observe_final(&tokens) } else { self.policy.observe(&tokens) };
        let mut events = Vec::new();
        if !committed.text.trim().is_empty() {
            // Time the event from the committed token slice, clamping to the hypothesis length.
            let (start, end) =
                if committed.committed_upto > committed.committed_from && committed.committed_upto <= tokens.len() {
                    (tokens[committed.committed_from].start, tokens[committed.committed_upto - 1].end)
                } else {
                    (0.0, 0.0)
                };
            events.push(StreamEvent::CommittedSegment { text: committed.text.trim().to_string(), start, end });
        }
        // Partial = the full current hypothesis text (tentative view).
        let partial: String = super::join_tokens(&tokens);
        if !partial.is_empty() {
            events.push(StreamEvent::PartialText(partial));
        }
        events
    }
}

//! Structured, analytics-ready transcript types.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeakerId(pub u32);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SegmentFlags {
    /// Set by the (later) post-processing stage when a segment looks hallucinated.
    pub hallucination_suspect: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub speaker: Option<SpeakerId>,
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub words: Vec<Word>,
    pub flags: SegmentFlags,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcript {
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// Concatenate segment text with single spaces, trimmed.
    pub fn plain_text(&self) -> String {
        self.segments.iter().map(|s| s.text.trim()).collect::<Vec<_>>().join(" ")
    }
}

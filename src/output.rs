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

    /// Render as SubRip (`.srt`) subtitles: numbered cues, `HH:MM:SS,mmm` timestamps.
    pub fn to_srt(&self) -> String {
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            out.push_str(&(i + 1).to_string());
            out.push('\n');
            out.push_str(&format_timestamp(seg.start, ','));
            out.push_str(" --> ");
            out.push_str(&format_timestamp(seg.end, ','));
            out.push('\n');
            out.push_str(seg.text.trim());
            out.push_str("\n\n");
        }
        out
    }

    /// Render as WebVTT (`.vtt`) subtitles: `WEBVTT` header, `HH:MM:SS.mmm` timestamps.
    pub fn to_vtt(&self) -> String {
        let mut out = String::from("WEBVTT\n\n");
        for seg in &self.segments {
            out.push_str(&format_timestamp(seg.start, '.'));
            out.push_str(" --> ");
            out.push_str(&format_timestamp(seg.end, '.'));
            out.push('\n');
            out.push_str(seg.text.trim());
            out.push_str("\n\n");
        }
        out
    }
}

/// Format a time in seconds as `HH:MM:SS<sep>mmm` (`sep` is `,` for SRT, `.` for VTT).
/// Negative inputs clamp to zero.
fn format_timestamp(seconds: f32, sep: char) -> String {
    let total_ms = (seconds.max(0.0) as f64 * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let s = (total_ms / 1000) % 60;
    let m = (total_ms / 60_000) % 60;
    let h = total_ms / 3_600_000;
    format!("{h:02}:{m:02}:{s:02}{sep}{ms:03}")
}

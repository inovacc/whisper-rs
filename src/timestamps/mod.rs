//! Word-level timestamps from whisper token data, with monotonicity enforcement. Safe layer.
use crate::ffi;
use crate::output::Word;

/// Clamp a word list to be ordered and non-overlapping in sequence.
pub fn enforce_monotonic(mut words: Vec<Word>) -> Vec<Word> {
    let mut cursor = 0.0f32;
    for w in words.iter_mut() {
        if w.start < cursor {
            w.start = cursor;
        }
        if w.end < w.start {
            w.end = w.start;
        }
        cursor = w.end;
    }
    words
}

/// Build words from raw whisper token tuples (text, t0_cs, t1_cs, prob), filtering special tokens.
pub fn words_from_tokens(tokens: Vec<(String, i64, i64, f32)>) -> Vec<Word> {
    let mut words = Vec::new();
    for (text, t0_cs, t1_cs, p) in tokens {
        let trimmed = text.trim();
        // Skip whisper special tokens ("[_BEG_]", "<|...|>", etc.) and empties.
        if trimmed.is_empty() || trimmed.starts_with('[') || trimmed.starts_with("<|") {
            continue;
        }
        words.push(Word { text: trimmed.to_string(), start: t0_cs as f32 / 100.0, end: t1_cs as f32 / 100.0, confidence: p });
    }
    enforce_monotonic(words)
}

/// Build words for one segment from the context's raw token data (special tokens filtered).
pub(crate) fn words_for_segment(ctx: &ffi::Context, seg: i32) -> Vec<Word> {
    words_from_tokens(ctx.segment_tokens(seg))
}

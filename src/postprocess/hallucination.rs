//! Hallucination flagging via cross-pass disagreement (pure heuristic).
use crate::output::Segment;

/// Word-set Jaccard similarity of two texts (lowercased, whitespace-split). 1.0 = identical sets.
pub fn text_similarity(a: &str, b: &str) -> f32 {
    use std::collections::HashSet;
    let wa: HashSet<String> = a.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();
    let wb: HashSet<String> = b.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();
    if wa.is_empty() && wb.is_empty() {
        return 1.0;
    }
    let inter = wa.intersection(&wb).count() as f32;
    let union = wa.union(&wb).count() as f32;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

/// For each `primary` segment, flag it as a hallucination suspect when its best text-similarity to any
/// TIME-OVERLAPPING `secondary` segment is below `threshold`. Returns one bool per primary segment.
///
/// A primary segment with **no time-overlapping** secondary segment scores 0 and is therefore
/// flagged — absence of a cross-pass counterpart is treated as a hallucination signal.
pub fn flag_hallucinations(primary: &[Segment], secondary: &[Segment], threshold: f32) -> Vec<bool> {
    primary
        .iter()
        .map(|p| {
            let best = secondary
                .iter()
                .filter(|s| s.end > p.start && s.start < p.end) // temporal overlap
                .map(|s| text_similarity(&p.text, &s.text))
                .fold(0.0f32, f32::max);
            best < threshold
        })
        .collect()
}

/// Apply flags in place onto the primary segments' `SegmentFlags.hallucination_suspect`.
pub fn apply_flags(primary: &mut [Segment], flags: &[bool]) {
    for (seg, &f) in primary.iter_mut().zip(flags.iter()) {
        seg.flags.hallucination_suspect = f;
    }
}

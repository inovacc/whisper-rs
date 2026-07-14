//! Pure timeline-merge: attach speaker turns to transcript segments by
//! maximum temporal overlap.

use super::SpeakerTurn;
use crate::output::Segment;

/// Assign each segment the speaker of the turn it overlaps most with.
///
/// Ties are broken in favor of the earliest turn (by position in `turns`).
/// Segments with no overlapping turn are left with `speaker: None`.
pub fn assign_speakers(mut segments: Vec<Segment>, turns: &[SpeakerTurn]) -> Vec<Segment> {
    for segment in &mut segments {
        let mut best: Option<(f32, usize)> = None;
        for (idx, turn) in turns.iter().enumerate() {
            let overlap = (segment.end.min(turn.end) - segment.start.max(turn.start)).max(0.0);
            if overlap <= 0.0 {
                continue;
            }
            match best {
                Some((best_overlap, _)) if overlap <= best_overlap => {}
                _ => best = Some((overlap, idx)),
            }
        }
        segment.speaker = best.map(|(_, idx)| turns[idx].speaker);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{SegmentFlags, SpeakerId, Word};

    fn seg(text: &str, s: f32, e: f32) -> Segment {
        Segment {
            speaker: None,
            text: text.into(),
            start: s,
            end: e,
            words: vec![Word { text: text.into(), start: s, end: e, confidence: 1.0 }],
            flags: SegmentFlags::default(),
        }
    }

    #[test]
    fn assigns_speaker_by_max_overlap() {
        let turns = vec![
            SpeakerTurn { speaker: SpeakerId(0), start: 0.0, end: 2.0 },
            SpeakerTurn { speaker: SpeakerId(1), start: 2.0, end: 4.0 },
        ];
        let out = assign_speakers(vec![seg("a", 0.1, 1.8), seg("b", 2.2, 3.9)], &turns);
        assert_eq!(out[0].speaker, Some(SpeakerId(0)));
        assert_eq!(out[1].speaker, Some(SpeakerId(1)));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].text, "a");
    }

    #[test]
    fn no_overlap_leaves_speaker_none() {
        let turns = vec![SpeakerTurn { speaker: SpeakerId(0), start: 10.0, end: 11.0 }];
        let out = assign_speakers(vec![seg("x", 0.0, 1.0)], &turns);
        assert_eq!(out[0].speaker, None);
    }

    #[test]
    fn tie_prefers_earliest_turn() {
        let turns = vec![
            SpeakerTurn { speaker: SpeakerId(0), start: 0.0, end: 1.0 },
            SpeakerTurn { speaker: SpeakerId(1), start: 1.0, end: 2.0 },
        ];
        // Segment spans both turns equally (0.5 overlap each).
        let out = assign_speakers(vec![seg("x", 0.5, 1.5)], &turns);
        assert_eq!(out[0].speaker, Some(SpeakerId(0)));
    }
}

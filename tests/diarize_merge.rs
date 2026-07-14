#![cfg(feature = "diarization")]
use whisper_rs::diarize::{merge::assign_speakers, SpeakerTurn};
use whisper_rs::output::{Segment, SegmentFlags, SpeakerId, Word};

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

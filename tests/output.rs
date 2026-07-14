use whisper_rs::output::{Segment, SegmentFlags, Transcript, Word};

#[test]
fn plain_text_joins_segment_text_in_order() {
    let t = Transcript {
        segments: vec![
            Segment {
                speaker: None,
                text: "hello".into(),
                start: 0.0,
                end: 1.0,
                words: vec![],
                flags: SegmentFlags::default(),
            },
            Segment {
                speaker: None,
                text: "world".into(),
                start: 1.0,
                end: 2.0,
                words: vec![],
                flags: SegmentFlags::default(),
            },
        ],
    };
    assert_eq!(t.plain_text(), "hello world");
}

#[test]
fn word_fields_roundtrip() {
    let w = Word { text: "hi".into(), start: 0.1, end: 0.3, confidence: 0.9 };
    assert_eq!(w.text, "hi");
    assert!((w.end - w.start - 0.2).abs() < 1e-6);
}

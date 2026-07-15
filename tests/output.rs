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

fn seg(text: &str, start: f32, end: f32) -> Segment {
    Segment { speaker: None, text: text.into(), start, end, words: vec![], flags: SegmentFlags::default() }
}

#[test]
fn to_srt_numbers_cues_and_formats_timestamps_with_comma() {
    let t = Transcript { segments: vec![seg("  hello  ", 1.5, 65.25), seg("world", 65.25, 66.0)] };
    let expected = "1\n00:00:01,500 --> 00:01:05,250\nhello\n\n2\n00:01:05,250 --> 00:01:06,000\nworld\n\n";
    assert_eq!(t.to_srt(), expected);
}

#[test]
fn to_vtt_has_header_and_dot_separator() {
    let t = Transcript { segments: vec![seg("hello", 1.5, 65.25)] };
    let expected = "WEBVTT\n\n00:00:01.500 --> 00:01:05.250\nhello\n\n";
    assert_eq!(t.to_vtt(), expected);
}

#[test]
fn subtitle_timestamp_handles_hours() {
    // 3723.0s = 01:02:03.000
    let t = Transcript { segments: vec![seg("x", 3723.0, 3723.0)] };
    assert!(t.to_vtt().contains("01:02:03.000 --> 01:02:03.000"));
    assert!(t.to_srt().contains("01:02:03,000 --> 01:02:03,000"));
}

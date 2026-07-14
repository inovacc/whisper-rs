use whisper_rs::postprocess::hallucination::{apply_flags, flag_hallucinations, text_similarity};
use whisper_rs::output::{Segment, SegmentFlags};

fn seg(text: &str, s: f32, e: f32) -> Segment {
    Segment {
        speaker: None,
        text: text.into(),
        start: s,
        end: e,
        words: vec![],
        flags: SegmentFlags::default(),
    }
}

#[test]
fn similarity_basic() {
    assert!((text_similarity("the cat sat", "the cat sat") - 1.0).abs() < 1e-6);
    assert_eq!(text_similarity("apple", "banana"), 0.0);
    assert!(text_similarity("the cat", "the dog") > 0.0 && text_similarity("the cat", "the dog") < 1.0);
}

#[test]
fn agreeing_passes_flag_nothing() {
    let a = vec![seg("hello world", 0.0, 1.0), seg("how are you", 1.0, 2.0)];
    let b = vec![seg("hello world", 0.0, 1.0), seg("how are you", 1.0, 2.0)];
    assert_eq!(flag_hallucinations(&a, &b, 0.5), vec![false, false]);
}

#[test]
fn divergent_segment_is_flagged() {
    let a = vec![seg("thank you for watching", 0.0, 2.0)]; // classic whisper hallucination
    let b = vec![seg("uh", 0.0, 2.0)]; // second pass disagrees
    assert_eq!(flag_hallucinations(&a, &b, 0.5), vec![true]);
}

#[test]
fn no_overlapping_secondary_flags_suspect() {
    let a = vec![seg("hello", 0.0, 1.0)];
    let b = vec![seg("hello", 5.0, 6.0)]; // no temporal overlap -> best=0 -> flagged
    assert_eq!(flag_hallucinations(&a, &b, 0.5), vec![true]);
}

#[test]
fn apply_flags_sets_segment_flag() {
    let mut a = vec![seg("x", 0.0, 1.0)];
    apply_flags(&mut a, &[true]);
    assert!(a[0].flags.hallucination_suspect);
}

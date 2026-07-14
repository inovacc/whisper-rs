use whisper_rs::output::Word;
use whisper_rs::timestamps::enforce_monotonic;

#[test]
fn enforce_monotonic_fixes_overlap_and_order() {
    let words = vec![
        Word { text: "a".into(), start: 0.0, end: 0.5, confidence: 1.0 },
        Word { text: "b".into(), start: 0.4, end: 0.9, confidence: 1.0 },
        Word { text: "c".into(), start: 0.8, end: 0.7, confidence: 1.0 },
    ];
    let fixed = enforce_monotonic(words);
    for pair in fixed.windows(2) {
        assert!(pair[0].end <= pair[1].start + 1e-6, "overlap remains: {pair:?}");
    }
    assert!(fixed.iter().all(|w| w.end >= w.start));
}

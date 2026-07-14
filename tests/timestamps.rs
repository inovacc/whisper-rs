use whisper_rs::output::Word;
use whisper_rs::timestamps::{enforce_monotonic, words_from_tokens};

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

#[test]
fn words_from_tokens_filters_special_and_orders() {
    let toks = vec![
        ("[_BEG_]".into(), 0, 0, 1.0),
        (" Hello".into(), 0, 50, 0.9),
        ("<|endoftext|>".into(), 50, 50, 1.0),
        (" world".into(), 40, 90, 0.8), // overlaps -> monotonic fix
        ("   ".into(), 90, 90, 1.0),    // whitespace-only, dropped
    ];
    let words = words_from_tokens(toks);
    assert_eq!(words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), vec!["Hello", "world"]);
    assert!(words[0].end <= words[1].start + 1e-6);
}

//! Property tests for word-timestamp monotonicity enforcement (`timestamps::enforce_monotonic`).
//!
//! `enforce_monotonic` must, for any input, produce a word list where each word's `end >= start`
//! and each word's `start >= the previous word's end` (ordered, non-overlapping in sequence), and it
//! must be idempotent. These hold regardless of how garbled the raw whisper token times are.
use proptest::prelude::*;
use whisper_rs::output::Word;
use whisper_rs::timestamps::enforce_monotonic;

/// Arbitrary word lists with finite (non-NaN, bounded) start/end times so comparisons are total.
fn words_strategy() -> impl Strategy<Value = Vec<Word>> {
    proptest::collection::vec((-1000.0f32..1000.0, -1000.0f32..1000.0), 0..64).prop_map(|pairs| {
        pairs
            .into_iter()
            .map(|(start, end)| Word { text: "w".into(), start, end, confidence: 0.5 })
            .collect()
    })
}

proptest! {
    #[test]
    fn output_is_ordered_and_non_overlapping(words in words_strategy()) {
        let out = enforce_monotonic(words);
        for w in &out {
            prop_assert!(w.end >= w.start, "word end {} < start {}", w.end, w.start);
        }
        for pair in out.windows(2) {
            prop_assert!(pair[1].start >= pair[0].end, "start {} < previous end {}", pair[1].start, pair[0].end);
        }
    }

    #[test]
    fn enforcement_is_idempotent(words in words_strategy()) {
        let once = enforce_monotonic(words);
        let twice = enforce_monotonic(once.clone());
        prop_assert_eq!(once, twice);
    }
}

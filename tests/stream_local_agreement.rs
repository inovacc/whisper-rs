#![cfg(feature = "streaming")]
use whisper_rs::stream::{LocalAgreement2, StreamPolicy, Token};

fn toks(words: &[&str]) -> Vec<Token> {
    words.iter().enumerate().map(|(i, w)| Token { text: w.to_string(), start: i as f32, end: i as f32 + 1.0 }).collect()
}

#[test]
fn commits_prefix_stable_across_two_hypotheses() {
    let mut p = LocalAgreement2::new();
    assert_eq!(p.observe(&toks(&["the", "quick"])).text.trim(), "");
    assert_eq!(p.observe(&toks(&["the", "quick", "brown"])).text.trim(), "the quick");
    assert_eq!(p.observe(&toks(&["the", "quick", "brown", "fox"])).text.trim(), "brown");
}

#[test]
fn revision_does_not_recommit() {
    let mut p = LocalAgreement2::new();
    p.observe(&toks(&["hello", "wrld"]));
    p.observe(&toks(&["hello", "world"])); // "hello" stable -> committed
    let c = p.observe(&toks(&["hello", "world", "now"]));
    assert!(!c.text.contains("hello"));
}

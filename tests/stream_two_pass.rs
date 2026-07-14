#![cfg(feature = "streaming")]
use whisper_rs::stream::{StreamPolicy, Token, TwoPass};

fn toks(words: &[&str]) -> Vec<Token> {
    words
        .iter()
        .enumerate()
        .map(|(i, w)| Token {
            text: w.to_string(),
            start: i as f32,
            end: i as f32 + 1.0,
        })
        .collect()
}

#[test]
fn tentative_commits_nothing_final_commits_all() {
    let mut p = TwoPass::new();
    assert_eq!(p.observe(&toks(&["hello", "wor"])).text.trim(), ""); // tentative -> nothing
    let c = p.observe_final(&toks(&["hello", "world"]));
    assert_eq!(c.text.trim(), "hello world");
    // a second final after committing commits only new tokens
    let c2 = p.observe_final(&toks(&["hello", "world", "again"]));
    assert_eq!(c2.text.trim(), "again");
}

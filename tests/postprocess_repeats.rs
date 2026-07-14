use whisper_rs::postprocess::collapse_repeats;

#[test]
fn collapses_three_or_more() {
    assert_eq!(collapse_repeats("the the the cat"), "the cat");
    assert_eq!(collapse_repeats("no no thanks"), "no no thanks"); // only 2 -> unchanged
    assert_eq!(collapse_repeats("go Go GO now"), "go now"); // case-insensitive
}

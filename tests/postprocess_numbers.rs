use whisper_rs::postprocess::normalize_numbers;

#[test]
fn cardinals_to_digits() {
    assert_eq!(normalize_numbers("i have three cats"), "i have 3 cats");
    assert_eq!(normalize_numbers("twenty five"), "25");
    assert_eq!(normalize_numbers("one hundred and five"), "105");
    assert_eq!(normalize_numbers("two thousand"), "2000");
    assert_eq!(normalize_numbers("no numbers here"), "no numbers here");
}

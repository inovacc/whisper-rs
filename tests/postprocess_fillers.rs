use whisper_rs::postprocess::remove_fillers;

#[test]
fn removes_english_fillers() {
    assert_eq!(remove_fillers("um so uh yeah", "en"), "so yeah");
    assert_eq!(remove_fillers("i like cats", "en"), "i like cats"); // "like" is NOT a filler here
    assert_eq!(remove_fillers("hola eh que tal", "es"), "hola que tal");
    assert_eq!(remove_fillers("no fillers", "xx"), "no fillers"); // unknown lang unchanged
}

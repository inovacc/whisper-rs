use whisper_rs::pipeline::{ModelRef, Pipeline};

#[test]
fn build_requires_a_whisper_model() {
    let err = Pipeline::builder().build().unwrap_err();
    assert!(matches!(err, whisper_rs::WhisperError::Config(_)));
}

#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn transcribe_file_returns_timestamped_transcript() {
    let mut p = Pipeline::builder()
        .whisper_model(ModelRef::path("models/ggml-tiny.en.bin"))
        .language(Some("en".into()))
        .build().unwrap();
    let t = p.transcribe_file("tests/fixtures/jfk.wav").unwrap();
    assert!(!t.segments.is_empty());
    assert!(t.plain_text().to_lowercase().contains("country"));
    assert!(t.segments.iter().flat_map(|s| &s.words).count() > 0);
}

use whisper_rs::audio::preprocess::PreprocessLevel;
use whisper_rs::pipeline::{ModelRef, Pipeline};

#[test]
fn builder_accepts_preprocess_level() {
    // model missing -> build() errors, but .preprocess() must exist and chain, and default stays L0.
    let err = Pipeline::builder()
        .preprocess(PreprocessLevel::L2)
        .whisper_model(ModelRef::path("nope.bin"))
        .build();
    assert!(err.is_err());
}

#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn transcribe_with_preprocess_still_works() {
    let mut p = Pipeline::builder()
        .whisper_model(ModelRef::path("models/ggml-tiny.en.bin"))
        .language(Some("en".into()))
        .preprocess(PreprocessLevel::L2)
        .build().unwrap();
    let t = p.transcribe_file("tests/fixtures/jfk.wav").unwrap();
    assert!(t.plain_text().to_lowercase().contains("country"));
}

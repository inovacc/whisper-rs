use whisper_rs::asr::{AsrOptions, Transcriber};

#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn transcribes_known_clip() {
    let mut t = Transcriber::from_model_file("models/ggml-tiny.en.bin").unwrap();
    let a = whisper_rs::audio::AudioInput::from_wav_file("tests/fixtures/jfk.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    let segs = t.transcribe(&pcm, &AsrOptions::default()).unwrap();
    let text = segs.iter().map(|s| s.text.as_str()).collect::<String>().to_lowercase();
    assert!(text.contains("country"), "expected JFK clip text, got: {text:?}");
    assert!(segs.iter().all(|s| s.end >= s.start));
}

#[test]
fn missing_model_is_typed_error() {
    let err = Transcriber::from_model_file("models/does-not-exist.bin").unwrap_err();
    assert!(matches!(err, whisper_rs::WhisperError::ModelNotFound { .. }));
}

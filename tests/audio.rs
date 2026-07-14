use whisper_rs::audio::AudioInput;

#[test]
fn decodes_and_resamples_to_16k_mono() {
    let a = AudioInput::from_wav_file("tests/fixtures/sine_8k_stereo.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    assert!((pcm.len() as i32 - 16000).abs() < 400, "got {} samples", pcm.len());
    assert!(pcm.iter().all(|s| s.abs() <= 1.001));
}

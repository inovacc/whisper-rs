use whisper_rs::audio::AudioInput;

#[test]
fn decodes_and_resamples_to_16k_mono() {
    let a = AudioInput::from_wav_file("tests/fixtures/sine_8k_stereo.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    assert!((pcm.len() as i32 - 16000).abs() < 400, "got {} samples", pcm.len());
    assert!(pcm.iter().all(|s| s.abs() <= 1.001));
}

#[test]
fn empty_input_returns_empty() {
    // Build an AudioInput with zero samples at a non-16k rate via a tiny silent WAV.
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let path = std::env::temp_dir().join("whisper_rs_empty.wav");
    {
        let w = hound::WavWriter::create(&path, spec).unwrap();
        w.finalize().unwrap();
    }
    let a = whisper_rs::audio::AudioInput::from_wav_file(&path).unwrap();
    assert_eq!(a.to_mono_16k().unwrap(), Vec::<f32>::new());
}

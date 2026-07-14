use whisper_rs::audio::AudioInput;
use whisper_rs::error::WhisperError;

#[test]
fn decodes_and_resamples_to_16k_mono() {
    let a = AudioInput::from_wav_file("tests/fixtures/sine_8k_stereo.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    // Tightened from the pre-fix ±400 now that `resample` drains the rubato delay line
    // instead of silently dropping the resampler's trailing ~8 ms of output.
    assert!((pcm.len() as i32 - 16000).abs() < 80, "got {} samples", pcm.len());
    assert!(pcm.iter().all(|s| s.abs() <= 1.001));
}

#[test]
fn resample_preserves_temporal_alignment() {
    // Silence, then a step to a constant level partway through a 1 s, 8 kHz clip. If the
    // resampler's delay line isn't drained, the whole signal shifts earlier (and the tail is
    // clipped) — this test asserts the step lands at approximately the same relative time
    // after resampling to 16 kHz, i.e. at ~2x its original sample index.
    let spec = hound::WavSpec { channels: 1, sample_rate: 8000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let path = std::env::temp_dir().join("whisper_rs_step.wav");
    let step_at = 4000usize; // 0.5 s into the clip
    {
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for i in 0..8000usize {
            let v: i16 = if i < step_at { 0 } else { i16::MAX / 2 };
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
    }
    let a = whisper_rs::audio::AudioInput::from_wav_file(&path).unwrap();
    let pcm = a.to_mono_16k().unwrap();

    // Find where the resampled signal first sustains a level near the step's amplitude
    // (avoids tripping on sinc-interpolation ringing right at the edge).
    let threshold = 0.4; // step amplitude is ~0.5 of full scale
    let window = 8;
    let rise_idx = (0..pcm.len().saturating_sub(window))
        .find(|&i| pcm[i..i + window].iter().all(|s| s.abs() > threshold))
        .expect("expected a sustained rise in the resampled signal");

    let expected_idx = step_at * 2; // 16 kHz is 2x the 8 kHz input rate
    let diff = (rise_idx as i64 - expected_idx as i64).abs();
    // Within ~5 ms at 16 kHz (80 samples).
    assert!(diff < 80, "rise at {rise_idx}, expected near {expected_idx} (diff {diff})");
}

#[test]
fn empty_input_returns_empty() {
    // Build an AudioInput with zero samples at a non-16k rate via a tiny silent WAV.
    let spec = hound::WavSpec { channels: 1, sample_rate: 8000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let path = std::env::temp_dir().join("whisper_rs_empty.wav");
    {
        let w = hound::WavWriter::create(&path, spec).unwrap();
        w.finalize().unwrap();
    }
    let a = whisper_rs::audio::AudioInput::from_wav_file(&path).unwrap();
    assert_eq!(a.to_mono_16k().unwrap(), Vec::<f32>::new());
}

#[test]
fn decodes_float_wav() {
    let a = AudioInput::from_wav_file("tests/fixtures/sine_f32_16k.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    assert!(!pcm.is_empty());
    assert!(pcm.iter().all(|s| *s >= -1.0 && *s <= 1.0));
}

#[test]
fn corrupt_wav_is_audio_decode_error() {
    let path = std::env::temp_dir().join("whisper_rs_corrupt.wav");
    std::fs::write(&path, b"not a real wav file").unwrap();
    match AudioInput::from_wav_file(&path) {
        Err(WhisperError::AudioDecode(_)) => {}
        Err(other) => panic!("expected AudioDecode error, got {other:?}"),
        Ok(_) => panic!("expected AudioDecode error, got Ok"),
    }
}

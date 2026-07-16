#![cfg(feature = "ffmpeg")]
//! Exercises the ffmpeg libavfilter decode path end-to-end (feature = "ffmpeg").
//!
//! ffmpeg decodes WAV too, so this runs the full `abuffer -> aresample -> abuffersink` graph
//! against the in-repo `jfk.wav` fixture without needing a separate compressed sample. It is the
//! only non-ignored test that executes `decode_to_mono_16k`; it runs in the `ffmpeg` CI job.
use whisper_rs::audio::media::decode_to_mono_16k;

#[test]
fn decodes_wav_fixture_to_mono_16k() {
    let pcm = decode_to_mono_16k("tests/fixtures/jfk.wav").expect("decode jfk.wav via ffmpeg");
    // jfk.wav is ~11 s of speech; at 16 kHz mono that is ~176k samples.
    assert!(pcm.len() > 16_000, "expected >1 s of 16 kHz mono samples, got {}", pcm.len());
    assert!(pcm.iter().all(|s| s.is_finite()), "decoded samples must be finite");
    assert!(pcm.iter().any(|&s| s.abs() > 0.01), "expected non-silent audio");
}

#![cfg(feature = "streaming")]
use whisper_rs::asr::{AsrOptions, Transcriber};
use whisper_rs::stream::{LocalAgreement2, StreamEvent, StreamSession};

#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn streams_jfk_clip_in_chunks() {
    let t = Transcriber::from_model_file("models/ggml-tiny.en.bin").unwrap();
    let mut sess = StreamSession::new(
        t,
        Box::new(LocalAgreement2::new()),
        AsrOptions { language: Some("en".into()), ..Default::default() },
    );
    let pcm = whisper_rs::audio::AudioInput::from_wav_file("tests/fixtures/jfk.wav")
        .unwrap()
        .to_mono_16k()
        .unwrap();
    // feed in ~0.5s chunks, polling as we go
    let chunk = 8000;
    let mut committed = String::new();
    let mut i = 0;
    while i < pcm.len() {
        let end = (i + chunk).min(pcm.len());
        sess.push(&pcm[i..end]);
        for ev in sess.poll() {
            if let StreamEvent::CommittedSegment { text, .. } = ev {
                committed.push_str(&text);
                committed.push(' ');
            }
        }
        i = end;
    }
    for ev in sess.finalize() {
        if let StreamEvent::CommittedSegment { text, .. } = ev {
            committed.push_str(&text);
            committed.push(' ');
        }
    }
    assert!(committed.to_lowercase().contains("country"), "streamed text: {committed:?}");
}

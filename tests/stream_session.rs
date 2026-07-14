#![cfg(feature = "streaming")]
use whisper_rs::asr::{AsrOptions, Transcriber};
use whisper_rs::stream::{LocalAgreement2, StreamEvent, StreamSession, TwoPass};

// ---- Offline unit tests using a fake transcriber (no model needed) ----

/// A scripted [`Transcribe`] that returns a pre-baked segment list per call, ignoring the PCM.
struct FakeTranscriber {
    scripts: Vec<Vec<whisper_rs::output::Segment>>,
    i: usize,
}

impl whisper_rs::stream::Transcribe for FakeTranscriber {
    fn transcribe(
        &mut self,
        _pcm: &[f32],
        _opts: &whisper_rs::asr::AsrOptions,
    ) -> whisper_rs::Result<Vec<whisper_rs::output::Segment>> {
        let out = self.scripts.get(self.i).cloned().unwrap_or_default();
        self.i += 1;
        Ok(out)
    }
}

/// Build a single-segment hypothesis from `(word, start, end)` triples (one whisper Word each).
fn seg(words: &[(&str, f32, f32)]) -> Vec<whisper_rs::output::Segment> {
    use whisper_rs::output::{Segment, SegmentFlags, Word};
    let ws: Vec<Word> =
        words.iter().map(|(t, s, e)| Word { text: t.to_string(), start: *s, end: *e, confidence: 1.0 }).collect();
    let text = words.iter().map(|(t, _, _)| *t).collect::<Vec<_>>().join(" ");
    let start = words.first().map(|(_, s, _)| *s).unwrap_or(0.0);
    let end = words.last().map(|(_, _, e)| *e).unwrap_or(0.0);
    vec![Segment { speaker: None, text, start, end, words: ws, flags: SegmentFlags::default() }]
}

fn fake_session(scripts: Vec<Vec<whisper_rs::output::Segment>>, policy_two_pass: bool) -> StreamSession<FakeTranscriber> {
    let fake = FakeTranscriber { scripts, i: 0 };
    if policy_two_pass {
        StreamSession::new(fake, Box::new(TwoPass::new()), AsrOptions::default())
    } else {
        StreamSession::new(fake, Box::new(LocalAgreement2::new()), AsrOptions::default())
    }
}

fn committed(events: &[StreamEvent]) -> Vec<(String, f32, f32)> {
    events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::CommittedSegment { text, start, end } => Some((text.clone(), *start, *end)),
            _ => None,
        })
        .collect()
}

#[test]
fn reset_clears_buffer() {
    let mut sess = fake_session(vec![seg(&[("a", 0.0, 1.0)]), seg(&[("b", 0.0, 1.0)])], false);
    sess.push(&[0.1_f32; 16]);
    let _ = sess.poll(); // consumes script 0
    sess.reset();
    // Buffer is empty again -> poll is a no-op and emits nothing.
    assert_eq!(sess.poll(), Vec::<StreamEvent>::new());
}

#[test]
fn poll_empty_buffer_returns_nothing() {
    let mut sess = fake_session(vec![seg(&[("a", 0.0, 1.0)])], false);
    assert_eq!(sess.poll(), Vec::<StreamEvent>::new());
}

#[test]
fn local_agreement_commits_then_finalize_flushes_tail() {
    // Growing hypotheses: two agreeing polls commit "the quick"; the trailing word "brown"
    // appears only in the final hypothesis and must be flushed by finalize() (regression:
    // finalize used to call observe(), which drops the tail).
    let mut sess = fake_session(
        vec![
            seg(&[("the", 0.0, 0.5), ("quick", 0.5, 1.0)]),
            seg(&[("the", 0.0, 0.5), ("quick", 0.5, 1.0)]),
            seg(&[("the", 0.0, 0.5), ("quick", 0.5, 1.0), ("brown", 1.0, 1.5)]),
        ],
        false,
    );
    sess.push(&[0.1_f32; 16]);
    assert!(committed(&sess.poll()).is_empty(), "first poll: nothing agrees yet");
    sess.push(&[0.1_f32; 16]);
    let c2 = committed(&sess.poll());
    assert_eq!(c2.len(), 1);
    assert_eq!(c2[0].0, "the quick");
    // finalize flushes the trailing word.
    let cf = committed(&sess.finalize());
    assert_eq!(cf.len(), 1, "finalize must emit the trailing word");
    assert_eq!(cf[0].0, "brown");
}

#[test]
fn two_pass_poll_partial_only_finalize_commits_full() {
    // TwoPass commits nothing on poll (only PartialText); finalize() commits the whole text.
    // Regression for "TwoPass never commits through a StreamSession".
    let mut sess = fake_session(
        vec![seg(&[("hello", 0.0, 1.0), ("world", 1.0, 2.0)]), seg(&[("hello", 0.0, 1.0), ("world", 1.0, 2.0)])],
        true,
    );
    sess.push(&[0.1_f32; 16]);
    let ev = sess.poll();
    assert!(committed(&ev).is_empty(), "TwoPass poll must not commit");
    assert!(ev.iter().any(|e| matches!(e, StreamEvent::PartialText(_))), "TwoPass poll must emit a PartialText");
    let cf = committed(&sess.finalize());
    assert_eq!(cf.len(), 1);
    assert_eq!(cf[0].0, "hello world");
}

#[test]
fn committed_segment_timings_match_slice() {
    // The committed segment's (start, end) equal the first/last committed token's times.
    // Regression for Step 3 (previously used whole-buffer first/last token times).
    let mut sess = fake_session(
        vec![seg(&[("the", 0.0, 0.5), ("quick", 0.5, 1.2)]), seg(&[("the", 0.0, 0.5), ("quick", 0.5, 1.2)])],
        false,
    );
    sess.push(&[0.1_f32; 16]);
    let _ = sess.poll();
    sess.push(&[0.1_f32; 16]);
    let c = committed(&sess.poll());
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].0, "the quick");
    assert_eq!(c[0].1, 0.0, "start = first committed token start");
    assert_eq!(c[0].2, 1.2, "end = last committed token end");
}

#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn streams_jfk_clip_in_chunks() {
    let t = Transcriber::from_model_file("models/ggml-tiny.en.bin").unwrap();
    let mut sess = StreamSession::new(
        t,
        Box::new(LocalAgreement2::new()),
        AsrOptions { language: Some("en".into()), ..Default::default() },
    );
    let pcm = whisper_rs::audio::AudioInput::from_wav_file("tests/fixtures/jfk.wav").unwrap().to_mono_16k().unwrap();
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

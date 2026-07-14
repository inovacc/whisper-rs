use whisper_rs::audio::vad::{segment, VadConfig};

fn tone(n: usize, amp: f32) -> Vec<f32> {
    (0..n).map(|i| ((i as f32) * 0.2).sin() * amp).collect()
}

#[test]
fn silence_tone_silence_yields_one_span() {
    let sr = 16000;
    let mut sig = vec![0.0f32; sr / 2]; // 0.5s silence
    sig.extend(tone(sr, 0.5)); // 1.0s tone
    sig.extend(vec![0.0f32; sr / 2]); // 0.5s silence
    let spans = segment(&sig, sr as u32, &VadConfig::default());
    assert_eq!(spans.len(), 1, "spans={spans:?}");
    let (s, e) = spans[0];
    assert!((0.3..=0.7).contains(&s), "start {s}");
    assert!((1.4..=1.8).contains(&e), "end {e}");
}

#[test]
fn pure_silence_yields_no_spans() {
    assert!(segment(&vec![0.0f32; 16000], 16000, &VadConfig::default()).is_empty());
}

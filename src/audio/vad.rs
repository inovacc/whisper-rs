//! Energy-based voice-activity detection (pure, no model). Returns speech spans in seconds.

#[derive(Debug, Clone)]
pub struct VadConfig {
    pub frame_ms: u32,         // analysis frame size
    pub energy_threshold: f32, // RMS above this = speech
    pub min_speech_ms: u32,    // drop spans shorter than this
    pub hangover_ms: u32,      // extend speech past the last active frame
}
impl Default for VadConfig {
    fn default() -> Self {
        Self { frame_ms: 30, energy_threshold: 0.01, min_speech_ms: 100, hangover_ms: 100 }
    }
}

/// Return speech (start_s, end_s) spans. Frames the signal, marks frames whose RMS exceeds the
/// threshold, applies hangover, merges contiguous speech, and drops spans shorter than min_speech_ms.
pub fn segment(pcm: &[f32], sample_rate: u32, cfg: &VadConfig) -> Vec<(f32, f32)> {
    if pcm.is_empty() || sample_rate == 0 {
        return vec![];
    }
    let frame = ((cfg.frame_ms as f32 / 1000.0) * sample_rate as f32).max(1.0) as usize;
    let hangover_frames = (cfg.hangover_ms as f32 / cfg.frame_ms.max(1) as f32).ceil() as i32;
    // per-frame speech flags via RMS
    let mut active: Vec<bool> = Vec::new();
    let mut i = 0;
    while i < pcm.len() {
        let end = (i + frame).min(pcm.len());
        let rms = (pcm[i..end].iter().map(|s| s * s).sum::<f32>() / (end - i) as f32).sqrt();
        active.push(rms > cfg.energy_threshold);
        i += frame;
    }
    // apply hangover: keep speech active for N frames after the last active frame
    let mut hang = 0i32;
    let held: Vec<bool> = active
        .iter()
        .map(|&a| {
            if a {
                hang = hangover_frames;
                true
            } else if hang > 0 {
                hang -= 1;
                true
            } else {
                false
            }
        })
        .collect();
    // merge contiguous true runs into (start,end) seconds; drop short ones
    let min_frames = (cfg.min_speech_ms as f32 / cfg.frame_ms.max(1) as f32).ceil() as usize;
    let secs_per_frame = frame as f32 / sample_rate as f32;
    let mut spans = Vec::new();
    let mut run_start: Option<usize> = None;
    for (idx, &h) in held.iter().enumerate() {
        match (run_start, h) {
            (None, true) => run_start = Some(idx),
            (Some(s), false) => {
                if idx - s >= min_frames {
                    spans.push((s as f32 * secs_per_frame, idx as f32 * secs_per_frame));
                }
                run_start = None;
            }
            _ => {}
        }
    }
    // flush a trailing run that reaches the end of the signal
    if let Some(s) = run_start {
        let idx = held.len();
        if idx - s >= min_frames {
            spans.push((s as f32 * secs_per_frame, idx as f32 * secs_per_frame));
        }
    }
    spans
}

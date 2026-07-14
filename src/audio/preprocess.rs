//! Tiered audio preprocessing over 16 kHz mono f32 PCM (Galle 0-4 scheme). Pure DSP.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PreprocessLevel {
    #[default]
    L0,
    L1,
    L2,
    L3,
    L4,
}

/// Apply the given tier. Higher tiers compose lower ones. Output stays in [-1, 1].
///
/// Mutates a single owned buffer through the stages in place (rather than allocating a fresh
/// `Vec` per stage) so higher tiers don't pay for multiple full-length copies.
pub fn preprocess(pcm: &[f32], level: PreprocessLevel) -> Vec<f32> {
    let mut buf = pcm.to_vec();
    match level {
        PreprocessLevel::L0 => {}
        PreprocessLevel::L1 => normalize_peak_in_place(&mut buf, 0.95),
        PreprocessLevel::L2 => {
            remove_dc_in_place(&mut buf);
            normalize_peak_in_place(&mut buf, 0.95);
        }
        PreprocessLevel::L3 => {
            remove_dc_in_place(&mut buf);
            normalize_peak_in_place(&mut buf, 0.95);
            noise_gate_in_place(&mut buf, 0.02);
        }
        PreprocessLevel::L4 => {
            remove_dc_in_place(&mut buf);
            normalize_peak_in_place(&mut buf, 0.95);
            noise_gate_in_place(&mut buf, 0.05);
        }
    }
    buf
}

/// Subtract the mean (DC offset) in place.
fn remove_dc_in_place(pcm: &mut [f32]) {
    if pcm.is_empty() {
        return;
    }
    let mean = pcm.iter().sum::<f32>() / pcm.len() as f32;
    for s in pcm.iter_mut() {
        *s -= mean;
    }
}

/// Scale so the peak absolute value becomes `target` (no-op if already silent), in place.
fn normalize_peak_in_place(pcm: &mut [f32], target: f32) {
    let peak = pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak <= f32::EPSILON {
        return;
    }
    let g = target / peak;
    for s in pcm.iter_mut() {
        *s = (*s * g).clamp(-1.0, 1.0);
    }
}

/// Zero samples whose absolute value is below `threshold` (simple noise floor), in place.
fn noise_gate_in_place(pcm: &mut [f32], threshold: f32) {
    for s in pcm.iter_mut() {
        if s.abs() < threshold {
            *s = 0.0;
        }
    }
}

/// Subtract the mean (DC offset).
pub fn remove_dc(pcm: &[f32]) -> Vec<f32> {
    let mut out = pcm.to_vec();
    remove_dc_in_place(&mut out);
    out
}

/// Scale so the peak absolute value becomes `target` (no-op if already silent).
pub fn normalize_peak(pcm: &[f32], target: f32) -> Vec<f32> {
    let mut out = pcm.to_vec();
    normalize_peak_in_place(&mut out, target);
    out
}

/// Zero samples whose absolute value is below `threshold` (simple noise floor).
pub fn noise_gate(pcm: &[f32], threshold: f32) -> Vec<f32> {
    let mut out = pcm.to_vec();
    noise_gate_in_place(&mut out, threshold);
    out
}

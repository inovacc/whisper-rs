//! Tiered audio preprocessing over 16 kHz mono f32 PCM (Galle 0-4 scheme). Pure DSP.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreprocessLevel {
    L0,
    L1,
    L2,
    L3,
    L4,
}

/// Apply the given tier. Higher tiers compose lower ones. Output stays in [-1, 1].
pub fn preprocess(pcm: &[f32], level: PreprocessLevel) -> Vec<f32> {
    match level {
        PreprocessLevel::L0 => pcm.to_vec(),
        PreprocessLevel::L1 => normalize_peak(pcm, 0.95),
        PreprocessLevel::L2 => normalize_peak(&remove_dc(pcm), 0.95),
        PreprocessLevel::L3 => noise_gate(&normalize_peak(&remove_dc(pcm), 0.95), 0.02),
        PreprocessLevel::L4 => noise_gate(&normalize_peak(&remove_dc(pcm), 0.95), 0.05),
    }
}

/// Subtract the mean (DC offset).
pub fn remove_dc(pcm: &[f32]) -> Vec<f32> {
    if pcm.is_empty() {
        return vec![];
    }
    let mean = pcm.iter().sum::<f32>() / pcm.len() as f32;
    pcm.iter().map(|s| s - mean).collect()
}

/// Scale so the peak absolute value becomes `target` (no-op if already silent).
pub fn normalize_peak(pcm: &[f32], target: f32) -> Vec<f32> {
    let peak = pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak <= f32::EPSILON {
        return pcm.to_vec();
    }
    let g = target / peak;
    pcm.iter().map(|s| (s * g).clamp(-1.0, 1.0)).collect()
}

/// Zero samples whose absolute value is below `threshold` (simple noise floor).
pub fn noise_gate(pcm: &[f32], threshold: f32) -> Vec<f32> {
    pcm.iter().map(|s| if s.abs() < threshold { 0.0 } else { *s }).collect()
}

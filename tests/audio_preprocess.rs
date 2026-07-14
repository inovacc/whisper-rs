use whisper_rs::audio::preprocess::{preprocess, remove_dc, PreprocessLevel};

#[test]
fn l0_is_identity() {
    let x = vec![0.1, -0.2, 0.3];
    assert_eq!(preprocess(&x, PreprocessLevel::L0), x);
}
#[test]
fn l1_normalizes_peak_to_0_95() {
    let out = preprocess(&[0.1, -0.4, 0.2], PreprocessLevel::L1);
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((peak - 0.95).abs() < 1e-4, "peak={peak}");
}
#[test]
fn remove_dc_zeroes_mean() {
    let out = remove_dc(&[1.0, 1.0, 1.0, 1.0]); // constant -> all zero after DC removal
    assert!(out.iter().all(|s| s.abs() < 1e-6));
}
#[test]
fn l3_gates_low_amplitude_noise() {
    // loud tone samples + tiny noise; after L3 the tiny samples are zeroed
    let out = preprocess(&[0.5, 0.001, -0.5, 0.0005], PreprocessLevel::L3);
    assert_eq!(out[1], 0.0);
    assert_eq!(out[3], 0.0);
    assert!(out[0].abs() > 0.1);
}
#[test]
fn all_levels_stay_in_range() {
    let x: Vec<f32> = (0..100).map(|i| ((i as f32) * 0.3).sin() * 2.0).collect(); // out-of-range input
    // L0 is documented identity (passes input through verbatim), so it is intentionally excluded
    // from this range assertion.
    for lvl in [PreprocessLevel::L1, PreprocessLevel::L2, PreprocessLevel::L3, PreprocessLevel::L4] {
        assert!(preprocess(&x, lvl).iter().all(|s| s.abs() <= 1.0001), "{lvl:?} out of range");
    }
}

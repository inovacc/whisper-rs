#![cfg(feature = "diarization")]
use whisper_rs::diarize::cluster::agglomerative;

#[test]
fn two_clear_clusters() {
    let embs = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.98, 0.02, 0.0], // group A
        vec![0.0, 1.0, 0.0],
        vec![0.01, 0.99, 0.0], // group B
    ];
    let groups = agglomerative(&embs, 0.3, None);
    assert_eq!(groups[0], groups[1]); // A members together
    assert_eq!(groups[2], groups[3]); // B members together
    assert_ne!(groups[0], groups[2]); // A != B
    assert_eq!(groups.iter().cloned().collect::<std::collections::BTreeSet<_>>().len(), 2);
}

#[test]
fn max_speakers_caps_groups() {
    let embs = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![-1.0, 0.0]];
    let groups = agglomerative(&embs, 0.1, Some(2)); // force <=2 groups despite 3 distinct
    assert!(groups.iter().cloned().collect::<std::collections::BTreeSet<_>>().len() <= 2);
}

/// Determinism/regression guard for the O(n^2) matrix-cached rewrite: 30
/// embeddings in 3 well-separated clusters (10 each, each near a distinct
/// basis direction with a small deterministic-by-index perturbation) must
/// still resolve to exactly 3 groups with the expected memberships.
#[test]
fn thirty_points_three_clusters_deterministic() {
    let mut embs = Vec::with_capacity(30);
    // Cluster 0: near the x-axis, indices 0..10.
    for i in 0..10 {
        let perturb = 0.001 * i as f32;
        embs.push(vec![1.0, perturb, 0.0]);
    }
    // Cluster 1: near the y-axis, indices 10..20.
    for i in 0..10 {
        let perturb = 0.001 * i as f32;
        embs.push(vec![perturb, 1.0, 0.0]);
    }
    // Cluster 2: near the z-axis, indices 20..30.
    for i in 0..10 {
        let perturb = 0.001 * i as f32;
        embs.push(vec![0.0, perturb, 1.0]);
    }

    let groups = agglomerative(&embs, 0.3, None);
    assert_eq!(groups.len(), 30);

    let distinct: std::collections::BTreeSet<usize> = groups.iter().cloned().collect();
    assert_eq!(distinct.len(), 3, "expected exactly 3 groups, got {distinct:?}");

    // All members within each source cluster must share a label, and the
    // three clusters must map to three different labels.
    for chunk in groups.chunks(10) {
        assert!(chunk.iter().all(|&g| g == chunk[0]));
    }
    assert_ne!(groups[0], groups[10]);
    assert_ne!(groups[0], groups[20]);
    assert_ne!(groups[10], groups[20]);
}

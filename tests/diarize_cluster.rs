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

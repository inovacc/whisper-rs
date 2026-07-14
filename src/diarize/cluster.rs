//! Pure agglomerative clustering over embedding vectors (cosine distance,
//! average linkage). No ONNX / model dependency — operates purely on
//! already-computed embedding vectors.

/// Cluster `embeddings` via agglomerative (average-linkage, cosine-distance)
/// clustering.
///
/// Each embedding starts in its own cluster. The two closest clusters are
/// repeatedly merged as long as either:
/// - the current cluster count exceeds `max` (when `max` is `Some`), forcing
///   merges to respect the cap even past `threshold`; or
/// - the minimum inter-cluster distance is at or below `threshold`.
///
/// Returns a group index per input embedding, relabeled `0..k` in order of
/// first appearance.
pub fn agglomerative(embeddings: &[Vec<f32>], threshold: f32, max: Option<usize>) -> Vec<usize> {
    let n = embeddings.len();
    if n == 0 {
        return Vec::new();
    }

    let mut clusters: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

    loop {
        let count = clusters.len();
        if count <= 1 {
            break;
        }

        let mut best: Option<(f32, usize, usize)> = None;
        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                let d = average_linkage(&clusters[i], &clusters[j], embeddings);
                match best {
                    Some((best_dist, _, _)) if d >= best_dist => {}
                    _ => best = Some((d, i, j)),
                }
            }
        }
        let (min_dist, i, j) = best.expect("at least two clusters implies a pair exists");

        let must_force = max.is_some_and(|m| count > m);
        let within_threshold = min_dist <= threshold;
        if !must_force && !within_threshold {
            break;
        }

        let cj = clusters.remove(j);
        clusters[i].extend(cj);
    }

    relabel_by_first_appearance(&clusters, n)
}

fn relabel_by_first_appearance(clusters: &[Vec<usize>], n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..clusters.len()).collect();
    order.sort_by_key(|&ci| clusters[ci].iter().copied().min().unwrap_or(usize::MAX));

    let mut remap = vec![0usize; clusters.len()];
    for (new_label, &old_index) in order.iter().enumerate() {
        remap[old_index] = new_label;
    }

    let mut labels = vec![0usize; n];
    for (old_index, cluster) in clusters.iter().enumerate() {
        for &member in cluster {
            labels[member] = remap[old_index];
        }
    }
    labels
}

fn average_linkage(a: &[usize], b: &[usize], embeddings: &[Vec<f32>]) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for &ai in a {
        for &bi in b {
            total += cosine_distance(&embeddings[ai], &embeddings[bi]);
            count += 1;
        }
    }
    if count == 0 {
        return f32::INFINITY;
    }
    total / count as f32
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }
    1.0 - (dot / (norm_a * norm_b))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn empty_input_returns_empty() {
        let groups = agglomerative(&[], 0.3, None);
        assert!(groups.is_empty());
    }

    #[test]
    fn single_embedding_forms_one_group() {
        let groups = agglomerative(&[vec![1.0, 0.0]], 0.3, None);
        assert_eq!(groups, vec![0]);
    }
}

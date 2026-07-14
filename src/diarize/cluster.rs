//! Pure agglomerative clustering over embedding vectors (cosine distance,
//! average linkage). No ONNX / model dependency — operates purely on
//! already-computed embedding vectors.
//!
//! Implementation note: embeddings are L2-normalized once up front (so
//! cosine distance reduces to `1.0 - dot`), and a pairwise distance matrix
//! is cached and incrementally updated on each merge via the Lance-Williams
//! average-linkage recurrence, instead of rescanning every member pair of
//! every cluster on every iteration.

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

    let normalized = normalize_all(embeddings);

    // Cluster "slots" are fixed at their initial singleton index for the
    // whole run. A merge of (i, j) with i < j always absorbs j into i, so
    // slot i's position never moves — this mirrors the positional
    // invariant of the original `Vec<Vec<usize>>` + `remove(j)` approach
    // (surviving clusters keep their relative order, and a merged cluster's
    // minimum member is always its own slot index).
    let mut alive = vec![true; n];
    let mut sizes = vec![1usize; n];
    // parent[j] = i once j has been merged into i; used only to resolve the
    // final label of each original embedding after the loop.
    let mut parent: Vec<usize> = (0..n).collect();

    // Pairwise distance matrix over normalized embeddings, initialized to
    // the singleton average-linkage distance (== the pair's own distance).
    let mut dist = vec![0.0f32; n * n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = cosine_distance_normalized(&normalized[i], &normalized[j]);
            dist[i * n + j] = d;
            dist[j * n + i] = d;
        }
    }

    let mut count = n;
    while count > 1 {
        let mut best: Option<(f32, usize, usize)> = None;
        for i in 0..n {
            if !alive[i] {
                continue;
            }
            for j in (i + 1)..n {
                if !alive[j] {
                    continue;
                }
                let d = dist[i * n + j];
                match best {
                    Some((best_dist, _, _)) if d >= best_dist => {}
                    _ => best = Some((d, i, j)),
                }
            }
        }
        let (min_dist, i, j) = best.expect("at least two live clusters implies a pair exists");

        let must_force = max.is_some_and(|m| count > m);
        let within_threshold = min_dist <= threshold;
        if !must_force && !within_threshold {
            break;
        }

        // Absorb j into i (i < j, so i keeps its slot; j is marked dead
        // instead of removed).
        let na = sizes[i];
        let nb = sizes[j];
        for k in 0..n {
            if k == i || k == j || !alive[k] {
                continue;
            }
            let d_ik = dist[i * n + k];
            let d_jk = dist[j * n + k];
            let new_d = (na as f32 * d_ik + nb as f32 * d_jk) / (na + nb) as f32;
            dist[i * n + k] = new_d;
            dist[k * n + i] = new_d;
        }
        sizes[i] = na + nb;
        alive[j] = false;
        parent[j] = i;
        count -= 1;
    }

    relabel_by_first_appearance(&alive, &parent, n)
}

/// Relabel each original embedding index 0..k by the ascending order of the
/// surviving ("alive") slot it ultimately belongs to. Since a merge always
/// absorbs the higher slot index into the lower one, a slot's index is
/// always the minimum original member index of its final cluster, so
/// iterating alive slots in ascending order reproduces "relabel by first
/// appearance" exactly.
fn relabel_by_first_appearance(alive: &[bool], parent: &[usize], n: usize) -> Vec<usize> {
    let mut remap = vec![0usize; n];
    let mut next_label = 0usize;
    for (i, &is_alive) in alive.iter().enumerate() {
        if is_alive {
            remap[i] = next_label;
            next_label += 1;
        }
    }

    let mut labels = vec![0usize; n];
    for (x, label) in labels.iter_mut().enumerate() {
        let root = find_root(parent, x);
        *label = remap[root];
    }
    labels
}

fn find_root(parent: &[usize], start: usize) -> usize {
    let mut x = start;
    while parent[x] != x {
        x = parent[x];
    }
    x
}

fn normalize_all(embeddings: &[Vec<f32>]) -> Vec<Vec<f32>> {
    embeddings.iter().map(|v| normalize(v)).collect()
}

fn normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        vec![0.0; v.len()]
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

/// Cosine distance between two already L2-normalized vectors: `1.0 - dot`.
/// A zero-norm input (normalized to the zero vector) yields `dot == 0.0`,
/// i.e. distance `1.0`, matching the un-normalized fallback for degenerate
/// (all-zero) embeddings.
fn cosine_distance_normalized(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    1.0 - dot
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

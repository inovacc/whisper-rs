# Plan 006: Speed up agglomerative clustering from O(n³·d) to ~O(n²)

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- src/diarize/cluster.rs tests/diarize_cluster.rs`

## Status
- **Priority**: P2
- **Effort**: M
- **Risk**: LOW (pure function with existing unit tests; behavior preserved)
- **Depends on**: none
- **Category**: perf
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
`diarize::cluster::agglomerative` recomputes every pairwise linkage on every merge, and `cosine_distance`
recomputes both vector norms from scratch on each call. Total work is ~O(n³·d) for n embedding windows. It's
currently latent (no ONNX `Diarizer` feeds it yet — only tests reach it), so this is the right time to fix it
*before* the model path makes n large (a long meeting is hundreds–thousands of windows → minutes-to-hours).
Pre-normalizing embeddings once (so cosine distance = 1 − dot) and caching a pairwise distance matrix with a
Lance–Williams average-linkage update brings it to ~O(n²).

## Current state (verify before editing — read the whole file)
`src/diarize/cluster.rs`:
- `agglomerative(embeddings: &[Vec<f32>], threshold: f32, max: Option<usize>) -> Vec<usize>` — a `loop` that,
  each iteration, scans every cluster pair `(i,j)`, computes `average_linkage`, and merges the closest while
  `min_dist <= threshold` and (`max` is None or `count > max`). `clusters.remove(j)` is O(n).
- `average_linkage(a, b, embeddings)` iterates all member pairs, calling `cosine_distance`.
- `cosine_distance(a, b)` computes dot + `norm_a` + `norm_b` (two sqrts) every call.
- Returns a group index per input, relabeled 0..k by first appearance.
- Tests: `tests/diarize_cluster.rs` — `two_clear_clusters` (2 groups) and `max_speakers_caps_groups` (≤2).
- Convention: pure, no `unsafe`, no panics.

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Cluster tests | `cargo test --features diarization --test diarize_cluster` | all pass |
| Full tests | `cargo test --features diarization` | all pass |
| Lint | `cargo clippy --all-targets --features diarization -- -D warnings` | exit 0 |

## Scope
**In scope:** `src/diarize/cluster.rs`, `tests/diarize_cluster.rs` (may add a determinism/perf-shape test).
**Out of scope:** the public signature of `agglomerative` (keep `(&[Vec<f32>], f32, Option<usize>) -> Vec<usize>`
and its exact grouping semantics — the existing tests must pass unchanged), `src/diarize/merge.rs`, any ONNX
work. Do NOT add a dependency (`ndarray` etc.) — plain Rust.

## Git workflow
- Branch `advisor/006-clustering-perf`; message `perf(diarize): cache distances + normalize embeddings in clustering`. Do NOT push.

## Steps
### Step 1: Pre-normalize embeddings once
At the top of `agglomerative`, compute a unit-normalized copy of each embedding (divide by its L2 norm;
guard zero norm → leave as zeros). After normalization, cosine distance between two vectors = `1.0 - dot(a,b)`.
Replace `cosine_distance`'s per-call norm computation with a dot-only distance over normalized vectors.

**Verify**: `cargo test --features diarization --test diarize_cluster` → still pass.

### Step 2: Cache a pairwise distance matrix with Lance–Williams updates
Represent clusters as index sets with sizes. Precompute an initial n×n distance matrix over normalized
embeddings (average-linkage between singletons = the pair distance). Each iteration: find the min off-diagonal
entry among live clusters; if `> threshold` (and the `max`-speakers force condition isn't triggering a merge),
stop. On merging clusters `a` and `b` (sizes `na`, `nb`), update the merged cluster's distance to every other
live cluster `c` via the size-weighted average-linkage (Lance–Williams):
`d(ab, c) = (na*d(a,c) + nb*d(b,c)) / (na + nb)`. Mark `b` dead instead of `Vec::remove`. Preserve the existing
tie-breaking (earliest pair) and the `max`-speakers cap semantics exactly. Relabel groups 0..k by first
appearance as before.

**Verify**: `cargo test --features diarization --test diarize_cluster` → both existing tests pass unchanged.

### Step 3: Add a determinism test over a larger input
In `tests/diarize_cluster.rs` add a test with, say, 30 embeddings in 3 well-separated clusters (10 each,
each cluster near a distinct basis direction with small perturbations by index — do NOT use randomness; vary
by a deterministic function of the index) and assert exactly 3 groups with the right memberships. This guards
the O(n²) rewrite against grouping regressions on non-trivial n.

**Verify**: `cargo test --features diarization --test diarize_cluster` → all pass (3 tests).

## Test plan
`tests/diarize_cluster.rs`: keep `two_clear_clusters` + `max_speakers_caps_groups` (must pass unchanged — they
lock the semantics), add the 30-point determinism test. Average-linkage + tie-break behavior must be identical
to today for these inputs. Verification: `cargo test --features diarization --test diarize_cluster`.

## Done criteria
- [ ] Both existing cluster tests pass unchanged; the new 30-point test passes
- [ ] `cosine_distance` no longer recomputes norms per call (embeddings normalized once) — confirm by reading
- [ ] No `Vec::remove` inside the merge loop (dead-marking instead) — `grep -n "\.remove(" src/diarize/cluster.rs` shows none in the hot loop
- [ ] `cargo test --features diarization` all pass; clippy clean; `cargo build --no-default-features` exit 0
- [ ] Only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions
- The Lance–Williams rewrite changes the grouping for the existing tests and can't be reconciled to identical
  output — the semantics differ; report rather than editing the tests to match.
- Reproducing the exact tie-break (earliest pair) under the matrix representation is ambiguous — report the
  ambiguity with the specific case.

## Maintenance notes
- When the ONNX embedding path (BACKLOG P1) feeds real n, revisit the `threshold` default and consider a
  distance-matrix memory cap for very large n. A reviewer should verify the merged-row update matches
  average-linkage (size-weighted), not single/complete linkage.

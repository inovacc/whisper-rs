# Plan 010: Small cleanups — in-place preprocess, token-join helper, cache dir, hallucination doc

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done. These four items are independent — you may commit
> each separately and skip any that hits a STOP condition without blocking the others.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- src/audio/preprocess.rs src/stream src/models/mod.rs src/postprocess/hallucination.rs`

## Status
- **Priority**: P3
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none (item B touches `src/stream/*` — if plan 001 is in flight in another worktree,
  coordinate or do item B after 001 to avoid conflicts)
- **Category**: tech-debt / perf / docs
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
Four low-risk quality items surfaced by the audit, each small and independent: (A) the preprocessing chain
allocates a full-length `Vec` per DSP stage (L3/L4 copy the whole buffer 3×); (B) the token-list→string join
is duplicated verbatim in three places (rule-of-three met); (C) `default_cache_dir` is CWD-relative, so the
model cache location changes with the process's working directory; (D) `flag_hallucinations` flags segments
with no overlapping second-pass counterpart, which is intended but undocumented and surprising.

## Current state (verify each before editing)
- **A** — `src/audio/preprocess.rs`: `preprocess(pcm, level)` composes `remove_dc` → `normalize_peak` →
  `noise_gate`, each `-> Vec<f32>` via `.map().collect()`. L3/L4 allocate 3 full-length vecs.
- **B** — identical `iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ")` at
  `src/stream/session.rs:77`, `src/stream/local_agreement.rs` (~:42-46), `src/stream/two_pass.rs` (~:29-33).
- **C** — `src/models/mod.rs:41-43`: `default_cache_dir() -> PathBuf { PathBuf::from("models") }`.
- **D** — `src/postprocess/hallucination.rs:27-32`: `flag_hallucinations` uses `fold(0.0, f32::max)` over
  time-overlapping secondary segments; no overlap ⇒ `best = 0.0 < threshold` ⇒ flagged. Docstring says
  "best similarity … below threshold" without noting the no-overlap case.
- Conventions: no `unsafe`, no panics; streaming code behind `#[cfg(feature="streaming")]`.

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Full tests | `cargo test --all-features` | all pass |
| No-default build | `cargo build --no-default-features` | exit 0 |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |

## Scope
**In scope:** `src/audio/preprocess.rs` (A), `src/stream/mod.rs` + the three stream files (B),
`src/models/mod.rs` (C), `src/postprocess/hallucination.rs` (D), and any test that asserts these behaviors.
**Out of scope:** changing the *result* of any transform (A must be byte-identical output), the streaming
policy logic (B is a pure refactor of the join only), and `PostConfig::apply` (D is a docstring + optional
behavior toggle, not a wiring change).

## Git workflow
- Branch `advisor/010-cleanups`; one commit per item (`perf(audio): preprocess in place`, `refactor(stream): extract join_tokens`, `fix(download): stable default cache dir`, `docs(postprocess): document no-overlap hallucination flag`). Do NOT push.

## Steps
### Step A: In-place preprocessing
Refactor `preprocess` to mutate a single owned `Vec<f32>` through the stages instead of allocating per stage:
compute DC mean in one pass and subtract in place; compute peak in one pass and scale in place; apply the gate
in place. Output MUST be identical to today for the existing tests. Keep the public `preprocess(pcm: &[f32],
level) -> Vec<f32>` signature (clone into an owned vec once at the top, then mutate). Keep `remove_dc`/
`normalize_peak`/`noise_gate` as public helpers if they're tested — but you may add in-place private variants.

**Verify**: `cargo test --test audio_preprocess` → all existing tests pass unchanged.

### Step B: Extract `join_tokens`
Add `pub(crate) fn join_tokens(tokens: &[Token]) -> String` to `src/stream/mod.rs` (join by single space,
avoiding the throwaway `Vec` — e.g. build with `String` + push). Replace all three duplicated call sites with
it. Behavior identical.

**Verify**: `cargo test --features streaming` → all pass; `git grep -n "map(|t| t.text.as_str())" src/stream`
returns no matches.

### Step C: Stable default cache dir
Change `default_cache_dir` to resolve a stable, non-CWD-relative location: honor an env override
`WHISPER_RS_CACHE_DIR` if set; else use a per-user cache dir. To avoid a new dependency, use
`std::env::var_os("WHISPER_RS_CACHE_DIR")` → else `std::env::var_os("LOCALAPPDATA")`/`XDG_CACHE_HOME`/`HOME`
joined with `whisper-rs/models`, falling back to `PathBuf::from("models")` only if none resolve. Keep it
dependency-free — if implementing a robust per-OS cache dir cleanly requires the `dirs` crate, **STOP and
report** (dependency decision) and keep the env-override + `models` fallback only.

**Verify**: `cargo build --features download` → exit 0. Add a test in `tests/models.rs`:
`env_override_sets_cache_dir` — set `WHISPER_RS_CACHE_DIR` via `std::env::set_var` (in a `#[test]` that also
restores it), assert `default_cache_dir()` reflects it. (Note: env-var tests can be order-sensitive; keep it
self-contained.)

### Step D: Document the no-overlap hallucination behavior
In `src/postprocess/hallucination.rs`, expand the `flag_hallucinations` doc comment to state explicitly: "A
primary segment with **no time-overlapping** secondary segment scores 0 and is therefore flagged — absence of
a cross-pass counterpart is treated as a hallucination signal." No logic change. (Optional, only if trivial: a
`flag_hallucinations_opts` with a `flag_when_no_overlap: bool` — but keep the default behavior; if this grows
beyond a couple of lines, skip it and keep just the doc.)

**Verify**: `cargo test --all-features` → all pass (the existing `no_overlapping_secondary_flags_suspect` test
still passes, confirming behavior is unchanged).

## Test plan
- A: existing `tests/audio_preprocess.rs` must pass unchanged (proves identical output).
- B: existing streaming tests pass; grep proves the dup is gone.
- C: new `env_override_sets_cache_dir` in `tests/models.rs`.
- D: existing `tests/hallucination.rs` passes unchanged.
Verification: `cargo test --all-features`.

## Done criteria
- [ ] Preprocess output unchanged (audio_preprocess tests pass); fewer per-stage allocations (read to confirm)
- [ ] `join_tokens` helper exists; the three duplicated joins are gone (grep)
- [ ] `default_cache_dir` honors `WHISPER_RS_CACHE_DIR` and isn't bare-CWD `"models"` (unless all env lookups fail); env-override test passes
- [ ] `flag_hallucinations` doc explains the no-overlap case; behavior unchanged
- [ ] `cargo test --all-features` all pass; clippy clean; `cargo build --no-default-features` exit 0
- [ ] Only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions
- Any item changes a transform's output such that an existing test fails — that item's refactor is wrong;
  revert just that item and report (the others can still land).
- Item C cleanly needs the `dirs` crate — do the env-override + `models` fallback only and report the
  dependency question.

## Maintenance notes
- Item A: if more preprocessing tiers are added, keep them in-place. Item C: document `WHISPER_RS_CACHE_DIR` in
  README alongside the downloader. A reviewer should confirm item A produces byte-identical output (diff a
  preprocessed buffer before/after if unsure).

# Plan 007: Flush the resampler delay line so timestamps stay aligned

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- src/audio/mod.rs tests/audio.rs`

## Status
- **Priority**: P2
- **Effort**: M
- **Risk**: MED (resampler reconfiguration can change output length — tests assert length within tolerance)
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
`AudioInput::resample` runs `rubato::SincFixedIn` as a single `process(&[mono], None)` call with `sinc_len:
256` and never drains the resampler's internal delay line. `SincFixedIn` has an inherent group delay
(~`sinc_len/2` input samples); leaving it undrained shifts the resampled PCM earlier and drops the trailing
~8 ms. Every downstream whisper timestamp inherits a small constant offset, and the end of non-16 kHz audio
can be clipped. (16 kHz input is unaffected — it takes the passthrough.) This plan drains the delay line so
the output length and alignment are correct.

## Current state (verify before editing)
`src/audio/mod.rs`:
- `to_mono_16k(&self)` returns `mono` directly when `sample_rate == TARGET_RATE` (16000) — passthrough.
- `resample(&self, mono: Vec<f32>) -> Result<Vec<f32>>` (lines ~59-72):
  ```rust
  let ratio = TARGET_RATE as f64 / self.sample_rate as f64;
  let params = SincInterpolationParameters { sinc_len: 256, f_cutoff: 0.95, oversampling_factor: 256, interpolation: SincInterpolationType::Linear, window: WindowFunction::BlackmanHarris2 };
  let mut rs = SincFixedIn::<f32>::new(ratio, 2.0, params, mono.len(), 1)?;
  let out = rs.process(&[mono], None)?;
  Ok(out.into_iter().next().unwrap_or_default())
  ```
- Test `tests/audio.rs::decodes_and_resamples_to_16k_mono` asserts `(pcm.len() - 16000).abs() < 400` for a 1 s
  8 kHz→16 kHz input, and `empty_input_returns_empty`.
- `rubato` version is pinned in `Cargo.toml` (`rubato = "0.15"`). Check the exact `SincFixedIn` /
  `process_partial` API for that version before coding.
- Convention: no `unsafe`, no panics; return `WhisperError::Resample` on error.

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Audio tests | `cargo test --test audio` | all pass |
| Full tests | `cargo test` | all pass |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Docs (rubato API) | read `https://docs.rs/rubato/0.15` `Resampler::process_partial` / `output_delay` | — |

## Scope
**In scope:** `src/audio/mod.rs` (the `resample` method), `tests/audio.rs`.
**Out of scope:** `preprocess.rs`, `vad.rs`, the 16 kHz passthrough path, `Cargo.toml` (do NOT bump rubato).

## Git workflow
- Branch `advisor/007-resample-flush`; message `fix(audio): drain resampler delay so timestamps align`. Do NOT push.

## Steps
### Step 1: Account for the resampler delay
Choose ONE approach (prefer A; fall back to B only if A's API isn't available in rubato 0.15):

**A. Flush with `process_partial`.** After the main `process` call, call `rs.process_partial::<Vec<f32>>(None)`
(or the 0.15 equivalent) to drain the tail, and concatenate its output. Then trim the leading
`rs.output_delay()` samples (rubato exposes the output delay) so the result starts at t=0. Net effect: output
length ≈ `input_len * ratio` with correct alignment.

**B. Compensate for a known delay.** If `process_partial`/`output_delay` aren't available in the pinned
version, compute the delay as `sinc_len/2` scaled by `ratio`, and pad the input with that many trailing zeros
before `process` (so the real tail is emitted), then drop the leading delay samples from the output.

Keep the empty-input guard (`mono.is_empty()` → `Ok(vec![])`) intact.

**Verify**: `cargo build` → exit 0.

### Step 2: Tighten the length assertion
Update `tests/audio.rs::decodes_and_resamples_to_16k_mono` to assert the output length is within a *smaller*
tolerance of the expected `16000` (e.g. `< 80` samples, ~5 ms) now that the delay is drained — this is the
regression guard proving the fix. If the fix legitimately can't reach ±80, pick the smallest tolerance the
corrected resampler actually achieves and document why in a comment (the point is the tolerance shrinks from
±400).

Add a test asserting the output isn't shifted: feed a signal that is silence → impulse/step at a known time,
resample, and assert the feature appears at approximately the same time (±a few ms), proving no gross offset.

**Verify**: `cargo test --test audio` → all pass.

## Test plan
`tests/audio.rs`: tighten the existing length test (±80 or the true achievable), add an alignment test
(feature at known time survives resample without a large shift). Keep `empty_input_returns_empty`.
Verification: `cargo test --test audio`.

## Done criteria
- [ ] `resample` drains the delay line (Step 1 A or B) — confirm by reading
- [ ] The length tolerance in `tests/audio.rs` is tighter than the old ±400 and the test passes
- [ ] The alignment test passes; `empty_input_returns_empty` still passes
- [ ] `cargo test` all pass; clippy clean; `cargo build --no-default-features` exit 0
- [ ] Only `src/audio/mod.rs` and `tests/audio.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions
- rubato 0.15 exposes neither `process_partial` nor `output_delay` and approach B's math can't hit a tighter
  tolerance than ±400 — report; the fix may need a rubato bump (a maintainer decision, out of scope here).
- Draining the delay makes the model-gated ASR/pipeline tests (`--ignored`) fail on transcription content —
  report (an alignment change shouldn't break content, but verify).

## Maintenance notes
- If `rubato` is later bumped or swapped to `FftFixedIn`, re-verify `output_delay` semantics. A reviewer
  should confirm the drained output length matches `input_len * ratio` within the tightened tolerance and that
  16 kHz passthrough is untouched.

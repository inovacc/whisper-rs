# Plan 004: Fix VAD `min_speech_ms` — it is inert at default settings

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- src/audio/vad.rs tests/audio_vad.rs`
> If in-scope files changed, re-verify "Current state" before editing.

## Status
- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
`audio::vad::segment` applies the hangover extension *before* the `min_speech_ms` run-length filter. With the
defaults (`hangover_ms=100` → 4 frames, `min_speech_ms=100` → 4 frames), a single above-threshold frame (a
click/tap) is extended by 4 hangover frames into a ~5-frame run, which then passes the `>= min_frames` (4)
check. So `min_speech_ms` — documented as "drop spans shorter than this" — silently rejects nothing at
defaults, and isolated noise blips produce speech spans. The fix measures the *active* (pre-hangover) run
length against `min_speech_ms`, then applies hangover only for span boundaries.

## Current state (verify before editing)
`src/audio/vad.rs` (read the whole function). Key facts:
- Lines ~24-32: per-frame `active: Vec<bool>` computed by RMS > `energy_threshold`.
- Lines ~33-48: `held: Vec<bool>` = `active` with hangover applied (a `true` sets `hang = hangover_frames`;
  subsequent `false` frames stay `true` while `hang > 0`).
- Lines ~49-65 + a trailing-run flush: contiguous `held` runs are merged into `(start_s, end_s)` and kept only
  if `idx - s >= min_frames` where `min_frames = ceil(min_speech_ms / frame_ms)`.
- `VadConfig { frame_ms, energy_threshold, min_speech_ms, hangover_ms }`, `Default = {30, 0.01, 100, 100}`.
- Tests: `tests/audio_vad.rs` — `silence_tone_silence_yields_one_span` and `pure_silence_yields_no_spans`.
- Convention: pure function, no `unsafe`, no panics.

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Tests | `cargo test --test audio_vad` | all pass |
| Full tests | `cargo test` | all pass |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |

## Scope
**In scope:** `src/audio/vad.rs`, `tests/audio_vad.rs`.
**Out of scope:** the energy-threshold logic, `VadConfig` defaults (don't change default values), and any
consumer of `segment` (nothing wires VAD into the Pipeline yet).

## Git workflow
- Branch `advisor/004-vad-min-speech`; message `fix(audio): measure VAD min-speech on active frames, not post-hangover`. Do NOT push.

## Steps
### Step 1: Track active-run length separately from held (hangover) length
Modify `segment` so the `min_speech_ms` filter tests the count of *active* frames in the run, not the
hangover-padded length. Approach: when merging `held` runs into spans, also count how many frames in that run
were `active` (true in the original `active` vec), and keep the span only if that active count `>= min_frames`.
Concretely, iterate with both `active` and `held`, and for each merged `held` run compute
`active_count = active[run_start..run_end].iter().filter(|&&a| a).count()`, then keep if
`active_count >= min_frames`. Keep the span boundaries from the `held` (hangover-extended) run so trailing
context is preserved.

**Verify**: `cargo build` → exit 0.

### Step 2: Add a regression test
In `tests/audio_vad.rs` add a test proving the guard now works: build a signal with a single very short
above-threshold blip (e.g. one 30 ms frame of tone) surrounded by silence, with default `VadConfig`, and
assert `segment(...)` returns **no** spans (the blip is shorter than `min_speech_ms=100`). Keep the existing
two tests passing (a 1 s tone still yields exactly one span).

```rust
#[test]
fn short_blip_below_min_speech_is_dropped() {
    let sr = 16000;
    let mut sig = vec![0.0f32; sr/2];
    sig.extend((0..(sr/33)).map(|i| ((i as f32)*0.2).sin()*0.5)); // ~30ms tone
    sig.extend(vec![0.0f32; sr/2]);
    assert!(segment(&sig, sr as u32, &VadConfig::default()).is_empty());
}
```
(If a single 30 ms frame is exactly `min_frames`, make the blip clearly shorter, e.g. `sr/50` samples, so the
active-frame count is `< 4`. Tune the blip length, NOT the algorithm, so the test asserts the intended
"too-short is dropped" behavior.)

**Verify**: `cargo test --test audio_vad` → all pass (3 tests).

## Test plan
Tests in `tests/audio_vad.rs`: the new `short_blip_below_min_speech_is_dropped` (the regression), plus the
existing `silence_tone_silence_yields_one_span` (must still pass — a real 1 s tone survives) and
`pure_silence_yields_no_spans`. Verification: `cargo test --test audio_vad` → all pass.

## Done criteria
- [ ] `cargo test --test audio_vad` passes with the new regression test present
- [ ] `cargo test` (full suite) passes; `cargo clippy --all-targets -- -D warnings` exit 0
- [ ] The existing one-span tone test still passes (no regression)
- [ ] Only `src/audio/vad.rs` and `tests/audio_vad.rs` modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions
- `vad.rs` doesn't match "Current state" (drift) — report.
- The existing `silence_tone_silence_yields_one_span` test starts failing and can't be kept passing by tuning
  only the new test's blip length — the algorithm change is wrong; report.

## Maintenance notes
- When VAD is wired into streaming (BACKLOG P3), this active-vs-held distinction matters for chunk boundaries;
  keep the span boundaries hangover-extended (so trailing consonants aren't clipped) while the *keep/drop*
  decision uses active-frame count. A reviewer should confirm both existing tests still pass.

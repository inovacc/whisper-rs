# Plan 001: Fix streaming finalize/commit and make `StreamSession` unit-testable

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the
> expected result before moving on. If anything in "STOP conditions" occurs, stop and report — do not
> improvise. When done, update this plan's status row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- src/stream src/asr/mod.rs src/pipeline.rs`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the
> live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status
- **Priority**: P1
- **Effort**: M
- **Risk**: MED (touches the `StreamPolicy` trait contract + `StreamSession` construction)
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
The streaming API has two silent data-loss bugs. (1) `StreamSession::finalize()` promises to "commit
everything remaining" but is identical to `poll()` — it never flushes the uncommitted tail, so the final
words of every stream are lost (emitted only as `PartialText`). (2) The `TwoPass` policy commits **nothing**
through a `StreamSession`, because the session only ever calls `StreamPolicy::observe` (which `TwoPass`
always returns empty from) and never `observe_final`. Both bugs hid because `StreamSession` embeds a
concrete `Transcriber` (which needs a real model), so its logic has zero offline test coverage. This plan
adds a `Transcribe` trait seam, fixes finalize/commit, and fixes committed-segment timings — with real
unit tests.

## Current state

- `src/stream/mod.rs:30-42` — the `StreamPolicy` trait has only `observe`; `StreamSession` is re-exported.
  ```rust
  pub trait StreamPolicy {
      fn observe(&mut self, hypothesis: &[Token]) -> Committed;
  }
  ```
- `src/stream/session.rs:8-13,49-82` — `StreamSession` holds a concrete `Transcriber`; `decode_and_advance`
  ignores its `_final_pass` flag and always calls `observe`; committed-segment timings use the WHOLE token
  list, not the committed slice:
  ```rust
  pub struct StreamSession { transcriber: Transcriber, policy: Box<dyn StreamPolicy + Send>, opts: AsrOptions, buffer: Vec<f32>, dirty: bool }
  fn decode_and_advance(&mut self, _final_pass: bool) -> Vec<StreamEvent> {
      let segments = match self.transcriber.transcribe(&self.buffer, &self.opts) { Ok(s)=>s, Err(e)=>return vec![StreamEvent::Error(e.to_string())] };
      // ...builds tokens...
      let committed = self.policy.observe(&tokens);               // <-- ignores _final_pass
      // ...
      start: tokens.first()..., end: tokens.last()...,            // <-- whole buffer, not committed slice
  }
  ```
- `src/stream/two_pass.rs:23-51` — `observe` returns empty text; only `observe_final` commits.
- `src/stream/local_agreement.rs` — `LocalAgreement2::observe` commits a stable prefix and tracks
  `committed_upto`. Read this file before Step 2; you need its internal cursor field name.
- `src/asr/mod.rs:19-45` — `Transcriber::transcribe(&mut self, pcm: &[f32], opts: &AsrOptions) -> Result<Vec<Segment>>`.
- Convention: `unsafe` ONLY in `src/ffi/`; no `panic!`/`unwrap`/`expect` in library code (tests may unwrap);
  one crate-wide `WhisperError` (`src/error.rs`). Streaming code is behind `#[cfg(feature = "streaming")]`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build (streaming) | `cargo build --features streaming` | exit 0 |
| Build (no features) | `cargo build --no-default-features` | exit 0 |
| Unit tests | `cargo test --features streaming` | all pass |
| Lint | `cargo clippy --all-targets --features streaming -- -D warnings` | exit 0, no warnings |
| Model-gated e2e | `cargo test --features streaming --test stream_session -- --ignored` | pass (needs `models/ggml-tiny.en.bin`) |

## Scope
**In scope:**
- `src/stream/mod.rs` (extend the `StreamPolicy` trait; add a `Transcribe` trait)
- `src/stream/local_agreement.rs`, `src/stream/two_pass.rs` (implement the new trait method)
- `src/stream/session.rs` (make generic over `Transcribe`; fix finalize + timings)
- `src/asr/mod.rs` (impl the `Transcribe` trait for `Transcriber`)
- `src/pipeline.rs` (`into_stream` — keep compiling)
- `tests/stream_session.rs` (add offline unit tests with a fake transcriber)

**Out of scope (do NOT touch):**
- `src/ffi/**` — no FFI changes are needed.
- The VAD-boundary incremental-decoding perf work (BACKLOG P3) — keep the whole-buffer re-decode.
- `PartialText` semantics — leave partial emission as the full current hypothesis.

## Git workflow
- Branch: `advisor/001-streaming-finalize` (repo uses `feat/…` / `advisor/…`; conventional-commit messages,
  e.g. `fix(stream): flush tail on finalize; make session testable`). Commit per step. Do NOT push or open a PR.

## Steps

### Step 1: Add finality to the `StreamPolicy` trait
In `src/stream/mod.rs`, add a defaulted method to `StreamPolicy`:
```rust
pub trait StreamPolicy {
    fn observe(&mut self, hypothesis: &[Token]) -> Committed;
    /// Final signal (end of stream). Commit everything not yet committed.
    /// Default: same as `observe` (correct for prefix policies like LocalAgreement-2 IF they also
    /// flush their tail — see below).
    fn observe_final(&mut self, hypothesis: &[Token]) -> Committed { self.observe(hypothesis) }
}
```
Then override it where needed:
- `TwoPass` (`two_pass.rs`): it already has an inherent `observe_final`; change that method to be the trait
  method (`fn observe_final(&mut self, hypothesis: &[Token]) -> Committed` inside `impl StreamPolicy for TwoPass`)
  and remove the inherent one (or keep both by having the trait method call the existing logic).
- `LocalAgreement2` (`local_agreement.rs`): override `observe_final` to commit the tail — everything from its
  `committed_upto` cursor to `hypothesis.len()`, then advance the cursor. Read the file first to reuse its
  existing "join tokens `a..b`" logic and its cursor field name.

**Verify**: `cargo build --features streaming` → exit 0.

### Step 2: Route `finalize()` through `observe_final`
In `src/stream/session.rs`, change `decode_and_advance` to honor the flag:
```rust
let committed = if final_pass { self.policy.observe_final(&tokens) } else { self.policy.observe(&tokens) };
```
Rename the parameter `_final_pass` → `final_pass` (it's now used).

**Verify**: `cargo build --features streaming` → exit 0.

### Step 3: Fix committed-segment timestamps
`Committed` (`stream/mod.rs`) carries `committed_upto` but not the START index of this commit. Add a field so
the session can time the committed slice. In `stream/mod.rs` extend `Committed`:
```rust
pub struct Committed { pub text: String, pub committed_upto: usize, pub committed_from: usize }
```
Update both policies to set `committed_from` = the index the commit started at (for LocalAgreement2, the old
cursor value before advancing; for TwoPass, likewise). In `session.rs`, time the event from that slice:
```rust
let (s, e) = if committed.committed_upto > committed.committed_from {
    (tokens[committed.committed_from].start, tokens[committed.committed_upto - 1].end)
} else { (0.0, 0.0) };
events.push(StreamEvent::CommittedSegment { text: committed.text.trim().to_string(), start: s, end: e });
```
Guard indices against `tokens.len()` (a policy could report a range past the current token list on a shrink —
clamp to `tokens.len()`). Update the existing `Committed { .. }` literals and `..Default::default()` sites.

**Verify**: `cargo build --features streaming` and `cargo test --features streaming` → exit 0.

### Step 4: Introduce a `Transcribe` trait seam
In `src/stream/mod.rs` add:
```rust
/// Minimal transcription capability a StreamSession needs — lets tests inject a fake (no model).
pub trait Transcribe {
    fn transcribe(&mut self, pcm: &[f32], opts: &crate::asr::AsrOptions) -> crate::error::Result<Vec<crate::output::Segment>>;
}
```
In `src/asr/mod.rs`, implement it for the real type:
```rust
impl crate::stream::Transcribe for Transcriber {
    fn transcribe(&mut self, pcm: &[f32], opts: &AsrOptions) -> Result<Vec<crate::output::Segment>> {
        Transcriber::transcribe(self, pcm, opts)   // call the inherent method
    }
}
```
Make `StreamSession` generic over it:
```rust
pub struct StreamSession<T: Transcribe = Transcriber> { transcriber: T, policy: Box<dyn StreamPolicy + Send>, opts: AsrOptions, buffer: Vec<f32>, dirty: bool }
impl<T: Transcribe> StreamSession<T> { /* new/push/poll/finalize/reset/decode_and_advance unchanged bodies */ }
```
(The default type param `= Transcriber` keeps `Pipeline::into_stream` and existing call sites compiling
without changes. Confirm `into_stream` still builds.)

**Verify**: `cargo build --features streaming` and `cargo build --no-default-features` → both exit 0.

### Step 5: Add offline unit tests with a fake transcriber
In `tests/stream_session.rs` (keep the existing `#[ignore]`d model test), add non-ignored tests using a
fake `Transcribe` impl that returns scripted segments. Cover:
- `reset()` clears the buffer (push, reset, poll → empty).
- `poll()` on an empty buffer returns `[]`.
- A fake returning a growing hypothesis across polls: with `LocalAgreement2`, assert a `CommittedSegment` is
  emitted once two polls agree, and that `finalize()` emits the trailing word as a `CommittedSegment`.
- With `TwoPass`: assert `poll()` emits only `PartialText`, and `finalize()` emits a `CommittedSegment`
  containing the full text (this is the regression test for the "TwoPass never commits" bug).
- Committed-segment timings equal the committed slice's first/last token times (regression for Step 3).

Fake pattern (put it in the test file):
```rust
struct FakeTranscriber { scripts: Vec<Vec<whisper_rs::output::Segment>>, i: usize }
impl whisper_rs::stream::Transcribe for FakeTranscriber {
    fn transcribe(&mut self, _pcm: &[f32], _opts: &whisper_rs::asr::AsrOptions) -> whisper_rs::Result<Vec<whisper_rs::output::Segment>> {
        let out = self.scripts.get(self.i).cloned().unwrap_or_default(); self.i += 1; Ok(out)
    }
}
```
Construct via `StreamSession::new(fake, Box::new(LocalAgreement2::new()), AsrOptions::default())`.

**Verify**: `cargo test --features streaming --test stream_session` → all non-ignored tests pass (≥5 new).

### Step 6: Confirm the model-gated e2e still passes
**Verify**: `cargo test --features streaming --test stream_session -- --ignored` → the `streams_jfk_clip_in_chunks`
test passes (needs `models/ggml-tiny.en.bin` + `tests/fixtures/jfk.wav`; both are present in this repo). If the
model is absent, SKIP this step and note it — the offline tests from Step 5 are the gate.

## Test plan
New tests in `tests/stream_session.rs` (model after the existing test's structure). Cases listed in Step 5 —
the two named regressions are "finalize flushes the LocalAgreement2 tail" and "TwoPass commits on finalize".
Verification: `cargo test --features streaming` → all pass including the new tests.

## Done criteria
- [ ] `cargo build --features streaming` and `cargo build --no-default-features` exit 0
- [ ] `cargo clippy --all-targets --features streaming -- -D warnings` exit 0
- [ ] `cargo test --features streaming` passes; ≥5 new non-ignored `stream_session` tests exist and pass
- [ ] `grep -n "_final_pass" src/stream/session.rs` returns nothing (param is now used)
- [ ] No files outside the in-scope list modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions
- The "Current state" excerpts don't match the live code (drift) — report instead of guessing.
- Making `StreamSession` generic forces changes in files outside the in-scope list (e.g. a second call site of
  `StreamSession::new` with an inference that breaks) — report the call sites.
- Adding `committed_from` breaks the existing pure policy tests in `tests/stream_local_agreement.rs` /
  `tests/stream_two_pass.rs` in a way that isn't a trivial field addition — report; do NOT weaken those tests.

## Maintenance notes
- When VAD-boundary incremental decoding lands (BACKLOG P3), `decode_and_advance` changes but the
  `observe`/`observe_final` split and the `Transcribe` seam stay.
- A reviewer should confirm `observe_final` on `LocalAgreement2` never re-commits already-committed tokens
  (idempotence) and that `committed_from`/`committed_upto` indices are clamped to `tokens.len()`.
- Deferred out of scope: worker-thread session + cpal mic (separate hardware-gated work).

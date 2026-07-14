# whisper-rs v3 Streaming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Real-time / streaming transcription behind the `streaming` feature — a `StreamPolicy` trait with LocalAgreement-2 and two-pass strategies, a VAD-boundary-driven streaming session on a worker thread, and optional mic capture — emitting incremental events.

**Architecture:** The **commit policy** is the pure, testable core and is built first: a `StreamPolicy` decides when tentative hypotheses become committed text. The streaming **session** (Handy's proven shape: a worker thread fed 16 kHz frames over a channel, holding a whisper `Transcriber`, emitting `StreamEvent`s) integrates the policy with whisper — model-gated. **cpal** mic capture is an optional source. Wired into `Pipeline::stream(policy)` last.

**Tech Stack:** `tokio` (worker task + channels) or `std::sync::mpsc` + a thread; `cpal` (mic, optional). Reuses the foundation's `Transcriber` + `audio` resampling. Reference design: cjpais/Handy `StreamRouter`/`Stream` (BACKLOG P6).

## Global Constraints
- Everything gated under `#[cfg(feature = "streaming")]`; crate MUST still build `--no-default-features`.
- `unsafe` ONLY in `src/ffi/` — none in streaming.
- One crate-wide `WhisperError`; transient inference errors surface as a `StreamEvent::Error`, never a task-killing panic (spec error-handling rule).
- Streaming input is 16 kHz mono f32 frames (reuse `audio` resampling for non-16 kHz sources).
- Chunking is **VAD/agreement-boundary driven, not fixed windows** (spec decision #4).

---

### Task 1: `streaming` feature scaffolding + event/policy types

**Files:** `Cargo.toml`, `src/lib.rs`; create `src/stream/mod.rs`. Test: `tests/stream_types.rs`.

**Interfaces produced:**
- `stream::StreamEvent` enum: `PartialText(String)`, `CommittedSegment { text: String, start: f32, end: f32 }`, `SpeakerTurn(...)` (reserved; only if `diarization` also on), `Error(String)`.
- `stream::StreamPolicy` trait: `fn observe(&mut self, hypothesis: &[Token]) -> Committed` where `Token { text: String, start: f32, end: f32 }` and `Committed { text: String, committed_upto: usize }` — the policy sees successive full-hypothesis token lists and returns the newly-committable prefix.

- [ ] **Step 1: Cargo** — `streaming = ["dep:cpal"]` (cpal optional; tokio only if you choose async — std thread + mpsc is acceptable and lighter). Add `cpal = { version = "0.15", optional = true }`.
- [ ] **Step 2: failing test** `tests/stream_types.rs` (`#![cfg(feature="streaming")]`) constructing a `StreamEvent` and a `Token`, asserting field access.
- [ ] **Step 3:** implement the enum + `Token`/`Committed` structs + the `StreamPolicy` trait in `src/stream/mod.rs`; `#[cfg(feature="streaming")] pub mod stream;` in lib.rs.
- [ ] **Step 4:** `cargo test --features streaming --test stream_types` PASS; `cargo build --no-default-features` PASS.
- [ ] **Step 5: Commit** `feat(stream): feature scaffolding + StreamEvent/StreamPolicy types`.

---

### Task 2: LocalAgreement-2 policy (PURE — the testable core)

**Files:** create `src/stream/local_agreement.rs`; modify `src/stream/mod.rs`. Test: `tests/stream_local_agreement.rs`.

**Interfaces produced:** `stream::LocalAgreement2` implementing `StreamPolicy` — commits the longest common token prefix that has been stable across the last two hypotheses (beyond what's already committed).

- [ ] **Step 1: failing test** with synthetic hypothesis sequences:
```rust
#![cfg(feature = "streaming")]
use whisper_rs::stream::{LocalAgreement2, StreamPolicy, Token};

fn toks(words: &[&str]) -> Vec<Token> {
    words.iter().enumerate().map(|(i,w)| Token{ text: w.to_string(), start: i as f32, end: i as f32 + 1.0 }).collect()
}

#[test]
fn commits_prefix_stable_across_two_hypotheses() {
    let mut p = LocalAgreement2::new();
    // first hypothesis: nothing committed yet (need 2 to agree)
    assert_eq!(p.observe(&toks(&["the","quick"])).text, "");
    // second agrees on "the quick" -> commit it
    let c = p.observe(&toks(&["the","quick","brown"]));
    assert_eq!(c.text.trim(), "the quick");
    // third: "the quick brown" stable with prior's "brown" -> commit "brown"
    let c2 = p.observe(&toks(&["the","quick","brown","fox"]));
    assert_eq!(c2.text.trim(), "brown");
}

#[test]
fn revision_does_not_recommit() {
    let mut p = LocalAgreement2::new();
    p.observe(&toks(&["hello","wrld"]));
    p.observe(&toks(&["hello","world"]));  // "hello" stable -> committed; "wrld"->"world" revised
    let c = p.observe(&toks(&["hello","world","now"]));
    assert!(!c.text.contains("hello"));    // never re-commit already-committed text
}
```
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: implement** — keep the previous hypothesis + a `committed_upto` cursor; on each `observe`, compute the common prefix length between the current and previous hypotheses, commit tokens in `(committed_upto .. common_prefix)`, advance the cursor. Never re-commit.
- [ ] **Step 4:** `cargo test --features streaming --test stream_local_agreement` PASS.
- [ ] **Step 5: Commit** `feat(stream): LocalAgreement-2 commit policy (pure, tested)`.

---

### Task 3: Two-pass policy (PURE)

**Files:** create `src/stream/two_pass.rs`. Test: `tests/stream_two_pass.rs`.

**Interfaces produced:** `stream::TwoPass` implementing `StreamPolicy` — emits fast tentative text immediately and commits on a periodic/boundary "final" observation. Model the policy purely: it distinguishes tentative vs final `observe` calls via a `fn observe_final(&mut self, ...)` or a flag; commit only on final. (The dual-model runtime is Task 4's concern; the policy logic is pure and testable here.)

- [ ] **Step 1: failing test** — tentative observations produce `PartialText`-style empty commits; a final observation commits the accumulated stable text. (Write concrete synthetic assertions.)
- [ ] **Step 2–4:** implement + pass.
- [ ] **Step 5: Commit** `feat(stream): two-pass commit policy (pure, tested)`.

---

### Task 4: Streaming session (worker thread + VAD-boundary chunking) — MODEL-GATED

**Files:** create `src/stream/session.rs`; modify `src/stream/mod.rs`. Test: `tests/stream_session.rs` (`#[ignore]`, model-gated).

**Interfaces produced:** `stream::StreamSession` with `push(&mut self, frames: &[f32])`, `poll(&mut self) -> Vec<StreamEvent>`, `finalize(&mut self) -> Vec<StreamEvent>`, `reset(&mut self)`. Owns a `Transcriber` on a worker thread fed via a channel; runs the whisper pass on accumulated audio at agreement/VAD boundaries; runs the configured `StreamPolicy` over each hypothesis; emits `PartialText` for tentative and `CommittedSegment` for committed text; inference errors → `StreamEvent::Error` (no panic — wrap the worker in the panic-safe pattern from Handy P6).

- [ ] Steps: `#[ignore]` integration test feeding a known multi-word clip in chunks and asserting the concatenated committed text ≈ the batch transcription (fuzzy). Implement the worker (std thread + mpsc, or tokio task), boundary detection (reuse a simple energy/VAD gate or fixed-overlap re-decode windows), and event emission. Build must stay green for default + `--no-default-features`. Commit `feat(stream): worker-thread streaming session (model-gated)`.

---

### Task 5: cpal mic capture source (optional) — HARDWARE-GATED

**Files:** create `src/stream/mic.rs`. Test: `#[ignore]` (needs an input device).

**Interfaces produced:** `stream::MicCapture` that opens the default input device (cpal), negotiates format (F32>I16>I32 per Handy P6), resamples device-rate→16 kHz via `audio`'s resampler in ~30 ms frames, and pushes frames into a `StreamSession`. All `#[ignore]` for CI (no audio device).

- [ ] Steps: implement per Handy's recorder pattern (device config caching optional); tests `#[ignore]`. Commit `feat(stream): cpal mic capture source (hardware-gated)`.

---

### Task 6: Wire `Pipeline::stream(policy)`

**Files:** modify `src/pipeline.rs`. Test: extend `tests/pipeline.rs` (`#[cfg(feature="streaming")]`, `#[ignore]`).

**Interfaces produced:** `Pipeline::stream(&mut self, policy: impl StreamPolicy + Send + 'static) -> StreamSession` — constructs a session bound to the pipeline's model + options.

- [ ] Steps: implement the constructor; `#[ignore]` e2e test that streams the JFK clip in chunks and asserts committed text contains "country". Default + `--no-default-features` builds stay green. Commit `feat(pipeline): Pipeline::stream(policy) entry point`.

---

## Self-Review
- Spec decision #4 (StreamPolicy trait, both LocalAgreement-2 + two-pass, both configurable, VAD-boundary chunking): Tasks 2, 3 (policies), Task 4 (boundary-driven session). ✓
- Model-independent testable core ships first: feature + types (T1), LocalAgreement-2 (T2), two-pass (T3) — all pure, no model. ✓
- Model/hardware-gated work (T4 session, T5 mic, T6 e2e) is `#[ignore]`d and clearly depends on a whisper model / audio device — do not fake. ✓
- `--no-default-features` builds at every task. `unsafe` stays in `ffi`. Errors as `StreamEvent::Error`, no panics. ✓
- No placeholders: policy commit semantics are specified with concrete synthetic test assertions; the model-gated tasks' assertions are described, to be made concrete by the implementer against a real model.

## Execution note
Tasks 1–3 (feature, types, both pure policies) are executable and fully testable now. Tasks 4–6 need a whisper model (reuse `models/ggml-tiny.en.bin` from the foundation) and, for Task 5, an audio input device — their tests stay `#[ignore]`d until those are present.

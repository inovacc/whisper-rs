# whisper-rs v4 Preprocessing & Robustness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Complete the remaining Phase 4 scope — tiered audio preprocessing (levels 0–4), voice-activity detection (VAD), and hallucination flagging — and wire preprocessing into the `Pipeline`. (The post-processing text transforms and the whisper model downloader already shipped in earlier runs.)

**Architecture:** Preprocessing is a pure-DSP stage over 16 kHz mono f32 PCM (Galle's 0–4 tiers). VAD ships in two forms: a **pure energy/zero-crossing VAD** (default, testable without a model) and an optional **Silero ONNX VAD** (model-gated, like diarization). Hallucination flagging compares two decode passes (different params/models) and flags divergent segments — model-gated. Preprocessing plugs into `audio`/`Pipeline` before ASR.

**Tech Stack:** Pure Rust DSP (no new deps for levels + energy VAD); reuses `rubato`. Silero VAD would reuse `ort` (shared with diarization, deferred). Builds on `audio`, `output::SegmentFlags`, `asr`.

## Global Constraints
- `unsafe` ONLY in `src/ffi/`. New modules are unsafe-free.
- Pure preprocessing + energy VAD need NO new deps and are always available; Silero VAD + hallucination flagging are model-gated (`#[ignore]`d tests). Crate MUST build `--no-default-features`.
- One crate-wide `WhisperError`; no library panics.
- Preprocessing operates on 16 kHz mono f32 PCM (post-resample); output stays in `[-1, 1]`.

---

### Task 1: Tiered preprocessing levels 0–4 (PURE DSP)

**Files:** create `src/audio/preprocess.rs`; modify `src/audio/mod.rs`. Test: `tests/audio_preprocess.rs`.

**Interfaces produced:** `audio::PreprocessLevel` (`L0`..`L4`) and `audio::preprocess(pcm: &mut [f32], level: PreprocessLevel)` (or returning `Vec<f32>`), applying (Galle scheme): L0 none · L1 peak-normalize · L2 +DC-offset removal / high-pass · L3 +simple noise gate · L4 +stronger gate/compression.

- [ ] **Step 1: failing test** — assert L0 is identity; L1 normalizes peak to ~1.0; a below-threshold noise sample is attenuated at L3; all outputs stay within `[-1,1]`. (Concrete synthetic signals; no model.)
- [ ] **Step 2–4:** implement each tier as a small, composable function; higher levels compose lower ones. Pass tests.
- [ ] **Step 5: Commit** `feat(audio): tiered preprocessing levels 0-4 (pure DSP)`.

---

### Task 2: Energy-based VAD (PURE — default)

**Files:** create `src/audio/vad.rs`; modify `src/audio/mod.rs`. Test: `tests/audio_vad.rs`.

**Interfaces produced:** `audio::vad::segment(pcm: &[f32], sample_rate: u32, cfg: VadConfig) -> Vec<(f32, f32)>` — returns speech `(start_s, end_s)` spans via short-frame energy + zero-crossing thresholds with hangover smoothing. `VadConfig { frame_ms, energy_threshold, min_speech_ms, hangover_ms }` with `Default`.

- [ ] **Step 1: failing test** — a signal that is silence → tone → silence yields one span roughly covering the tone; pure silence yields no spans. (Synthetic; no model.)
- [ ] **Step 2–4:** implement frame energy gate + min-duration + hangover merge. Pass tests.
- [ ] **Step 5: Commit** `feat(audio): energy-based VAD segmentation (pure)`.

---

### Task 3: Wire preprocessing + VAD into the Pipeline

**Files:** modify `src/pipeline.rs`, `src/audio/mod.rs`. Test: extend `tests/pipeline.rs` (a non-model construction test + an `#[ignore]`d e2e).

**Interfaces produced:** `PipelineBuilder::preprocess(PreprocessLevel) -> Self` (default L0) and optional `.vad(VadConfig)`; `transcribe_file` applies preprocessing to the PCM before ASR, and (if VAD enabled) can gate/segment. Non-diarization, non-streaming path stays unchanged when neither is set.

- [ ] **Step 1: failing test** — builder `.preprocess(L2)` chains and compiles; an `#[ignore]`d e2e confirms transcription still works with L2 applied.
- [ ] **Step 2–4:** thread the level/vad config through `Pipeline`; apply before ASR. Default L0 = current behavior. Pass tests; `--no-default-features` builds.
- [ ] **Step 5: Commit** `feat(pipeline): preprocess level + optional VAD in transcribe_file`.

---

### Task 4: Hallucination flagging (MODEL-GATED)

**Files:** create `src/postprocess/hallucination.rs`; modify `src/postprocess/mod.rs`, `src/pipeline.rs`. Test: `tests/hallucination.rs` (a PURE heuristic test + an `#[ignore]`d model test).

**Interfaces produced:** `postprocess::flag_hallucinations(primary: &[Segment], secondary: &[Segment]) -> Vec<bool>` (PURE — compares two decode passes' segment texts by similarity; low agreement ⇒ suspect) and wiring that sets `SegmentFlags.hallucination_suspect`. A `PostConfig.flag_hallucinations` toggle runs a second decode pass (different temperature/params) and flags divergent segments.

- [ ] **Step 1: PURE test** — two near-identical segment sets ⇒ no flags; a segment present in `primary` with no similar `secondary` counterpart ⇒ flagged. (Synthetic segments; no model.)
- [ ] **Step 2: implement the pure comparison** (token-overlap / normalized edit distance threshold).
- [ ] **Step 3: model-gated wiring** — when enabled, run a second ASR pass (e.g. higher temperature) and apply the flags; `#[ignore]`d integration test (needs a model).
- [ ] **Step 4:** pure test passes; `--no-default-features` builds. **Step 5: Commit** `feat(postprocess): hallucination flagging via cross-pass disagreement`.

---

## Self-Review
- Spec decision #5 (hallucination flagging in v1): Task 4 (pure comparison now; model-gated second-pass wiring). Number normalization already shipped. ✓
- Spec preprocessing (levels 0–4, VAD): Tasks 1–3, energy VAD as the model-free default, Silero VAD deferred with diarization's `ort`. ✓
- Model-independent value first: preprocessing levels (T1), energy VAD (T2), pipeline wiring (T3), hallucination heuristic (T4 pure part) — all testable now. Silero VAD + second-pass hallucination are model-gated. ✓
- `--no-default-features` builds every task; `unsafe` stays in `ffi`. ✓

## Execution note
Tasks 1, 2, 3, and Task 4's pure heuristic are executable and testable now. The Silero ONNX VAD upgrade and Task 4's second-decode-pass wiring need a whisper/VAD model (reuse `models/ggml-tiny.en.bin`); those tests stay `#[ignore]`d. The `ort`-based Silero VAD shares the diarization `ort` integration — schedule it alongside Phase 2's ONNX work.

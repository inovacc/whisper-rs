# whisper-rs v2 Diarization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add speaker diarization ("who spoke when") behind the `diarization` feature — ONNX pyannote segmentation + speaker embeddings + clustering — and merge speaker turns into the `Transcript` produced by the foundation.

**Architecture:** A new `src/diarize/` module (all in the safe layer; any ONNX FFI stays inside `ort`). The pure, model-independent **timeline-merge** (`merge(words, turns)`) is the testable core and is built first. ONNX inference (segmentation + embedding) is integrated via the `ort` crate (leaning on the `pyannote-rs` precedent), model-gated behind `#[ignore]`d tests. Diarization is wired into the high-level `Pipeline` last.

**Tech Stack:** `ort` (ONNX Runtime, pre-release rc — pinned), `ndarray` (tensor shaping), models `pyannote-segmentation-3.0` + a speaker-embedding model (both ONNX). Builds on the existing `output::{Segment, Word, SpeakerId}` types.

## Global Constraints
- Everything gated under `#[cfg(feature = "diarization")]`; the crate MUST still build `--no-default-features`.
- `unsafe` stays ONLY in `src/ffi/` — the `diarize` module uses `ort`'s safe API, no `unsafe`.
- One crate-wide `WhisperError`; add `Onnx(String)` + reuse `ModelNotFound { kind: DiarizeSegmentation | DiarizeEmbedding, .. }` (variants already exist in `error.rs`). No panics on expected-failure paths.
- **Models are consumer-supplied by path** (matches spec decision #6 — the downloader is a separate Phase 4). Do NOT bundle or auto-download models here.
- Audio into the diarizer is the same 16 kHz mono f32 PCM as ASR.
- `ort` is a pre-release rc — pin the exact version and note it; the onnxruntime shared lib must be resolvable at build/run (document the `ORT_*`/download-binaries strategy the chosen `ort` version uses).

## ⚠️ External prerequisite (read before starting)
`pyannote-segmentation-3.0` (and most speaker-embedding ONNX models) are **HuggingFace-gated**: a human must accept the model license and download the `.onnx` files. This plan CANNOT be fully green without those files present locally. Tasks 1–2 and 5's wiring are model-independent and fully testable now; Tasks 3–4 and the end-to-end test are `#[ignore]`d and only pass once a maintainer places the models at the configured paths. Treat "models acquired" as a checklist item, not an automated step.

---

### Task 1: `diarization` feature scaffolding + types

**Files:**
- Modify: `Cargo.toml` (feature deps), `src/lib.rs` (cfg module)
- Create: `src/diarize/mod.rs`
- Test: `tests/diarize_types.rs`

**Interfaces produced:**
- `diarize::{SpeakerTurn, DiarizeConfig, Diarizer}` (Diarizer body stubbed in Task 3).
- `SpeakerTurn { speaker: SpeakerId, start: f32, end: f32 }`.
- `DiarizeConfig { segmentation_model: PathBuf, embedding_model: PathBuf, max_speakers: Option<usize> }` with a `builder`/constructor.

- [ ] **Step 1: Cargo feature deps.** Under `[dependencies]` add (optional, feature-gated):
```toml
ort = { version = "=2.0.0-rc.10", optional = true }   # verify latest rc at impl time; pin exactly
ndarray = { version = "0.16", optional = true }
```
and update: `diarization = ["dep:ort", "dep:ndarray"]`.

- [ ] **Step 2: failing test `tests/diarize_types.rs`** (only compiled with the feature):
```rust
#![cfg(feature = "diarization")]
use whisper_rs::diarize::{DiarizeConfig, SpeakerTurn};
use whisper_rs::output::SpeakerId;

#[test]
fn speaker_turn_and_config_construct() {
    let t = SpeakerTurn { speaker: SpeakerId(0), start: 0.0, end: 1.5 };
    assert!(t.end > t.start);
    let cfg = DiarizeConfig::new("seg.onnx", "emb.onnx");
    assert_eq!(cfg.max_speakers, None);
}
```

- [ ] **Step 3: `src/diarize/mod.rs`** — `SpeakerTurn`, `DiarizeConfig` (+ `new(seg, emb)` and `max_speakers` setter), and a `Diarizer` struct with a `new(&DiarizeConfig) -> Result<Diarizer>` that for now returns `Ok` without loading (loading added in Task 3). Add `#[cfg(feature = "diarization")] pub mod diarize;` to `src/lib.rs`.

- [ ] **Step 4:** `cargo test --features diarization --test diarize_types` PASS; `cargo build --no-default-features` PASS.
- [ ] **Step 5: Commit** `feat(diarize): feature scaffolding + SpeakerTurn/DiarizeConfig types`.

---

### Task 2: Timeline merge (PURE — the testable core)

**Files:**
- Create: `src/diarize/merge.rs`; Modify: `src/diarize/mod.rs`
- Test: `tests/diarize_merge.rs`

**Interfaces produced:**
- `diarize::merge::assign_speakers(segments: Vec<Segment>, turns: &[SpeakerTurn]) -> Vec<Segment>` —
  assigns each segment (and each of its words) the `SpeakerId` of the turn with maximum temporal overlap;
  `None` if no turn overlaps. Pure function, no model.

- [ ] **Step 1: failing test `tests/diarize_merge.rs`**:
```rust
#![cfg(feature = "diarization")]
use whisper_rs::diarize::{merge::assign_speakers, SpeakerTurn};
use whisper_rs::output::{Segment, SegmentFlags, SpeakerId, Word};

fn seg(text: &str, s: f32, e: f32) -> Segment {
    Segment { speaker: None, text: text.into(), start: s, end: e,
              words: vec![Word{text:text.into(),start:s,end:e,confidence:1.0}], flags: SegmentFlags::default() }
}

#[test]
fn assigns_speaker_by_max_overlap() {
    let turns = vec![
        SpeakerTurn { speaker: SpeakerId(0), start: 0.0, end: 2.0 },
        SpeakerTurn { speaker: SpeakerId(1), start: 2.0, end: 4.0 },
    ];
    let out = assign_speakers(vec![seg("a", 0.1, 1.8), seg("b", 2.2, 3.9)], &turns);
    assert_eq!(out[0].speaker, Some(SpeakerId(0)));
    assert_eq!(out[1].speaker, Some(SpeakerId(1)));
    assert_eq!(out[0].words[0].? , /* words inherit segment speaker if you choose to propagate; keep simple: segment-level */ out[0].words[0].text.len().min(1)*1); // placeholder-free: assert words unchanged length
}

#[test]
fn no_overlap_leaves_speaker_none() {
    let turns = vec![SpeakerTurn { speaker: SpeakerId(0), start: 10.0, end: 11.0 }];
    let out = assign_speakers(vec![seg("x", 0.0, 1.0)], &turns);
    assert_eq!(out[0].speaker, None);
}
```
> Implementer note: drop the placeholder assertion line — assert only segment-level speaker assignment
> and that segment count/text are preserved. (Word-level speaker propagation is out of scope; words carry
> timing, the segment carries the speaker.)

- [ ] **Step 2:** run → FAIL (module missing).
- [ ] **Step 3: implement `assign_speakers`** — for each segment compute overlap `max(0, min(seg.end,turn.end) - max(seg.start,turn.start))` against every turn, pick the argmax turn (ties → earliest), set `segment.speaker`. Preserve everything else.
- [ ] **Step 4:** `cargo test --features diarization --test diarize_merge` PASS.
- [ ] **Step 5: Commit** `feat(diarize): pure timeline-merge assign_speakers + tests`.

---

### Task 3: ONNX segmentation inference → speaker turns (MODEL-GATED)

**Files:** Create `src/diarize/segmentation.rs`; Modify `src/diarize/mod.rs`, `src/error.rs` (add `Onnx(String)` if absent). Test: `tests/diarize_infer.rs` (`#[ignore]`).

**Interfaces produced:** `Diarizer::diarize(&mut self, pcm: &[f32]) -> Result<Vec<SpeakerTurn>>`.

- [ ] **Step 1:** add `#[error("onnx error: {0}")] Onnx(String)` to `WhisperError` (behind nothing — a plain variant; keep the enum non-feature-gated).
- [ ] **Step 2: `#[ignore]`d integration test `tests/diarize_infer.rs`** requiring the gated models at `models/pyannote-segmentation-3.0.onnx` (+ embedding model): builds a `Diarizer`, runs `diarize` on `tests/fixtures/two_speakers.wav`, asserts ≥2 distinct `SpeakerId`s and turns are ordered/non-negative-length. Reason string documents the model requirement.
- [ ] **Step 3: implement segmentation** — load the segmentation ONNX with `ort` in `Diarizer::new` (error `ModelNotFound{ kind: DiarizeSegmentation, .. }` if the path is missing). In `diarize`, window the 16 kHz PCM per the model's expected input (pyannote-segmentation-3.0: 10 s windows), run inference, threshold per-frame speaker-activity into contiguous `(start,end)` activity spans. Verify the model's exact input/output tensor shapes against the actual `.onnx` (use `ort`'s introspection); the implementer has latitude to match real shapes. Emit provisional per-window speaker slots (local IDs) — global identity is Task 4.
- [ ] **Step 4:** `cargo test --features diarization` (non-ignored) still PASS; `cargo build --no-default-features` PASS. Run `-- --ignored` only if the maintainer has placed models; otherwise record "model-gated, not run".
- [ ] **Step 5: Commit** `feat(diarize): pyannote ONNX segmentation inference (model-gated)`.

---

### Task 4: Speaker embeddings + clustering → global speaker IDs (MODEL-GATED)

**Files:** Create `src/diarize/embedding.rs`, `src/diarize/cluster.rs`; Modify `src/diarize/mod.rs`. Test: extend `tests/diarize_infer.rs`; add a PURE clustering unit test in `tests/diarize_cluster.rs`.

**Interfaces produced:** internal `cluster::agglomerative(embeddings: &[Vec<f32>], threshold: f32, max: Option<usize>) -> Vec<usize>` (pure, testable) mapping each turn to a global speaker index; `embedding` module extracts an embedding per activity span.

- [ ] **Step 1: PURE clustering test `tests/diarize_cluster.rs`** — feed synthetic embeddings in two clear clusters (e.g. `[1,0,0]`-ish vs `[0,1,0]`-ish), assert the function returns 2 groups and that near-identical vectors share a group. No model.
- [ ] **Step 2: implement `cluster::agglomerative`** — cosine-distance agglomerative clustering with a distance threshold (and optional `max_speakers` cap). Pure over `&[Vec<f32>]`.
- [ ] **Step 3: implement `embedding`** — load the embedding ONNX (`ModelNotFound{ kind: DiarizeEmbedding, .. }` if missing), run it over each segmentation activity span to get a speaker vector; feed vectors to `agglomerative`; relabel `SpeakerTurn`s with the resulting global `SpeakerId`s.
- [ ] **Step 4:** pure cluster test PASS; feature build + `--no-default-features` build PASS; ignored inference test now asserts consistent global IDs when models are present.
- [ ] **Step 5: Commit** `feat(diarize): speaker embeddings + agglomerative clustering (pure cluster tested)`.

---

### Task 5: Wire diarization into the high-level Pipeline

**Files:** Modify `src/pipeline.rs`, `src/diarize/mod.rs`. Test: extend `tests/pipeline.rs` (a `#[cfg(feature="diarization")]` `#[ignore]`d e2e test).

**Interfaces produced:** `PipelineBuilder::diarization(DiarizeConfig) -> Self` (feature-gated); `transcribe_file` runs the diarizer when configured and calls `assign_speakers` before returning the `Transcript`.

- [ ] **Step 1: failing test** (`#[cfg(feature="diarization")]`, `#[ignore]` model-gated): build a pipeline with `.whisper_model(...).diarization(DiarizeConfig::new(seg, emb))`, transcribe a 2-speaker clip, assert the returned `Transcript` has segments with `Some(speaker)` and ≥2 distinct speakers.
- [ ] **Step 2: implement** — add a feature-gated `diarizer: Option<Diarizer>` to `Pipeline`; in `transcribe_file`, after ASR+timestamps, if a diarizer is configured run `diarize(&pcm)` and `assign_speakers(segments, &turns)`. Keep the non-diarization path unchanged.
- [ ] **Step 3:** `cargo test` (default + `--features diarization`, non-ignored) PASS; `--no-default-features` build PASS.
- [ ] **Step 4: Commit** `feat(pipeline): optional diarization stage wired into transcribe_file`.

---

## Self-Review
- Spec decision #3 (ONNX pyannote default): Tasks 3–4. Stereo channel-split fast-path stays deferred (backlog P4) — not in this plan. ✓
- Model-independent value ships first and is fully tested without gated models: feature scaffolding (T1), timeline-merge (T2), clustering (T4 pure part), pipeline wiring (T5 non-model path). ✓
- Model-gated work (T3, T4 embedding, T5 e2e) is `#[ignore]`d and clearly depends on the HF-gated `.onnx` files — flagged in the external-prerequisite note. ✓
- `--no-default-features` must build at every task (constraint) — each task asserts it. ✓
- Placeholder scan: Task 2's test carries one explicitly-marked placeholder line the implementer is told to delete; no other placeholders. Types/signatures consistent across tasks.

## Execution note
Tasks 1, 2, and the pure part of 4 (clustering) + the non-model pipeline wiring are executable and testable now. Tasks 3, 4 (embedding), and the e2e test require a maintainer to accept the HuggingFace licenses and place `pyannote-segmentation-3.0.onnx` + an embedding `.onnx` under `models/`. Until then those tests remain `#[ignore]`d — do not fake them.

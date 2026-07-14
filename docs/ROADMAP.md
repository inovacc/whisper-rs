# Roadmap — whisper-rs
<!-- rev:006 -->

**Project type:** Rust (pre-scaffold) — no `Cargo.toml` exists yet. This roadmap tracks the crate
defined in `docs/superpowers/specs/2026-07-14-whisper-rs-design.md` and built by the plans under
`docs/superpowers/plans/`.

**Status:** Design approved interactively (2026-07-14) via `/discover` → `superpowers:brainstorming`.
Ready to implement, starting with the foundation plan. The crate is **local-use only** (not
published to crates.io); the name collision with the existing `whisper-rs` crate is a non-issue.

**v1 shape:** feature-rich from the start — core ASR, audio preprocessing, word timestamps,
diarization, streaming, and structured output all ship in v1. The phases below are **build order**
(each an independently testable plan), not staged feature releases.

## Phase 0 — Discovery & design (COMPLETE)
- [x] Scan + inventory the planning folder (`docs/discovery/IDEA-BRIEF.md`)
- [x] Deep web research: 13 reference URLs + competitive teardown (tazz4843/whisper-rs, WhisperX,
      whisper-diarization)
- [x] Design spec approved interactively (`docs/superpowers/specs/2026-07-14-whisper-rs-design.md`)
- [x] Foundation implementation plan written
      (`docs/superpowers/plans/2026-07-14-whisper-rs-v1-foundation.md`)
- [x] User sign-off on spec (interactive)

## Phase 1 — Foundation (COMPLETE — branch `feat/v1-foundation`, final review READY TO MERGE)
Plan: `docs/superpowers/plans/2026-07-14-whisper-rs-v1-foundation.md`. Delivers batch transcription
of an audio file → structured, word-timestamped `Transcript`. All 11 tests pass; `clippy` clean;
`unsafe` confined to `src/ffi/`; real JFK-clip transcription verified end-to-end.
- [x] Task 1: Scaffold crate + `git init` (4f6dc65)
- [x] Task 2: Vendor whisper.cpp + `build.rs` (bindgen + cc) + raw FFI bindings + smoke test (1b2ac1d)
- [x] Task 3: Crate-wide `WhisperError` + `ModelKind` (dc8f2bf)
- [x] Task 4: Structured output types (`Transcript`/`Segment`/`Word`) (e59e0f3)
- [x] Task 5: Audio decode + downmix + 16 kHz resample (2aab7f2)
- [x] Task 6: Core ASR over FFI (`Transcriber`, RAII `Context`) (a414081)
- [x] Task 7: token-level word timestamps + monotonic enforcement (ae1b2c9)
- [x] Task 8: Batch `Pipeline` (builder + `transcribe_file`) (6d4fa8c)

## Phase 2 — Diarization (IN PROGRESS — model-independent slice done)
`feature = "diarization"`. Plan: `docs/superpowers/plans/2026-07-14-whisper-rs-v2-diarization.md`.
The strongest market differentiator (no Rust crate ships native diarization ergonomically).
- [x] Types (`SpeakerTurn`, `DiarizeConfig`) + `diarization` feature scaffolding (df7f65c)
- [x] `merge(words, turns)` → `assign_speakers` timeline join (pure, tested) (df7f65c)
- [x] Agglomerative speaker clustering (pure, tested) (df7f65c)
- [ ] **BLOCKED** — `ort` + `pyannote-segmentation-3.0` ONNX segmentation inference (needs HF-gated model)
- [ ] **BLOCKED** — speaker-embedding ONNX inference (needs HF-gated model)
- [ ] Wire `Pipeline::diarization(cfg)` (after inference lands)

> Blocker: `pyannote-segmentation-3.0` + the embedding model are HuggingFace-gated — a maintainer must
> accept the licenses and place the `.onnx` files under `models/`. The plan's model-gated tasks (3–5) and
> their tests stay `#[ignore]`d until then. See `docs/BACKLOG.md` P3.

## Phase 3 — Streaming (IN PROGRESS — pure policy core done)
`feature = "streaming"`. Plan: `docs/superpowers/plans/2026-07-14-whisper-rs-v3-streaming.md`. Fills the
confirmed gap — no tool ships real-time diarized transcription. Reference: Handy `StreamRouter` (BACKLOG P6).
- [x] `StreamPolicy` trait + `LocalAgreement2` + `TwoPass` (pure, tested) (95a887e)
- [x] `StreamSession` (synchronous push/poll/finalize) + `Pipeline::into_stream` — e2e verified (01303ef)
- [ ] VAD-boundary incremental decoding (replace the O(n²) full-buffer re-decode) — perf, BACKLOG P3
- [ ] Worker-thread session variant + `cpal` mic capture source (hardware-gated)

## Phase 4 — Preprocessing, post-processing & model downloader (IN PROGRESS)
Plan: `docs/superpowers/plans/2026-07-14-whisper-rs-v4-preprocessing.md`.
- [x] Number normalization (spoken → digit) — `postprocess::normalize_numbers` (8adcf12)
- [x] Post-processing transforms: repeat-collapse + filler-removal + `PostConfig` wired into `Pipeline` (8adcf12, bc17537)
- [x] `ModelRef::download` + cache behind `feature = "download"` (whisper GGML, public models) (1c908e5)
- [x] Audio preprocessing levels 0–4 (Galle scheme) + energy VAD (pure) (4df4175) + wired into `Pipeline` (4623c9e)
- [x] Hallucination flagging heuristic — pure cross-pass comparison + `apply_flags` (cf19308)
- [ ] Silero ONNX VAD upgrade (model-gated; shares diarization `ort`) — planned
- [ ] Hallucination second-decode-pass wiring (model-gated) — planned

## Test coverage
`cargo llvm-cov` (CI + local). **Baseline (default features): 71.77% line / 70.31% region**, excluding the
4 model-gated `#[ignore]`d tests. Full suite at `--all-features`: ~40 passing + model-gated ignored.

## Deferred to post-v1 (see `docs/BACKLOG.md`)
Stereo channel-split diarization fast-path · DER metrics hooks · multi-mic DOA/TDOA spatial
diarization · pure-Rust Burn reimplementation (whisper-burn) as an FFI alternative.

## Known limitation carried into v1
Overlapping/simultaneous speech is out of scope for v1 (documented) — diarization degrades without
clear speaker pauses. See the design spec.

See `docs/discovery/IDEA-BRIEF.md` for the evidence and citations behind every phase.

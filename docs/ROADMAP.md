# Roadmap — whisper-rs
<!-- rev:002 -->

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

## Phase 1 — Foundation (NOT STARTED, ready)
Plan: `docs/superpowers/plans/2026-07-14-whisper-rs-v1-foundation.md`. Delivers batch transcription
of an audio file → structured, word-timestamped `Transcript`.
- [ ] Task 1: Scaffold crate + `git init`
- [ ] Task 2: Vendor whisper.cpp + `build.rs` (bindgen + cc) + raw FFI bindings + smoke test
- [ ] Task 3: Crate-wide `WhisperError` + `ModelKind`
- [ ] Task 4: Structured output types (`Transcript`/`Segment`/`Word`)
- [ ] Task 5: Audio decode + downmix + 16 kHz resample
- [ ] Task 6: Core ASR over FFI (`Transcriber`, RAII `Context`)
- [ ] Task 7: DTW word timestamps + monotonic enforcement
- [ ] Task 8: Batch `Pipeline` (builder + `transcribe_file`)

## Phase 2 — Diarization (NOT STARTED)
`feature = "diarization"`. Depends on Phase 1. The strongest market differentiator found in research
(no Rust crate ships native diarization ergonomically).
- [ ] `ort` (ONNX Runtime) integration, pinned pre-release rc (tracked risk)
- [ ] `pyannote-segmentation-3.0` + speaker-embedding + clustering → `Diarizer::diarize`
- [ ] `merge(words, turns)` timeline join → speaker-labeled `Transcript`
- [ ] Wire `Pipeline::diarization(cfg)`

## Phase 3 — Streaming (NOT STARTED)
`feature = "streaming"`. Depends on Phase 1. Fills the confirmed gap — no tool ships real-time
diarized transcription.
- [ ] `StreamPolicy` trait + `LocalAgreement2` + `TwoPass` implementations (both configurable)
- [ ] `cpal` mic capture + `tokio` channel glue, VAD-boundary-driven chunking
- [ ] `Pipeline::stream(policy)` emitting `PartialText` / `CommittedSegment` / `SpeakerTurn` / `Error`

## Phase 4 — Preprocessing, post-processing & model downloader (NOT STARTED)
Depends on Phase 1 (post-processing after Phases 2–3 for full effect).
- [ ] Audio preprocessing levels 0–4 (Galle scheme) + Silero VAD segmentation
- [ ] Hallucination flagging (cross-method disagreement) — `SegmentFlags.hallucination_suspect`
- [ ] Number normalization (spoken → digit)
- [ ] `ModelRef::download` + cache behind `feature = "download"`

## Test coverage
**N/A** — no code exists yet, no coverage tool wired in. `cargo llvm-cov` gets wired in Phase 1
(Task 1 lands the crate); see `docs/BACKLOG.md` P2.

## Deferred to post-v1 (see `docs/BACKLOG.md`)
Stereo channel-split diarization fast-path · DER metrics hooks · multi-mic DOA/TDOA spatial
diarization · pure-Rust Burn reimplementation (whisper-burn) as an FFI alternative.

## Known limitation carried into v1
Overlapping/simultaneous speech is out of scope for v1 (documented) — diarization degrades without
clear speaker pauses. See the design spec.

See `docs/discovery/IDEA-BRIEF.md` for the evidence and citations behind every phase.

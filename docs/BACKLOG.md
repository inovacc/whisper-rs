# Backlog — whisper-rs
<!-- rev:005 -->

Grounded in `docs/discovery/IDEA-BRIEF.md`, the approved design spec
(`docs/superpowers/specs/2026-07-14-whisper-rs-design.md`), and the foundation plan
(`docs/superpowers/plans/2026-07-14-whisper-rs-v1-foundation.md`). No code exists yet — items are
scoped as "before/after Phase N" rather than file:line references.

## P1 — Blocking / must resolve before or during Phase 1
- **Pin whisper.cpp submodule to a known tag** and confirm the `build.rs` source-file list matches
  that tag's on-disk layout (the plan pins `v1.7.4`; verify at scaffold time). Effort: S.
- **Acquire a small test-fixture model** (`ggml-tiny.en.bin`) + a known clip (`jfk.wav` ships in
  whisper.cpp `samples/`) for the model-gated (`#[ignore]`) tests in Phases 1–2. Effort: S.

## P1 — Blocker: acquire HF-gated diarization models
- **Accept the HuggingFace licenses + download `pyannote-segmentation-3.0.onnx` and a speaker-embedding
  `.onnx`**, place under `models/`. This is a **human step** (license acceptance) — it blocks the ONNX
  segmentation/embedding tasks of the Phase 2 diarization plan (Tasks 3–5) and un-`#[ignore]`s their
  tests. Also requires wiring `ort` + resolving an onnxruntime binary on the target platform. Effort: M.

## P2 — Near-term
- **Pin `ort`** to the exact pre-release rc + a tracking note when the diarization ONNX tasks land — it
  is pre-1.0, an API-stability risk (design spec). Effort: S.
- **macOS/Windows CI validation** — current CI (`.github/workflows/ci.yml`) builds Linux only. Extend to
  a matrix once the FFI build is confirmed on other OSes. Effort: M.

## P3 — Deferred v1-adjacent features (design-approved, scheduled post-foundation)
These are all part of the feature-rich v1 but land in later build-order plans (Phases 2–4):
- **Diarization (Phase 2)** — `ort` + pyannote-segmentation-3.0 + embeddings + clustering. Strongest
  differentiator. Effort: L.
- **Streaming (Phase 3)** — `StreamPolicy` trait (LocalAgreement-2 + two-pass), `cpal`, `tokio`.
  Effort: L.
- **Preprocessing + post-processing (Phase 4)** — levels 0–4, Silero VAD, hallucination flagging,
  number normalization, `download` feature. Effort: M.

## P4 — Post-v1 (explicitly deferred out of v1 during brainstorming)
- **Stereo channel-split diarization fast-path** — near-zero clustering error for dual-channel /
  call-center audio (Galle pattern). Deferred from the diarization module. Effort: M.
- **DER (Diarization Error Rate) metrics hooks** — eval tooling, not an end-user feature. Effort: M.
- **Multi-mic DOA/TDOA spatial diarization** — hardware-specific, least-validated-in-Rust; the
  heaviest deferred item. Effort: XL / unscoped.
- **Pure-Rust Burn reimplementation** (whisper-burn) as an FFI alternative — revisit only if
  whisper.cpp build friction becomes a real blocker. Effort: XL.

## P5 — Nice-to-haves / competitive parity (not committed)
- **Convenience layer** — non-WAV input decoding (`symphonia` beyond WAV), SRT/VTT output writers.
  Effort: M.
- **Raw-API escape hatch** — expose the `ffi` module (currently `#[doc(hidden)]`) under an opt-in
  feature for consumers who need unwrapped bindings, mirroring tazz4843/whisper-rs. Effort: S.

## P2.5 — Foundation review follow-ups — ✅ RESOLVED 2026-07-14 (see Resolved)
Optional future refinement: wire the real DTW params (`dtw_token_timestamps`/`dtw_aheads`) for
higher-accuracy word times than the current per-token timestamps. Effort: M.

## P6 — Prior-art-derived candidates (Handy / cjpais analysis, 2026-07-14)
Mined from `github.com/cjpais/Handy` (Rust/Tauri whisper dictation app) + its local install. Handy uses
its own `transcribe-cpp`/`transcribe-rs` crates, not `whisper-rs` — these are the capabilities it ships
that whisper-rs lacks. (Handy has **no diarization** — confirming that remains whisper-rs's differentiator.)

**Backends / runtime (asr)**
- **Runtime accelerator selection with fallback** — Auto/CPU/GPU chosen at model-load, GPU→Auto fallback
  if unavailable; Vulkan/CUDA/Metal via a `Backend` enum. whisper-rs picks backend at compile time today.
  The local install ships `ggml-vulkan.dll` + 8 `ggml-cpu-*.dll` microarch variants (dynamic backends). Effort: L.
- **Compute-device enumeration API** (`--list-devices`, name/kind/VRAM) for a device picker. Effort: M.
- **GGUF header capability probing before load** (arch, streaming/translate/lang-detect flags, supported
  languages) — powers a pre-download catalog UI. Effort: M.
- **Idle-timeout auto-unload** of the loaded model (Never/2–15 min/1 h) via a watcher thread — memory mgmt. Effort: M.
- **Panic-safe engine** — `catch_unwind` around inference + drop-and-reload, never poison the mutex. Effort: M.
- **Translate-to-English task** gated on model capability; **language auto-detect** validated against the
  model's advertised language list with graceful "auto" fallback. Effort: M.

**streaming** (reference design for Phase 3)
- **Incremental streaming architecture** — worker thread fed 16 kHz frames over a channel; `feed()`/
  `finalize()`/`reset()` with a **committed vs tentative text split** for flicker-free live captions;
  per-model streaming gating; real-time-factor perf instrumentation. Concrete blueprint for whisper-rs's
  planned `StreamPolicy`. Effort: L.

**audio (capture/preprocess) — Phase 4**
- **cpal mic capture with format negotiation** (F32>I16>I32), per-device config caching (avoids 40–85 ms
  HAL query), device-rate→16 kHz via `rubato::FftFixedIn` in 30 ms frames. Effort: M.
- **Live level/spectrum visualizer** (FFT-bucketed VU meter) for consumers building UIs. Effort: S.
- **System mute during capture** (Windows COM / Linux wpctl / macOS AppleScript). Effort: M (platform-specific).

**models (downloader) — Phase 4**
- **Resumable, cancellable HTTP downloader** — Range support + auto-restart if Range ignored, SHA-256
  verification off-executor, tar.gz extraction, atomic temp-dir. Plus a `ModelSource` enum (URL /
  HuggingFace `hf-hub` shared cache / Local) and a `ModelInfo` registry (accuracy/speed/flags). Strong
  reference for whisper-rs's `download` feature. Effort: L.

**postprocess / output — Phase 4 (extends the planned hallucination + number-normalization set)**
- **Fuzzy custom-vocabulary correction** — n-gram (1–3 word) matching against a user dictionary using
  Levenshtein + Soundex, case-preserving, ampersand expansion ("R and D"→"R&D"); use whisper's
  `initial_prompt` as the primary path when available. Effort: M.
- **Filler-word removal** with per-language lists (16 languages, respecting real-word collisions). Effort: M.
- **Stutter/repetition collapsing** (3+ repeats → 1). Effort: S.
- **Language-intent normalization** (zh-Hans/zh-Hant→zh, BCP-47 base-language matching). Effort: S.

Full evidence + file:line citations: discovery evidence (`...\scratchpad\exec\handy-analysis.md` was not
written — the analyst lacked a Write tool; citations are in the run record / this backlog).

## Resolved
- 2026-07-14 — **Post-processing + streaming core + downloader (`feat/postproc-streaming`).**
  Pure text transforms: number normalization, repeat-collapse, filler-removal (8adcf12) + `PostConfig`
  wired into `Pipeline` (bc17537); pure streaming policy core — `StreamPolicy` + LocalAgreement-2 +
  two-pass (95a887e); whisper GGML model downloader behind `feature = "download"` (1c908e5, public models,
  not HF-gated). Phase 4 plan written; `docs/ISSUES.md` created. All at `--all-features`: 34 passing,
  4 model-gated ignored, clippy clean, `--no-default-features` builds.
- 2026-07-14 — **Foundation polish (`feat/foundation-polish`).** Empty-audio + i32-length guards +
  pure `words_from_tokens` filter test (7c890d0); GitHub Actions CI with feature-matrix + clippy +
  llvm-cov coverage (3b032223); README + canonical AGENTS.md + thin CLAUDE.md (2b3d55e). "DTW" rename
  was a no-op (no DTW string in `src/`; only the immutable commit message).
- 2026-07-14 — **Phase 2 diarization — model-independent slice (df7f65c).** `diarization` feature +
  `SpeakerTurn`/`DiarizeConfig` types, pure `assign_speakers` timeline-merge, pure agglomerative
  clustering — all tested; `--no-default-features` still builds; no `ort` yet. ONNX inference remains
  P1-blocked on HF-gated models.
- 2026-07-14 — **Phase 2 + Phase 3 plans written.** `docs/superpowers/plans/2026-07-14-whisper-rs-v2-diarization.md`
  and `...-v3-streaming.md` (build-order, model-independent cores first).
- 2026-07-14 — **Foundation (Phase 1) built & reviewed.** All 8 tasks implemented TDD, committed on
  `feat/v1-foundation` (4f6dc65..6d4fa8c); final whole-branch review READY TO MERGE (0 Critical/Important).
  Resolves the prior P1 "pin whisper.cpp submodule" (pinned v1.7.4) and "acquire test-fixture model"
  (tiny.en + jfk.wav) items.
- 2026-07-14 — **Design sign-off.** Spec + foundation plan approved interactively (was P1 blocker in
  the prior auto-generated backlog).
- 2026-07-14 — **Crate name decision.** Local-use-only; keep `whisper-rs`; crates.io collision moot.
- 2026-07-14 — **Model bundling / licensing.** Resolved by decision #6: models are consumer-supplied
  by path (default) with an opt-in downloader; the crate never bundles pyannote models, so the
  segmentation-model license is not a redistribution concern.

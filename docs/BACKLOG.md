# Backlog — whisper-rs
<!-- rev:002 -->

Grounded in `docs/discovery/IDEA-BRIEF.md`, the approved design spec
(`docs/superpowers/specs/2026-07-14-whisper-rs-design.md`), and the foundation plan
(`docs/superpowers/plans/2026-07-14-whisper-rs-v1-foundation.md`). No code exists yet — items are
scoped as "before/after Phase N" rather than file:line references.

## P1 — Blocking / must resolve before or during Phase 1
- **Pin whisper.cpp submodule to a known tag** and confirm the `build.rs` source-file list matches
  that tag's on-disk layout (the plan pins `v1.7.4`; verify at scaffold time). Effort: S.
- **Acquire a small test-fixture model** (`ggml-tiny.en.bin`) + a known clip (`jfk.wav` ships in
  whisper.cpp `samples/`) for the model-gated (`#[ignore]`) tests in Phases 1–2. Effort: S.

## P2 — Near-term (Phase 1 execution)
- **Wire `cargo llvm-cov`** once Task 1 lands a buildable crate — currently `N/A` (no `Cargo.toml`).
  Effort: S.
- **Set up CI** (GitHub Actions) building on Linux with the feature matrix
  (`--no-default-features`, each feature alone, `--all-features`) per the plan's Task 8 check.
  macOS/Windows validation deferred. Effort: M.
- **Pin `ort`** to the exact pre-release rc and add a tracking note — it is pre-1.0 and an
  API-stability risk (design spec, known limitations). Effort: S.

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

## Resolved
- 2026-07-14 — **Design sign-off.** Spec + foundation plan approved interactively (was P1 blocker in
  the prior auto-generated backlog).
- 2026-07-14 — **Crate name decision.** Local-use-only; keep `whisper-rs`; crates.io collision moot.
- 2026-07-14 — **Model bundling / licensing.** Resolved by decision #6: models are consumer-supplied
  by path (default) with an opt-in downloader; the crate never bundles pyannote models, so the
  segmentation-model license is not a redistribution concern.

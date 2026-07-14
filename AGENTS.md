# AGENTS.md
<!-- rev:002 -->

Canonical cross-tool agent instructions for `whisper-rs`. `CLAUDE.md` imports this file — edit here,
not there.

## Overview

`whisper-rs` is a safe Rust wrapper over whisper.cpp for local, offline speech-to-text. Today it
ships batch ASR + word timestamps + a high-level `Pipeline`, post-processing, audio preprocessing, a
real HTTPS model downloader, and the pure/core slices of diarization and streaming. Model-backed
diarization inference and a Silero ONNX VAD upgrade remain blocked on HuggingFace-gated models (see
`docs/ROADMAP.md`).

## Build / test commands

```powershell
# one-time: vendored whisper.cpp submodule must be checked out
git submodule update --init --recursive

cargo build
cargo test              # model-gated tests are #[ignore]d, run without extra setup
cargo test -- --ignored # requires models/ggml-tiny.en.bin + a WAV fixture
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Requires **`libclang`** (bindgen generates FFI bindings from whisper.cpp's header) and a C/C++
toolchain (whisper.cpp + ggml CPU backend are compiled from source via `cc`). On Windows this means
MSVC Build Tools — `build.rs` detects the `-msvc` target and adjusts flags/links `advapi32`
accordingly.

## Feature flags

`default = ["diarization", "streaming", "download"]` in `Cargo.toml`. `download` is fully working
(`ModelRef::download(id)` fetches + caches a GGML model over HTTPS via `ureq`). `diarization` and
`streaming` ship their pure/core logic (types, timeline merge, clustering, `StreamPolicy`,
`StreamSession`) but model-backed diarization inference isn't wired into `Pipeline` yet — it's
blocked on HuggingFace-gated ONNX models (see `docs/ROADMAP.md`). Don't advertise model-backed
diarization as working; do treat the downloader and the pure diarization/streaming core as real,
shipped behavior.

## Code style

- Rust 2021, MSRV 1.75.
- **HARD RULE: `unsafe` only in `src/ffi/`.** Every other module is safe Rust; the FFI layer is the
  sole boundary that touches whisper.cpp's C API and owns the `Context` RAII wrapper.
- One crate-wide error type: `crate::error::WhisperError` (`thiserror`-derived), returned as
  `crate::Result<T>`. Don't introduce a second error enum — extend `WhisperError` instead.
- No panics on expected-failure paths (missing model, bad WAV, resample failure, FFI non-zero
  return) — these are `Result::Err`, not `panic!`/`unwrap()`/`expect()`. Panics are reserved for
  genuine programmer-error invariant violations.
- Prefer composable, small modules matching the layered API (see Architecture below) — the
  high-level `Pipeline` should stay a thin composition of the lower-level stages, not accrue its
  own logic.

## Testing

- Fast/unit tests run under plain `cargo test` — no model or audio fixture required.
- Any test that needs a real GGML model or transcribes real audio is `#[ignore]`d
  (`tests/asr.rs`, `tests/pipeline.rs`). Run them explicitly with `cargo test -- --ignored` after
  placing a model at `models/ggml-tiny.en.bin`. Fixture WAVs live under `tests/fixtures/`.
  `tests/ffi_smoke.rs` is NOT ignored — it links against whisper.cpp and runs by default.
- New behavior in the safe layers (`audio`, `asr`, `timestamps`, `output`, `pipeline`) needs a
  non-ignored unit test where possible; only FFI/model-dependent behavior should be `#[ignore]`d.

## Architecture

Layered, safe-by-construction API:

```
src/
  ffi/               unsafe whisper.cpp bindings + RAII Context (the ONLY unsafe module)
  audio/             WAV decode, downmix, 16 kHz resample (rubato)
    preprocess.rs    preprocessing levels 0-4 (Galle scheme): DC removal, peak normalize, noise gate
    vad.rs           pure energy-based VAD (VadConfig, segment) — Silero ONNX upgrade planned
  asr/               Transcriber: safe wrapper calling ffi::Context, produces Segment/Word
  timestamps/        word-timestamp extraction from raw tokens + monotonicity enforcement
  diarize/           SpeakerTurn/DiarizeConfig types, timeline merge, agglomerative clustering
                      (pure/tested core; ONNX-model-backed inference is blocked — see ROADMAP)
  stream/            StreamPolicy (LocalAgreement2, TwoPass) + StreamSession (push/poll/finalize)
  models/            ModelRef::download cache + resolution behind feature = "download" (ureq)
  postprocess/       normalize_numbers, collapse_repeats, remove_fillers, PostConfig, hallucination
  output.rs          Transcript / Segment / Word / SegmentFlags — structured output types
  pipeline.rs        high-level Pipeline: composes audio -> asr -> output for one-call transcription
  prelude.rs         convenience re-exports for consumers
  error.rs           crate-wide WhisperError + Result
```

`build.rs` compiles `vendor/whisper.cpp` (git submodule, pinned v1.7.4 — core + ggml CPU backend)
via `cc`, then runs `bindgen` over `wrapper.h` to generate `OUT_DIR/bindings.rs`, consumed only by
`src/ffi/`.

## Security

- No bundled models, no bundled audio beyond test fixtures — models are always consumer-supplied by
  path today.
- No secrets. The `download` feature (default-on) makes real outbound HTTPS calls via
  `ureq::get(...)` in `src/models/mod.rs` — it is a genuine network attack surface (URL
  construction, response handling, cache-path writes) and should be treated as a hardening
  checkpoint, not an afterthought (see plan 005).

## Commit conventions

- Conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- No AI attribution (no `Co-Authored-By: Claude` or similar) — use the configured git identity.
- Concise, descriptive messages; one logical change per commit.

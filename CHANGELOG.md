# Changelog

All notable changes to `whisper-rs` are documented here. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/). This crate is local-use (`publish = false`).

## [0.1.0] — 2026-07-15

First stable release of the shipped surface — a safe Rust wrapper over whisper.cpp for local, offline
speech-to-text. Full CI matrix (ubuntu/macos/windows + coverage + audit + MSRV 1.86 + ffmpeg) is green.

### Added
- **Batch ASR** over a WAV file → structured, word-timestamped `Transcript` via the high-level
  `Pipeline` (decode → resample → transcribe) and the lower-level `Transcriber`.
- **Word-level timestamps** from raw tokens with monotonicity enforcement.
- **Audio**: WAV decode + downmix + 16 kHz resample (rubato, delay-line drained); preprocessing
  levels 0–4 (Galle scheme) + pure energy-based VAD.
- **Post-processing**: number normalization, repeat collapse, filler removal, cross-pass
  hallucination flagging.
- **Model downloader** (`feature = "download"`): HTTPS fetch + cache with strict model-id validation
  (path-traversal-safe), Content-Length truncation guard, optional SHA-256 verification, and
  `WHISPER_RS_CACHE_DIR` / `WHISPER_RS_HF_BASE` overrides.
- **Non-WAV media decode** (`feature = "ffmpeg"`): m4a/mp3/flac/mp4/… → 16 kHz mono f32 via
  ffmpeg-next 8.1 (`audio::media::decode_to_mono_16k`, `Pipeline::transcribe_media_file`).
- **Subtitle output**: `Transcript::to_srt` / `to_vtt`.
- **Diarization core** (pure/tested): `SpeakerTurn`/`DiarizeConfig`, timeline merge, O(n²)
  agglomerative clustering. **Streaming core**: `StreamPolicy` (LocalAgreement2, TwoPass) +
  synchronous `StreamSession`.
- `raw-api` feature to surface the raw `ffi` module for power users.
- CI gates: 3-OS matrix, `cargo fmt --check`, `clippy --all-targets -D warnings`, `cargo-audit`,
  MSRV 1.86, and an ffmpeg-feature build job.

### Fixed
- The crate now builds on **Linux and macOS** — `build.rs` previously compiled ggml's `.c` sources as
  C++ (via a single `.cpp(true)` build), which hard-errored under GCC; the C sources now compile as C
  with `_GNU_SOURCE` for the glibc affinity symbols.

### Known limitations
- Model-backed diarization (pyannote ONNX) and a Silero ONNX VAD upgrade are **blocked** on
  HuggingFace-gated models (a human license step) — the pure clustering/merge/VAD cores ship today.
- Overlapping/simultaneous speech is out of scope; word timestamps are per-token (not DTW-aligned);
  each `Transcriber` is single-threaded (whisper.cpp state is not `Sync`).

[0.1.0]: https://github.com/inovacc/whisper-rs/releases/tag/v0.1.0

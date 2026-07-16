# ADR-0003 — Feature-gated optional capabilities

**Status:** Accepted (2026-07-15)

## Context

Several capabilities carry a cost that not every consumer should pay: the model downloader adds a TLS
stack (`ureq` + `sha2`), non-WAV decode links native ffmpeg 8.x libraries, observability pulls in
`tracing`, and diarization/streaming add surface area. Forcing all of these on every build inflates
compile time, dependency count, and the native-toolchain requirements.

## Decision

**Optional capabilities are Cargo features, off-by-default where they add native or heavyweight deps.**

- `default = ["diarization", "streaming", "download"]` — pure/core logic plus the pure-Rust downloader.
- `ffmpeg` — opt-in; links external ffmpeg 8.x shared+dev libraries (`FFMPEG_DIR`).
- `raw-api` — opt-in; surfaces the raw `ffi` module for power users.
- `tracing` — opt-in; the internal trace facade no-ops entirely when disabled.

## Consequences

- The default build compiles with only a C/C++ toolchain + `libclang`; no ffmpeg, no logging subscriber.
- Public enums touched by features (`WhisperError`, `ModelRef`) are `#[non_exhaustive]`, so feature-gated
  variants/paths never become a breaking change.
- CI must exercise the feature matrix: default, `--no-default-features`, the in-tree feature set, and a
  dedicated `ffmpeg` job that installs the native libraries.
- Consumers opt into cost explicitly, and the crate's default footprint stays minimal.

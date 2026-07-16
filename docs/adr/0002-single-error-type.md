# ADR-0002 — One crate-wide error type

**Status:** Accepted (2026-07-15)

## Context

The crate spans several failure domains: audio decode/resample, FFI non-zero returns, missing models,
network downloads, and configuration errors. A per-module error enum with `From` glue between them is a
common Rust pattern but multiplies boilerplate and makes the public API harder to match on.

## Decision

**A single `thiserror`-derived `crate::error::WhisperError`, returned as `crate::Result<T>`.** New
fallible states are added as new variants of `WhisperError` rather than a second enum. The type is
`#[non_exhaustive]` so adding a variant is not a downstream-breaking change (see
[ADR-0003](0003-feature-gated-optional-capabilities.md) and the semver policy in AGENTS.md).

## Consequences

- Consumers match on one error type with a wildcard arm; error handling stays uniform.
- `#[from]` conversions (e.g. `std::io::Error`) make `?` ergonomic across layers.
- No panics on expected-failure paths — missing model, bad WAV, resample failure, FFI non-zero, and
  (as of the observability pass) missing ffmpeg filter pads are all `Result::Err`. Panics are reserved
  for genuine invariant violations (e.g. `diarize::cluster` asserting a live-cluster pair exists).
- The trade-off — a broad enum rather than tightly-scoped per-module errors — is accepted for the
  ergonomic and API-stability gains at this crate's size.

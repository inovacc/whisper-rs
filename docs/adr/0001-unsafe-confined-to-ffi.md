# ADR-0001 — `unsafe` confined to `src/ffi/`

**Status:** Accepted (2026-07-15)

## Context

`whisper-rs` links whisper.cpp's C API through `bindgen`-generated bindings. FFI calls, raw pointers,
and manual lifetime management are inherently `unsafe`. Left unchecked, `unsafe` tends to leak across a
codebase, eroding the safety guarantees that are the whole point of a Rust wrapper.

## Decision

**All `unsafe` lives in `src/ffi/` and nowhere else.** This is a hard rule enforced in AGENTS.md. The
FFI layer is the sole boundary that touches whisper.cpp's C API and owns the `Context` RAII wrapper
(allocation on `from_file`, release on `Drop`). Every other module — `audio`, `asr`, `timestamps`,
`diarize`, `stream`, `models`, `postprocess`, `output`, `pipeline` — is safe Rust and reaches
whisper.cpp only through the safe methods `ffi::Context` exposes.

## Consequences

- The safe layers cannot cause UB directly; any memory-safety audit narrows to one small module.
- `ffi::Context` must present a fully safe, panic-free API (fallible operations return `Result`), which
  it does — the safe layers never see a raw pointer.
- New whisper.cpp capabilities require adding a safe method to `ffi::Context` first, then consuming it
  from a safe layer — a deliberate, reviewable choke point.
- The `raw-api` feature can surface the raw `ffi` module for power users, but doing so is explicitly
  opt-in and documented as an escape hatch, not the supported path.

# Plan 003: Delete dead code — unused `ndarray` dependency and `Context::as_ptr`

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- Cargo.toml src/ffi/mod.rs src/diarize`
> If in-scope files changed, re-verify the "Current state" facts before editing.

## Status
- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: tech-debt
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
Two pieces of speculative code add surface with zero callers. (1) The `diarization` feature declares
`ndarray` but nothing in `src/` uses it — every default build compiles it for nothing. (2)
`ffi::Context::as_ptr` is `pub(crate)` with no call sites — a raw-pointer escape hatch that widens the
`unsafe`-adjacent API with no consumer. Deleting both reduces build time and the misuse surface, per the
restraint principle (the fix for speculative code is deletion).

## Current state (verify before editing)
- `Cargo.toml:11,19` — `diarization = ["dep:ndarray"]` and an optional `ndarray` dependency. Confirm it's
  unused: `git grep -n "ndarray" -- src/` returns **no** hits (only docs reference it). `src/diarize/cluster.rs`
  operates on plain `&[Vec<f32>]`.
- `src/ffi/mod.rs:106-108` — `pub(crate) fn as_ptr(&self) -> *mut whisper_context { self.0 }`. Confirm no
  callers: `git grep -n "\.as_ptr()" -- src/` shows only `CString`/slice `.as_ptr()` (lines ~37, 66, 71) and
  the definition — none call `Context::as_ptr`. (The FFI methods use `self.0` directly.)
- Convention: `unsafe` only in `src/ffi/`; feature gating in `Cargo.toml`.

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Confirm ndarray unused | `git grep -n "ndarray" -- src/` | no matches |
| Confirm as_ptr unused | `git grep -n "Context::as_ptr\|self.as_ptr\|ctx.as_ptr" -- src/` | no matches |
| Build all features | `cargo build --all-features` | exit 0 |
| Build default | `cargo build` | exit 0 |
| Tests | `cargo test --all-features` | all pass |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |

## Scope
**In scope:** `Cargo.toml`, `src/ffi/mod.rs`.
**Out of scope:** `src/diarize/**` (do not change clustering — it correctly uses `Vec<f32>`); any other
feature. If a future ONNX diarizer needs `ndarray`/`as_ptr`, re-add them *then* (BACKLOG P1/P3), not now.

## Git workflow
- Branch `advisor/003-delete-dead-code`; message `chore: remove unused ndarray dep and dead Context::as_ptr`. Do NOT push.

## Steps
### Step 1: Remove the `ndarray` dependency
In `Cargo.toml`: change `diarization = ["dep:ndarray"]` to `diarization = []`, and delete the optional
`ndarray = { … }` dependency line. If removing it leaves `[dependencies]` referencing nothing else new,
that's fine.

**Verify**: `cargo build --features diarization` and `cargo build --all-features` → exit 0;
`git grep -n "ndarray" -- Cargo.toml` → no matches.

### Step 2: Remove `Context::as_ptr`
In `src/ffi/mod.rs`, delete the `pub(crate) fn as_ptr(&self) -> *mut whisper_context { self.0 }` method (and
its doc comment if any). Do not change any other method.

**Verify**: `cargo build --all-features` → exit 0; `git grep -n "fn as_ptr" -- src/ffi/mod.rs` → no matches.

### Step 3: Full verification
**Verify**: `cargo test --all-features` all pass; `cargo clippy --all-targets --all-features -- -D warnings`
exit 0; `cargo build --no-default-features` exit 0.

## Test plan
No new tests — this is pure deletion covered by the existing suite. The gate is that all existing tests still
pass at `--all-features` and the crate builds at every feature combination.

## Done criteria
- [ ] `git grep -n "ndarray" -- src/ Cargo.toml` → no matches
- [ ] `git grep -n "fn as_ptr" -- src/ffi/mod.rs` → no matches
- [ ] `cargo build`, `cargo build --all-features`, `cargo build --no-default-features` all exit 0
- [ ] `cargo test --all-features` all pass; clippy clean
- [ ] Only `Cargo.toml` and `src/ffi/mod.rs` modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions
- `git grep` finds an actual `ndarray` use in `src/`, or a real caller of `Context::as_ptr` — the drift check
  failed; report instead of deleting (something now depends on them).
- Removing `ndarray` breaks a build that isn't obviously fixed by the feature-list edit — report.

## Maintenance notes
- When the ONNX diarizer / Silero VAD lands (BACKLOG P1/P3), whichever tensor lib it needs is added at that
  point; do not pre-wire it. A reviewer should confirm no `#[cfg]`-gated code silently relied on `ndarray`.

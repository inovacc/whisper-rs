# Plan 002: Reconcile README / AGENTS.md / ISSUES.md with the shipped code

> **Executor instructions**: Follow step by step; run every verification and confirm before moving on. On any
> STOP condition, stop and report. Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- README.md AGENTS.md docs/ISSUES.md src`
> If in-scope docs or `src` changed since this plan was written, re-verify the claims below against the live
> code before editing.

## Status
- **Priority**: P1
- **Effort**: S
- **Risk**: LOW (docs only; no code change)
- **Depends on**: none
- **Category**: docs
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
`README.md` and `AGENTS.md` (both `rev:001`, written before three feature cycles landed) still describe
diarization, streaming, and the model downloader as "empty stubs" and claim `ModelRef::download(...)` "always
returns `WhisperError::Config`". All three are implemented and tested. `AGENTS.md` even tells agents "no
network calls in the current codebase," which is false and would cause an agent to skip real network
hardening. `docs/ROADMAP.md` (rev:006) is accurate — README/AGENTS/ISSUES are the drifted copies. Wrong docs
are worse than missing ones.

## Current state (claims to fix — verify each against the cited code before editing)
- `README.md:8-31, 104-114` — says diarization/streaming/preprocessing/downloader are "planned … empty
  stubs"; twice says `ModelRef::download` "always returns `WhisperError::Config`". **Reality:**
  `src/pipeline.rs:19-31` resolves `Download` via `crate::models::download_model` under `#[cfg(feature="download")]`;
  `src/models/mod.rs:19-38` is a working `ureq` downloader. `src/diarize/`, `src/stream/`, `src/postprocess/`
  are implemented + tested.
- `README.md:18-31` Features omits shipped, always-available surface: `postprocess::{normalize_numbers,
  collapse_repeats, remove_fillers, PostConfig}`, `postprocess::hallucination`, `audio::preprocess` (L0–L4),
  `audio::vad::segment`.
- `AGENTS.md:33-36` — "All three are currently **empty stubs** … `ModelRef::download(...)` always returns
  `WhisperError::Config` today." False.
- `AGENTS.md:84-88` (Security) — "no network calls in the current codebase (the `download` feature is a
  stub)." Contradicted by `src/models/mod.rs:26` (`ureq::get(...).call()`).
- `AGENTS.md:66-76` architecture tree omits `diarize/`, `stream/`, `models/`, `postprocess/`,
  `audio/preprocess.rs`, `audio/vad.rs`.
- `docs/ISSUES.md:38` — "CI currently builds Linux only." **Reality:** `.github/workflows/ci.yml` matrixes
  `ubuntu-latest, macos-latest, windows-latest`. Move that item to Resolved.
- `AGENTS.md:53-60` (or wherever the `#[ignore]` list is) — lists `tests/ffi_smoke.rs` among the ignored
  tests, but `tests/ffi_smoke.rs:1-6` is NOT `#[ignore]`d (it runs by default). Correct that.
- Convention: living instruction/reference docs carry `<!-- rev:NNN -->` on the line after the H1 — bump it
  by exactly 1 on any edited living doc (README, AGENTS, ISSUES are living → bump; do NOT stamp dated
  specs/plans).

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Confirm downloader is real | `grep -n "ureq::get" src/models/mod.rs` | one hit (line ~26) |
| Confirm CI matrix | `grep -n "macos-latest" .github/workflows/ci.yml` | ≥1 hit |
| Build (docs don't break doctests) | `cargo build` | exit 0 |

## Scope
**In scope:** `README.md`, `AGENTS.md`, `docs/ISSUES.md`.
**Out of scope:** `docs/ROADMAP.md`, `docs/BACKLOG.md` (already accurate — do not edit); all `src/**`;
`docs/superpowers/**` (dated specs/plans — leave as-is).

## Git workflow
- Branch `advisor/002-reconcile-docs`; conventional-commit message e.g. `docs: reconcile README/AGENTS/ISSUES with shipped features`. Do NOT push.

## Steps
### Step 1: Fix README
Rewrite the Status/Features sections so they state what ships today (batch ASR + word timestamps + layered
Pipeline + **post-processing** [numbers/repeats/fillers/hallucination] + **preprocessing** [levels 0–4 + energy
VAD] + **model downloader** + diarization **core** [types/merge/clustering] + streaming **core & session**),
and what is still blocked (ONNX-model-backed diarization + Silero VAD, needing HF-gated models). Remove both
"`ModelRef::download` always returns Config" claims. Add a "Post-processing & preprocessing" subsection listing
the public entry points above. Bump the README `<!-- rev -->` by 1.

**Verify**: `grep -n "empty stub" README.md` → no matches; `grep -n "always returns" README.md` → no matches
referring to download.

### Step 2: Fix AGENTS.md
Correct the "empty stubs" and "no network calls" statements; update the Security section to note the
downloader makes real HTTPS calls via `ureq` (and that it's a genuine hardening surface — see plan 005). Add
`diarize/`, `stream/`, `models/`, `postprocess/`, `audio/preprocess.rs`, `audio/vad.rs` to the architecture
tree. Fix the ignored-tests list to not include `tests/ffi_smoke.rs`. Bump AGENTS `<!-- rev -->` by 1.

**Verify**: `grep -n "empty stub" AGENTS.md` → no matches; `grep -n "no network calls" AGENTS.md` → no matches.

### Step 3: Fix ISSUES.md
Change the "CI builds Linux only" line to reflect the 3-OS matrix and move it to the Resolved section (dated
2026-07-14). Bump ISSUES `<!-- rev -->` by 1.

**Verify**: `grep -n "Linux only" docs/ISSUES.md` → no matches.

## Test plan
No new tests (docs only). Verification: `cargo build` still exits 0 (confirms no doctest fences were broken if
any exist in README-linked code — README code fences should be ```rust,no_run or ```text if they can't compile).

## Done criteria
- [ ] `grep -rn "empty stub" README.md AGENTS.md` → no matches
- [ ] `grep -n "no network calls" AGENTS.md` → no matches
- [ ] `grep -n "Linux only" docs/ISSUES.md` → no matches
- [ ] Each edited doc's `<!-- rev -->` incremented by exactly 1
- [ ] `cargo build` exits 0
- [ ] Only `README.md`, `AGENTS.md`, `docs/ISSUES.md` modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions
- The code claims above don't match live code (e.g. the downloader was reverted) — report; the docs should
  match whatever the code actually does.
- An edit would require touching `src/**` — out of scope; report.

## Maintenance notes
- Keep README/AGENTS in sync when plans 003–010 land (e.g. plan 003 removes `ndarray`; plan 005 hardens the
  downloader — reflect both). A reviewer should diff docs against `docs/ROADMAP.md` for consistency.

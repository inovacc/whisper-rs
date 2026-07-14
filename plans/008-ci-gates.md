# Plan 008: Add CI gates — `cargo fmt --check`, `cargo-audit`, and an MSRV leg

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- .github/workflows/ci.yml Cargo.toml`

## Status
- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none — but **run this plan LAST** among code-changing plans (see Why).
- **Category**: dx
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
Formatting is unenforced: CI runs clippy but not `cargo fmt --check`, and there's no `rustfmt.toml`, yet the
source uses a wide-line house style (many single-line fn bodies exceed rustfmt's default `max_width=100`) and
`AGENTS.md` tells agents to run `cargo fmt` — which would produce a huge reflow diff against undefined style.
The new `ureq` TLS dependency tree ships with no advisory/license gate. And MSRV `1.75` is declared but never
built. This plan pins the format style, runs `cargo fmt` once to normalize, and adds fmt-check + `cargo-audit`
+ an MSRV CI leg. **Run last** because the one-time `cargo fmt` reflow would otherwise collide with plans
001–007/010's diffs.

## Current state (verify before editing)
- `.github/workflows/ci.yml` — a `test` job (OS matrix: ubuntu/macos/windows) that installs libclang, runs
  `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `--no-default-features` /
  `--all-features` builds; plus a Linux `coverage` job running `cargo llvm-cov --summary-only`. `rustfmt` is
  installed (`components: clippy, rustfmt`) but never invoked.
- No `rustfmt.toml`, no `deny.toml` at repo root.
- `Cargo.toml:5` — `rust-version = "1.75"`. Source has many >100-col lines (e.g. `src/audio/mod.rs:22,29`,
  `src/pipeline.rs` one-line fns).
- Repo build needs the whisper.cpp submodule + libclang (the CI already handles this in existing jobs — copy
  that setup for any new job that compiles).

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Format check | `cargo fmt --all -- --check` | after Step 2, exit 0 |
| Normalize once | `cargo fmt --all` | rewrites files to the pinned style |
| Build | `cargo build` | exit 0 |
| Audit (local, optional) | `cargo audit` | runs (install: `cargo install cargo-audit`) |

## Scope
**In scope:** `.github/workflows/ci.yml`, a new `rustfmt.toml` (create), optionally `deny.toml` (create), and
the whole tree's formatting (from the one-time `cargo fmt --all`).
**Out of scope:** any logic change; `Cargo.toml` dependency versions (do NOT change deps); the coverage job.

## Git workflow
- Branch `advisor/008-ci-gates`; commit the `rustfmt.toml` + the `cargo fmt` reflow as ONE commit
  (`style: add rustfmt.toml and normalize formatting`) and the CI changes as a second
  (`ci: add fmt-check, cargo-audit, and MSRV 1.75 job`). Do NOT push.

## Steps
### Step 1: Pin the format style
Create `rustfmt.toml` capturing the existing house style so `cargo fmt` doesn't reflow everything. At minimum
set `max_width` to match the code (inspect the widest intentional lines; `120` is likely — verify a few files
and pick the smallest value that leaves the current single-line fns intact). Keep it minimal (just `max_width`
unless another option is clearly needed).

**Verify**: `cargo fmt --all -- --check` — note how many files would change (this is expected before Step 2).

### Step 2: Normalize once
Run `cargo fmt --all`. Review `git diff --stat` — it should be formatting-only (whitespace/wrapping), no
semantic changes. Then `cargo build` + `cargo test` to confirm nothing broke.

**Verify**: `cargo fmt --all -- --check` → exit 0 (clean); `cargo test` → all pass.

### Step 3: Add the fmt-check CI step
In the `test` job of `.github/workflows/ci.yml`, add a step (after checkout/toolchain) running
`cargo fmt --all -- --check`. Since `rustfmt` is already in `components`, no new install is needed. Run it once
(it's OS-independent) — you may guard it with `if: runner.os == 'Linux'` to avoid running on all three legs.

**Verify**: YAML is valid (`grep -n "cargo fmt" .github/workflows/ci.yml` shows the step).

### Step 4: Add a dependency-audit job
Add a `security-audit` job: checkout, install cargo-audit via `taiki-e/install-action@cargo-audit` (matches the
existing `cargo-llvm-cov` install pattern), run `cargo audit`. Optionally add `deny.toml` + a `cargo-deny` job
for license/ban checks instead — pick `cargo-audit` (simpler) unless the repo already has a `deny.toml`.

**Verify**: `grep -n "cargo audit\|cargo-audit" .github/workflows/ci.yml` shows the job.

### Step 5: Add an MSRV leg
Add a job (or matrix entry) that builds on Rust `1.75` using `dtolnay/rust-toolchain@1.75` with the same
libclang + submodule setup as the existing Linux build, running `cargo build` (build only — not the full test
matrix). If `cargo build` fails on 1.75 because a dependency (`ureq`/`bindgen 0.72`/`rubato`/etc.) requires a
newer compiler, **STOP and report** — the declared MSRV is then wrong and raising/dropping the
`rust-version` claim is a maintainer decision.

**Verify**: `grep -n "1.75" .github/workflows/ci.yml` shows the leg.

## Test plan
No source tests. The gates ARE the verification: `cargo fmt --all -- --check` exits 0 locally, and the new CI
steps are syntactically present. (The MSRV/audit jobs run in CI, not locally.)

## Done criteria
- [ ] `rustfmt.toml` exists; `cargo fmt --all -- --check` exits 0 locally
- [ ] `cargo test` passes after the reflow (no semantic change)
- [ ] CI has a `cargo fmt --all -- --check` step, a `cargo audit` job, and a Rust `1.75` build leg
- [ ] No dependency versions changed (`git diff Cargo.toml` shows no dep edits)
- [ ] `plans/README.md` status row updated

## STOP conditions
- `cargo build` fails on Rust `1.75` — report (MSRV claim is inaccurate; maintainer decides whether to raise
  `rust-version` or the toolchain).
- The one-time `cargo fmt` produces a diff that includes anything other than formatting — STOP; something is
  wrong with the `rustfmt.toml` or the run.

## Maintenance notes
- Because this plan reflows the whole tree, land it AFTER 001–007/010 to avoid rebase pain. A reviewer should
  confirm the `cargo fmt` commit is formatting-only (diff is whitespace/wrapping). Keep `rustfmt.toml`'s
  `max_width` consistent with what the team actually writes.

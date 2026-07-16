# Maturity Rating — whisper-rs — 2026-07-15

**Project type:** Rust library crate (`Cargo.toml`, edition 2021, MSRV 1.86) — safe wrapper over
whisper.cpp for local, offline speech-to-text.
**Maturity stage:** **4 — Production** · **Weighted score: 89.8 / 100** · **Confidence: High**
(8 of 10 dimensions measured; Documentation + Stability at Medium).

This is a dated point-in-time record. It measures the crate at commit HEAD (post-v0.1.0 tag) against
the 10-dimension maturity rubric. Every score cites a real signal — no invented numbers.

## Scorecard

| # | Dimension | Wt | Grade | Pts | Evidence |
|---|-----------|----|-------|-----|----------|
| 1 | Architecture & Boundaries | 3 | **A** | 100 | Layered `src/` (ffi/audio/asr/timestamps/diarize/stream/models/postprocess); HARD rule `unsafe` only in `src/ffi/` holds; single `WhisperError`. Weakness: no `#[non_exhaustive]` on public `WhisperError`/`ModelRef` — latent breaking-change trap. |
| 2 | Testing & Coverage | 5 | **B** | 82 | `cargo llvm-cov` = **78.94% line / 76.68% region** (measured); 64 tests green + 7 model-gated `#[ignore]`d. Gaps: ffmpeg decode path never test-run (build-only), `pipeline.rs` ~50%, no CI job runs the model-gated ASR tests. Misses the 80% Stage-5 line by 1.06 pts. |
| 3 | CI/CD & Release | 4 | **B** | 82 | Full 7-job matrix green (3 OS + coverage + audit + MSRV 1.86 + ffmpeg); run 29458978444. Gates: fmt/clippy `-D warnings`/audit. Weakness: fully manual release, 1 tag, no changelog automation. |
| 4 | Security | 4 | **A−** | ~90 | `cargo audit` clean; no plaintext secrets; downloader hardened (id validation, truncation guard, SHA-256). Weakness: default `HF_BASE` uses mutable `resolve/main` (TOFU); no CVE tracking for vendored whisper.cpp v1.7.4. |
| 5 | Documentation | 2 | **A** | 100 | README + AGENTS/CLAUDE + ROADMAP/BACKLOG/ISSUES + CHANGELOG + release-readiness doc, all current. Weakness: design decisions live only as AGENTS.md prose (no `docs/ARCHITECTURE.md`/ADRs); no enforced doc coverage. |
| 6 | Operational Readiness | 4 | **B** | 82 | Typed `Result` error handling throughout; no-panic rule; long-audio guard. Weakness: **zero observability** — no `tracing`/`log` in a crate doing FFI + network + long-running transcription. |
| 7 | Code Quality & Tech Debt | 3 | **A−** | ~90 | clippy `-D warnings` clean; low TODO density; dead code removed (plan 003). Weakness: 5 `.expect()` sites in `media.rs` + 1 in `cluster.rs` — the only deviation from the no-panic-on-expected-failure rule. |
| 8 | Dependency & Supply-chain | 3 | **A−** | ~90 | `cargo audit` clean; lockfile present; MSRV enforced. Weakness: loose caret ranges, no `cargo-deny` (no license/ban policy), pre-release `ort` unpinned, stale "MSRV 1.75" doc mentions. |
| 9 | Stability & Change Mgmt | 3 | **B** | 82 | Conventional commits, deprecation policy documented, backlog consolidated, v0.1.0 tagged. Weakness: single release → no cross-version API history; `ort`/mutable-HF_BASE queued as debt. |
| 10 | Correctness & Robustness | 4 | **A** | 100 | Errors propagated as `Result`; `Context: Send` not `Sync` enforces single-thread safety at the type level; resampler delay drained; finalize data-loss fixed. Weakness: real-audio regression only manually verified (no CI harness). |

**Weighted rollup:** Σ(points × weight) = **3142** → 3142 / 35 = **89.8** → **Stage 4 (Production)**.

## Ranked weak points

1. **Testing (wt 5):** 78.94% < 80%; ffmpeg decode + model-gated ASR paths never run in CI — the crate's largest verification hole.
2. **Operational Readiness (wt 4):** zero observability (no `tracing`/`log`) across FFI + network + long loops.
3. **CI/CD (wt 4):** release is fully manual; 1 tag, no changelog automation.
4. **Security (wt 4):** mutable `HF_BASE` TOFU + no vendored-CVE tracking (severe but isolated).
5. **Architecture / Stability (wt 3):** public `WhisperError`/`ModelRef` lack `#[non_exhaustive]` — a latent forced-breaking-release trap, cheapest to fix now while `publish = false`.
6. **Code Quality (wt 3):** 6 `.expect()` sites (`media.rs`×5, `cluster.rs`×1) violate the no-panic rule.
7. **Dependencies (wt 3):** no `cargo-deny`, loose caret ranges, unpinned pre-release `ort`, stale MSRV doc text.

## Improvement route — Stabilize → Harden → Mature

Leverage = unblock fan-out × impact(weight) ÷ effort. **Severity ≠ leverage:** the most *severe*
single item (HF_BASE TOFU) ranks below cheaper items that unblock more dimensions.

### Phase 1 — Stabilize
- **1.1 · Add `#[non_exhaustive]` + semver policy** (S) — annotate `WhisperError` (`error.rs`) and `ModelRef` (`pipeline.rs`); one-paragraph policy in AGENTS.md. Unblocks API-stability → Stability + Dependencies. **Time-sensitive:** land now while `publish = false` and there are no external consumers to break.
- **1.2 · Live-integration CI harness (the one thing)** (M) — a CI job caching `ggml-tiny.en.bin` + a WAV, running `cargo test -- --ignored` **and** the ffmpeg decode path on a real sample. Unblocks Testing (past 80%), Correctness (real-audio regression), Ops (decode confidence).
- **1.3 · Fix MSRV doc drift + reproducible resolution** (S) — replace stale "1.75" mentions; add a `--frozen`/`--locked` CI leg. Unblocks Dependencies + CI/CD.
- **1.4 · `media.rs` `.expect()` → `Result`** (M) — thread `WhisperError` through the filter-graph pad construction; add a bad-input test. Closes the last no-panic deviation.

### Phase 2 — Harden
- **2.1 · `tracing`/`log` facade** (M) — optional `tracing` feature; instrument `ModelRef::download` + the ASR transcribe loop. Closes the biggest Ops gap; aids diagnosing 1.2 failures.
- **2.2 · `cargo-deny` (license/bans/advisories)** (S) — commit `deny.toml`, add a CI `cargo deny check` gate.
- **2.3 · Harden the model source** (M) — document + support pinning `HF_BASE` to an immutable `resolve/<sha>` digest; track whisper.cpp v1.7.4 advisories in `docs/ISSUES.md`.
- **2.4 · Raise `pipeline.rs` coverage** (M) — edge tests now measurable under 1.2; a `proptest` monotonicity property for word-timestamp enforcement.

### Phase 3 — Mature
- **3.1 · Automate releases + changelog** (M) — `release-plz`/`cargo-dist` tag-triggered job; builds the cross-version history Stability lacks.
- **3.2 · `docs/ARCHITECTURE.md` + ADRs + `#![deny(missing_docs)]`** (M) — migrate the AGENTS.md Architecture section to a Mermaid layer diagram; ADR-0001 = "unsafe confined to `src/ffi/`".
- **3.3 · Consolidate the pre-1.0 backlog** (S) — pin `ort` exactly; finalize the mutable-HF_BASE decision; close the corresponding BACKLOG P2 items.

## The one thing

**Stand up a live-integration CI harness** (a job that caches a tiny GGML model + WAV, runs the 7
`#[ignore]d` model-gated ASR tests, and exercises the ffmpeg decode path on a real sample). It has
the largest absolute fan-out on the heaviest-weighted dimension: it pushes **Testing** past 80%, gives
**Correctness** real-audio regression detection, and gives **Ops** decode confidence — converting the
crate's biggest Stage-5 gap from a manual ritual into an automated gate. Three auditors independently
pointed at this node. (Cheaper first move, if sequencing: `#[non_exhaustive]` — 1.1 — is S-effort and
time-sensitive; do it in the same session.)

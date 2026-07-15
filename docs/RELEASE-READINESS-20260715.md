# Stable Release Readiness — whisper-rs — 2026-07-15

**Verdict:** GO (v0.1.0) · **Overall confidence:** 83% · **Maturity:** release-candidate · **Recommended version:** v0.1.0

Scope note: this assesses a **stable 0.x release of the currently-shipped surface** (batch ASR + pipeline
+ post/pre-processing + hardened downloader + ffmpeg decode + the pure diarization/streaming cores). The
model-backed diarization/streaming inference remains blocked on HuggingFace-gated ONNX models — that is a
documented external limitation, not a defect, and is out of scope for a 0.1.0.

## Scorecard
| Dimension | Confidence | Maturity | Headline |
|---|---|---|---|
| Database migrations | 100% | stable | N/A — library crate, no datastore |
| Code confidence (tests) | 80% | release-candidate | 78.94% line / 76.68% region; 64 tests green, 7 model-gated |
| Build / lint / CI gates | 92% | stable | Full CI green: 3-OS + coverage + audit + MSRV 1.86 + ffmpeg |
| Bugs / debt / deprecations | 85% | stable | No open bugs; only documented by-design limits; no deprecations |
| Feature completeness | 75% | beta | Shipped surface complete; model-backed diarization blocked (HF) |
| Release / distribution | 70% | alpha | `publish=false`, no tags yet; distributed by git/path |
| **Overall (weighted)** | **83%** | **release-candidate** | Green gate + solid coverage; flagship diarization not operational |

Weights: migrations 15 · code-confidence 25 · build/CI 15 · bugs/debt 15 · feature-completeness 20 · distribution 10.

## Database migration plan
N/A — `whisper-rs` is a Rust library with no database, no migration system. No migration risk to the cut.

## Per-dimension detail

**1. Migrations — 100%.** No `migrations/`, no ORM, no schema. Evidence: repo layout; `Cargo.toml`.

**2. Code confidence — 80%.** Recorded `cargo llvm-cov` (default features): **78.94% line / 76.68% region**
(docs/ROADMAP.md). 64 non-ignored tests + 7 model-gated `#[ignore]`d, all green in CI. Lower-covered spots:
`pipeline.rs` (~50%, thin composition), `error.rs` (~71%, Display arms). Weighted up for the well-tested hot
paths (audio decode/resample, VAD, clustering, downloader validation, postprocess, output). Model-gated ASR
content verified manually against real audio (jfk + voice notes). Gap: no coverage on the `ffmpeg` feature
path in the % (feature excluded from the coverage run).

**3. Build / lint / CI gates — 92%.** CI green at HEAD across all 7 jobs: `test` on ubuntu/macos/windows,
`coverage`, `security-audit` (cargo-audit), `msrv (1.86)`, and `ffmpeg feature (Linux)`. Gates: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `--no-default-features` build, feature-set build.
Just fixed a latent bug where the crate **never built on Linux/macOS** (ggml.c compiled as C++). Evidence:
run `29454221778` (all success); `.github/workflows/ci.yml`.

**4. Bugs / debt / deprecations — 85%.** No `docs/BUGS.md`, no open P1/P2 bugs. `docs/ISSUES.md` lists only
by-design/upstream limitations (overlapping speech unhandled; per-token — not DTW — timestamps; single-thread
per `Transcriber`; WAV-only default now covered by the `ffmpeg` feature; ~37 h input guard). No deprecations,
no removal-dated cleanups. Tech debt low after the 10-plan maturation pass + steps:next hardening batch.
External blocker (not a bug): diarization ONNX models are HF-gated.

**5. Feature completeness — 75%.** Delivered + operationally live: batch ASR, word timestamps, high-level
`Pipeline`, post-processing (numbers/repeats/fillers/hallucination), preprocessing L0–4 + energy VAD, hardened
HTTPS downloader (id validation, truncation guard, SHA-256), SRT/VTT writers, `ffmpeg` non-WAV decode, and the
pure/tested diarization (merge + agglomerative clustering) and streaming (`LocalAgreement2`/`TwoPass` +
`StreamSession`) cores. Built-but-not-operational (blocked): `ort` + pyannote ONNX diarization inference,
Silero ONNX VAD — all gated on HF license acceptance (a human step). The original v1 vision's headline
differentiator (model-backed diarization) is thus not yet activated → operational-completeness < feature-
completeness. Correct framing for a **0.x** cut, not a 1.0.

**6. Release / distribution — 70%.** `version = "0.1.0"`, `publish = false` (local-use; not crates.io). No git
tags yet; never published. No `.goreleaser`/`cargo dist` — a library consumed via git/path dependency, so a
"release" = a git tag. CI produces no artifacts (nor should it for a lib). Remote `inovacc/whisper-rs` is
public with green CI.

## Blockers to stable (ranked)
No **high**-severity blockers for a v0.1.0 of the current surface. Notable non-blockers:
1. [medium] Model-backed diarization/streaming inference blocked on HF-gated ONNX — **documented limitation**,
   deferred to a later minor. Ship 0.1.0 with it noted as known-not-yet-available.
2. [low] `ffmpeg` feature path excluded from the coverage %; verified via CI build + manual e2e instead.
3. [low] Default `HF_BASE` still uses the mutable `resolve/main` ref (override exists; pinning the default
   is a supply-chain nicety, BACKLOG P2).

## Recommended version + rationale
**v0.1.0** — the first stable tag. Pre-1.0 is correct: the flagship differentiator (model-backed diarization)
is not operational, and the public API may still evolve as diarization/streaming inference lands. `0.1.0`
signals "stable, usable foundation; API not yet frozen." Reserve `1.0.0` for when model-backed diarization is
activated and the API is committed.

## Smallest path to GO
Already GO for v0.1.0. To cut it:
- [x] Full CI matrix green (done — run 29454221778).
- [x] No open high-severity bugs (none).
- [x] Docs reflect the shipped vs blocked surface (ROADMAP/BACKLOG/ISSUES current).
- [ ] Tag `v0.1.0` and push the tag.
- [ ] (optional) Add a `CHANGELOG.md` / GitHub release notes for the tag.

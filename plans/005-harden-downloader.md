# Plan 005: Harden the model downloader — id validation, checksum, truncation detection

> **Executor instructions**: Follow step by step; verify each step. On any STOP condition, stop and report.
> Update this plan's row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1cfc289..HEAD -- src/models/mod.rs src/error.rs src/pipeline.rs tests/models.rs`
> Re-verify "Current state" if any changed.

## Status
- **Priority**: P1
- **Effort**: M
- **Risk**: LOW (adds validation + a verify step; rejects only malformed/corrupt inputs)
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `1cfc289`, 2026-07-14

## Why this matters
`models::download_model`/`cached_path` interpolate the caller-supplied `id` straight into a filename and URL.
`cached_path` does `cache_dir.join(format!("ggml-{id}.bin"))`; the `"ggml-"` prefix only neutralizes the
first path component, so an `id` like `foo/../../etc/x` still escapes `cache_dir` (attacker-directed file
write, if a downstream passes an untrusted id). Separately, downloads have **no integrity check**: the body is
streamed to a temp file and renamed with no size/checksum verification, so a cleanly-closed but truncated
response (or a repository-side change on the mutable `main` ref) is cached permanently and fed to the native
GGML parser. This plan adds strict id validation, a byte-count/Content-Length truncation guard, and an
optional SHA-256 verification hook.

## Current state (verify before editing)
- `src/models/mod.rs`:
  - `:5` — `const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";` (mutable ref).
  - `:8-10` — `model_url(id)` = `format!("{HF_BASE}/ggml-{id}.bin")`.
  - `:13-15` — `cached_path(id, cache_dir)` = `cache_dir.join(format!("ggml-{id}.bin"))`.
  - `:19-38` — `download_model(id, cache_dir)`: early-return if `dest.exists()`; else `create_dir_all`,
    `ureq::get(&url).call()`, `into_reader()`, `std::io::copy` into `<dest>.bin.part`, `std::fs::rename`.
  - `:41-43` — `default_cache_dir()` = `PathBuf::from("models")`.
- `src/error.rs` — `WhisperError` (thiserror) has a `ModelDownload(String)` variant (used already) and
  `Config(String)`. Confirm the exact variant names before use.
- `src/pipeline.rs:19-31` — `ModelRef::resolve` calls `download_model(id, &default_cache_dir())` under
  `#[cfg(feature="download")]`.
- `tests/models.rs` — pure `model_url`/`cached_path` tests + an `#[ignore]`d network download test.
- Convention: no `unsafe`, no panics; return typed `WhisperError`.

## Commands you will need
| Purpose | Command | Expected |
|---|---|---|
| Tests (download feature) | `cargo test --features download` | pure tests pass; network test ignored |
| Build no features | `cargo build --no-default-features` | exit 0 |
| Lint | `cargo clippy --all-targets --features download -- -D warnings` | exit 0 |

## Scope
**In scope:** `src/models/mod.rs`, `tests/models.rs`, and (only if a new error variant is needed) `src/error.rs`.
**Out of scope:** `ureq` version/config, `src/ffi/**`, the diarization ONNX downloader (doesn't exist yet).
Do NOT change `default_cache_dir`'s value here (that's plan 010) — only its callers' safety.

## Git workflow
- Branch `advisor/005-harden-downloader`; message `fix(download): validate model id, detect truncation, add checksum hook`. Do NOT push.

## Steps
### Step 1: Validate the model id (fixes path traversal)
Add a private `fn validate_id(id: &str) -> Result<()>` that rejects any id not matching a strict allowlist:
non-empty, and every char in `[a-z0-9._-]` (lowercase letters, digits, dot, underscore, hyphen). Explicitly
reject ids containing `/`, `\`, `..`, or a leading `.`. On violation return
`WhisperError::Config(format!("invalid model id: {id:?}"))`. Call it at the top of BOTH `cached_path` and
`download_model` — but note `cached_path` currently returns `PathBuf`, not `Result`. Change `cached_path`'s
signature to `-> Result<PathBuf>` and update its callers (`download_model`, `tests/models.rs`, and any other).
If changing the public signature is undesirable, instead validate only in `download_model` and add a separate
`pub fn cached_path_checked(id, dir) -> Result<PathBuf>` — pick the signature-change option (cleaner) unless a
STOP condition arises.

**Verify**: `cargo build --features download` → exit 0.

### Step 2: Detect truncated downloads
In `download_model`, capture the response's `Content-Length` header if present (`resp.header("Content-Length")`
parsed to `u64`). Have `std::io::copy` return the byte count copied; after copy, if a Content-Length was
present and the copied bytes differ, delete the `.part` file and return
`WhisperError::ModelDownload(format!("truncated download: expected {expected} bytes, got {copied}"))`. Only
`rename` into place after this check passes. (If no Content-Length header, proceed — but see Step 3.)

**Verify**: `cargo build --features download` → exit 0.

### Step 3: Optional SHA-256 verification hook
Add an optional integrity path: a `pub fn download_model_verified(id: &str, cache_dir: &Path, expected_sha256: Option<&str>) -> Result<PathBuf>`
that behaves like `download_model` but, when `expected_sha256` is `Some`, computes the SHA-256 of the temp file
before rename and returns `WhisperError::ModelDownload("checksum mismatch …")` on mismatch. Implement SHA-256
with a small dependency ONLY if one is already in the tree; otherwise, keep this hook but document that the
digest arg is compared against a caller-provided value using the `sha2` crate — **and if adding `sha2` is
required, STOP and report** (adding a dependency is a maintainer decision). Fallback that needs no new dep:
implement Steps 1–2 fully and add `download_model_verified` that, when `expected_sha256` is `Some`, returns a
`WhisperError::Config("sha verification not yet wired — see plan 005")` so the API exists without a new dep.
Keep `download_model` as the un-verified convenience path.

**Verify**: `cargo build --features download` → exit 0.

### Step 4: Tests
In `tests/models.rs` add pure (offline) tests:
- `rejects_traversal_id`: `download_model("../../evil", &tmp)` (and `cached_path` if it now returns Result)
  returns `Err(WhisperError::Config(_))`; likewise ids with `/` and `..`.
- `accepts_valid_id`: `cached_path("tiny.en", Path::new("models"))` returns the expected `Ok(models/ggml-tiny.en.bin)`.
- Keep the existing `#[ignore]`d network test; update it if `cached_path`'s signature changed.

**Verify**: `cargo test --features download` → pure tests pass (network test ignored).

## Test plan
Tests in `tests/models.rs`: `rejects_traversal_id` (the security regression), `accepts_valid_id`, and the
existing url/path tests updated for any signature change. Truncation logic is hard to unit-test offline
without a mock server — cover it by construction + a code comment; do NOT add a network test that isn't
`#[ignore]`d. Verification: `cargo test --features download`.

## Done criteria
- [ ] `download_model` and `cached_path` reject ids containing `/`, `\`, or `..` with `WhisperError::Config`
- [ ] Truncation check present (Content-Length vs copied bytes) before `rename`
- [ ] `download_model_verified` exists (real SHA check OR documented stub per Step 3 — no unapproved new dep)
- [ ] `cargo test --features download` passes (incl. new traversal-rejection test); `cargo build --no-default-features` exit 0; clippy clean
- [ ] Only in-scope files modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions
- Implementing real SHA-256 requires adding a new crate (`sha2`) — STOP and report; do the documented-stub
  fallback instead and let the maintainer approve the dependency.
- Changing `cached_path` to return `Result` cascades into more call sites than `download_model` + tests —
  report the full list before proceeding.
- `ureq`'s API for reading a response header differs from `resp.header(...)` in the resolved version — report
  the actual API instead of guessing.

## Maintenance notes
- Pin `HF_BASE` to an immutable revision (a commit hash instead of `main`) when a known-good model revision is
  chosen — tracked as the supply-chain follow-up. A reviewer should confirm `validate_id` is called on every
  public entry that builds a path/URL from `id`.
- When the diarization ONNX downloader lands (BACKLOG P1), route it through the same `validate_id` + verify
  path.

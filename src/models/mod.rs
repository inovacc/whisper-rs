//! Public whisper GGML model downloader (feature = "download").
use crate::error::{Result, WhisperError};
use std::path::{Path, PathBuf};

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Validate a model id against a strict allowlist before it is used to build a filename or URL.
///
/// Accepts only non-empty strings composed of lowercase letters, digits, `.`, `_`, and `-`, and
/// explicitly rejects ids containing `/`, `\`, `..`, or a leading `.` — these are the shapes that
/// would otherwise let a caller-supplied `id` escape `cache_dir` (path traversal).
fn validate_id(id: &str) -> Result<()> {
    let invalid = || WhisperError::Config(format!("invalid model id: {id:?}"));
    if id.is_empty() {
        return Err(invalid());
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.starts_with('.') {
        return Err(invalid());
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')) {
        return Err(invalid());
    }
    Ok(())
}

/// Build the download URL for a whisper GGML model id (e.g. "tiny.en", "base", "small.en", "medium").
pub fn model_url(id: &str) -> String {
    format!("{HF_BASE}/ggml-{id}.bin")
}

/// The cached file path for a model id under `cache_dir`.
///
/// Returns `Err(WhisperError::Config(_))` if `id` fails [`validate_id`] (e.g. contains `/`, `\`,
/// or `..`), which would otherwise let the resulting path escape `cache_dir`.
pub fn cached_path(id: &str, cache_dir: &Path) -> Result<PathBuf> {
    validate_id(id)?;
    Ok(cache_dir.join(format!("ggml-{id}.bin")))
}

/// Download the model if not already cached; return the local path.
/// If the file already exists in `cache_dir`, returns it without downloading.
pub fn download_model(id: &str, cache_dir: &Path) -> Result<PathBuf> {
    validate_id(id)?;
    let dest = cached_path(id, cache_dir)?;
    if dest.exists() {
        return Ok(dest);
    }
    std::fs::create_dir_all(cache_dir)?;
    let url = model_url(id);
    let resp = ureq::get(&url).call().map_err(|e| WhisperError::ModelDownload(format!("GET {url}: {e}")))?;
    let expected_len: Option<u64> = resp.header("Content-Length").and_then(|v| v.parse::<u64>().ok());
    let mut reader = resp.into_reader();
    // Download to a temp file then rename (atomic-ish) so a partial download isn't mistaken for complete.
    let tmp = dest.with_extension("bin.part");
    let copied = {
        let mut f = std::fs::File::create(&tmp)?;
        std::io::copy(&mut reader, &mut f)?
    };
    if let Some(expected) = expected_len {
        if copied != expected {
            let _ = std::fs::remove_file(&tmp);
            return Err(WhisperError::ModelDownload(format!("truncated download: expected {expected} bytes, got {copied}")));
        }
    }
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Download the model like [`download_model`], optionally verifying its SHA-256 digest against
/// `expected_sha256` (lowercase hex) before it is cached.
///
/// SHA-256 verification is not yet wired up (it would require adding a new dependency, which is a
/// maintainer decision — see plan 005); passing `Some(_)` returns `WhisperError::Config` rather
/// than silently skipping the check. Passing `None` behaves exactly like [`download_model`].
pub fn download_model_verified(id: &str, cache_dir: &Path, expected_sha256: Option<&str>) -> Result<PathBuf> {
    if expected_sha256.is_some() {
        return Err(WhisperError::Config("sha verification not yet wired — see plan 005".into()));
    }
    download_model(id, cache_dir)
}

/// Default cache dir for downloaded models.
///
/// Resolution order (first that yields a value wins):
/// 1. `WHISPER_RS_CACHE_DIR` env var, used verbatim.
/// 2. A per-user cache dir joined with `whisper-rs/models`: `LOCALAPPDATA` (Windows),
///    `XDG_CACHE_HOME`, or `HOME/.cache` (Unix-like), in that order.
/// 3. `./models` under the current working directory, if none of the above resolve.
///
/// This intentionally avoids a new dependency (e.g. `dirs`) — see plan 010.
pub fn default_cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("WHISPER_RS_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(dir).join("whisper-rs").join("models");
    }
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("whisper-rs").join("models");
    }
    if let Some(dir) = std::env::var_os("HOME") {
        return PathBuf::from(dir).join(".cache").join("whisper-rs").join("models");
    }
    PathBuf::from("models")
}

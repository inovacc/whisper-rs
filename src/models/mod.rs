//! Public whisper GGML model downloader (feature = "download").
use crate::error::{Result, WhisperError};
use std::path::{Path, PathBuf};

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Build the download URL for a whisper GGML model id (e.g. "tiny.en", "base", "small.en", "medium").
pub fn model_url(id: &str) -> String {
    format!("{HF_BASE}/ggml-{id}.bin")
}

/// The cached file path for a model id under `cache_dir`.
pub fn cached_path(id: &str, cache_dir: &Path) -> PathBuf {
    cache_dir.join(format!("ggml-{id}.bin"))
}

/// Download the model if not already cached; return the local path.
/// If the file already exists in `cache_dir`, returns it without downloading.
pub fn download_model(id: &str, cache_dir: &Path) -> Result<PathBuf> {
    let dest = cached_path(id, cache_dir);
    if dest.exists() {
        return Ok(dest);
    }
    std::fs::create_dir_all(cache_dir)?;
    let url = model_url(id);
    let resp = ureq::get(&url)
        .call()
        .map_err(|e| WhisperError::ModelDownload(format!("GET {url}: {e}")))?;
    let mut reader = resp.into_reader();
    // Download to a temp file then rename (atomic-ish) so a partial download isn't mistaken for complete.
    let tmp = dest.with_extension("bin.part");
    {
        let mut f = std::fs::File::create(&tmp)?;
        std::io::copy(&mut reader, &mut f)?;
    }
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Default cache dir: `./models` under the current working dir.
pub fn default_cache_dir() -> PathBuf {
    PathBuf::from("models")
}

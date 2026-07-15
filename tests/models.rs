#![cfg(feature = "download")]
use std::path::Path;
use whisper_rs::error::WhisperError;
use whisper_rs::models::{cached_path, default_cache_dir, download_model, download_model_verified, model_url};

#[test]
fn url_and_path_are_correct() {
    assert_eq!(model_url("tiny.en"), "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin");
    assert_eq!(cached_path("base", Path::new("models")).unwrap(), Path::new("models/ggml-base.bin"));
}

#[test]
fn accepts_valid_id() {
    assert_eq!(cached_path("tiny.en", Path::new("models")).unwrap(), Path::new("models/ggml-tiny.en.bin"));
}

#[test]
fn rejects_traversal_id() {
    let tmp = std::env::temp_dir().join("whisper_rs_traversal_test");

    for bad in ["../../evil", "foo/../../etc/x", "a/b", "a\\b", "..", ".hidden"] {
        match cached_path(bad, Path::new("models")) {
            Err(WhisperError::Config(_)) => {}
            other => panic!("cached_path({bad:?}) expected Config error, got {other:?}"),
        }
        match download_model(bad, &tmp) {
            Err(WhisperError::Config(_)) => {}
            other => panic!("download_model({bad:?}) expected Config error, got {other:?}"),
        }
    }
}

#[test]
fn env_override_sets_cache_dir() {
    // Self-contained: save/restore the var so this test doesn't leak state to others.
    let saved = std::env::var_os("WHISPER_RS_CACHE_DIR");
    let want = std::env::temp_dir().join("whisper_rs_cache_dir_override_test");
    unsafe {
        std::env::set_var("WHISPER_RS_CACHE_DIR", &want);
    }
    let got = default_cache_dir();
    unsafe {
        match &saved {
            Some(v) => std::env::set_var("WHISPER_RS_CACHE_DIR", v),
            None => std::env::remove_var("WHISPER_RS_CACHE_DIR"),
        }
    }
    assert_eq!(got, want);
}

#[test]
fn cache_hit_returns_without_network() {
    let dir = std::env::temp_dir().join("whisper_rs_cache_hit_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = cached_path("tiny.en", &dir).unwrap();
    std::fs::write(&path, b"fake cached model bytes").unwrap();

    let got = download_model("tiny.en", &dir).unwrap();
    assert_eq!(got, path);
}

#[test]
fn verified_cache_hit_checks_sha256() {
    let dir = std::env::temp_dir().join("whisper_rs_sha_verify_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = cached_path("tiny.en", &dir).unwrap();
    std::fs::write(&path, b"hello").unwrap();
    // SHA-256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
    let good = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    // Correct digest on a cache hit -> Ok, no network.
    assert_eq!(download_model_verified("tiny.en", &dir, Some(good)).unwrap(), path);
    // Upper-case hex still matches (case-insensitive).
    assert!(download_model_verified("tiny.en", &dir, Some(&good.to_uppercase())).is_ok());
    // Wrong digest -> ModelDownload error.
    match download_model_verified("tiny.en", &dir, Some("deadbeef")) {
        Err(WhisperError::ModelDownload(_)) => {}
        other => panic!("expected checksum mismatch, got {other:?}"),
    }
    // None digest behaves like download_model (cache hit).
    assert_eq!(download_model_verified("tiny.en", &dir, None).unwrap(), path);

    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "network: downloads ~75MB tiny.en model"]
fn download_tiny_en_fetches_file() {
    let dir = std::env::temp_dir().join("whisper_rs_dl_test");
    let p = whisper_rs::models::download_model("tiny.en", &dir).unwrap();
    assert!(p.exists());
    assert!(std::fs::metadata(&p).unwrap().len() > 1_000_000);
}

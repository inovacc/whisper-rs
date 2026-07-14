#![cfg(feature = "download")]
use std::path::Path;
use whisper_rs::error::WhisperError;
use whisper_rs::models::{cached_path, download_model, model_url};

#[test]
fn url_and_path_are_correct() {
    assert_eq!(
        model_url("tiny.en"),
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
    );
    assert_eq!(
        cached_path("base", Path::new("models")).unwrap(),
        Path::new("models/ggml-base.bin")
    );
}

#[test]
fn accepts_valid_id() {
    assert_eq!(
        cached_path("tiny.en", Path::new("models")).unwrap(),
        Path::new("models/ggml-tiny.en.bin")
    );
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
#[ignore = "network: downloads ~75MB tiny.en model"]
fn download_tiny_en_fetches_file() {
    let dir = std::env::temp_dir().join("whisper_rs_dl_test");
    let p = whisper_rs::models::download_model("tiny.en", &dir).unwrap();
    assert!(p.exists());
    assert!(std::fs::metadata(&p).unwrap().len() > 1_000_000);
}

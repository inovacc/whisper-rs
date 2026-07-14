#![cfg(feature = "download")]
use std::path::Path;
use whisper_rs::models::{cached_path, model_url};

#[test]
fn url_and_path_are_correct() {
    assert_eq!(
        model_url("tiny.en"),
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
    );
    assert_eq!(
        cached_path("base", Path::new("models")),
        Path::new("models/ggml-base.bin")
    );
}

#[test]
#[ignore = "network: downloads ~75MB tiny.en model"]
fn download_tiny_en_fetches_file() {
    let dir = std::env::temp_dir().join("whisper_rs_dl_test");
    let p = whisper_rs::models::download_model("tiny.en", &dir).unwrap();
    assert!(p.exists());
    assert!(std::fs::metadata(&p).unwrap().len() > 1_000_000);
}

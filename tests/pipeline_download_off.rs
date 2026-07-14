#![cfg(not(feature = "download"))]
use whisper_rs::pipeline::{ModelRef, Pipeline};

#[test]
fn download_without_feature_is_config_error() {
    let err = Pipeline::builder().whisper_model(ModelRef::download("tiny.en")).build();
    assert!(matches!(err, Err(whisper_rs::WhisperError::Config(_))));
}

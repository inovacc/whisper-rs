use whisper_rs::pipeline::{ModelRef, Pipeline};
use whisper_rs::postprocess::PostConfig;

#[test]
fn postconfig_apply_runs_enabled_transforms() {
    let cfg = PostConfig::all("en");
    // "um three three three cats" -> fillers: "three three three cats" -> repeats: "three cats" -> numbers: "3 cats"
    assert_eq!(cfg.apply("um three three three cats"), "3 cats");
    // default config is a no-op
    assert_eq!(PostConfig::default().apply("um uh three"), "um uh three");
}

#[test]
fn builder_accepts_postprocess() {
    // build() will fail on missing model, but the postprocess() method must exist and chain.
    let err = Pipeline::builder()
        .postprocess(PostConfig::all("en"))
        .whisper_model(ModelRef::path("nope.bin"))
        .build();
    assert!(err.is_err()); // model missing -> error; the point is postprocess() compiles + chains
}

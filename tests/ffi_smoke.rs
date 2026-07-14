#[test]
fn links_and_reports_system_info() {
    let info = whisper_rs::ffi::system_info();
    assert!(info.contains("="), "expected capability report, got: {info:?}");
}

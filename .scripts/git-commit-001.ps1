Set-Location "D:\new_page\whisper-rs"
git add src/stream/mod.rs src/stream/local_agreement.rs src/stream/two_pass.rs src/stream/session.rs src/asr/mod.rs tests/stream_session.rs
git commit -m @'
fix(stream): flush tail on finalize; make session testable

- Add StreamPolicy::observe_final (defaulted) so end-of-stream commits the tail.
- Route StreamSession::finalize() through observe_final (was identical to poll).
- LocalAgreement2/TwoPass: override observe_final to commit uncommitted tail.
- Add committed_from to Committed; time committed segments from the committed
  token slice (clamped) instead of the whole buffer.
- Introduce Transcribe trait seam; make StreamSession<T: Transcribe = Transcriber>.
- Add 5 offline unit tests with a fake transcriber (finalize/commit regressions).
'@
git --no-pager log --oneline -1

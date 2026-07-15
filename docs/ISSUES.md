# Known Issues & Limitations — whisper-rs
<!-- rev:003 -->

Tracks known limitations, caveats, and external blockers. For planned work see `docs/ROADMAP.md`;
for prioritized follow-ups see `docs/BACKLOG.md`.

## Limitations (by design or upstream)

- **Overlapping / simultaneous speech is not handled** (v1, documented in the design spec). Diarization
  degrades without clear speaker pauses. Out of scope for v1; a best-effort or stereo-split mitigation is
  a backlog item (BACKLOG P4).
- **Word timestamps are per-token, not DTW-aligned.** The `timestamps` module surfaces whisper.cpp's
  `token_timestamps`, not the higher-accuracy DTW (`dtw_token_timestamps`/`dtw_aheads`) path. Accurate
  enough for most uses; DTW wiring is an optional refinement (BACKLOG P2.5).
- **Batch transcription is single-threaded per `Transcriber`.** whisper.cpp state is not `Sync`; use one
  `Transcriber`/`Pipeline` per thread. This is enforced by the type system (`Context: Send`, not `Sync`).
- **The default build decodes WAV (PCM int/float) only.** Other containers/codecs (m4a, mp3, ogg, …)
  are supported via the opt-in `ffmpeg` feature (`Pipeline::transcribe_media_file` /
  `audio::media::decode_to_mono_16k`, needs ffmpeg 8.x shared+dev libs). A pure-Rust `symphonia`
  decoder (no native deps) remains a backlog alternative (BACKLOG P5).
- **Long audio guard:** inputs over ~i32::MAX samples (~37 h @ 16 kHz) are rejected with a typed
  `Config` error rather than silently truncated.

## External blockers

- **Diarization ONNX models are HuggingFace-gated (BLOCKER, BACKLOG P1).** `pyannote-segmentation-3.0`
  and speaker-embedding models require a human to accept the model licenses on HuggingFace and download
  the `.onnx` files into `models/`. The Phase 2 diarization plan's ONNX inference tasks (3–5) and their
  tests stay `#[ignore]`d until these are present. The model-independent diarization core (types,
  timeline-merge, clustering) is implemented and tested.
- **`ort` (ONNX Runtime) is a pre-release rc.** When the diarization inference tasks land, `ort` must be
  pinned exactly and an onnxruntime shared library resolved for the target platform — an API-stability
  and packaging risk (design spec, known limitations).

## Environment notes

- **Build requires `libclang`** (for `bindgen`) + a C/C++ toolchain; on Windows also MSVC Build Tools.
  The whisper.cpp submodule must be initialized (`git submodule update --init --recursive`). See
  `README.md` / `AGENTS.md`.

## Resolved

- **2026-07-14 — CI now builds a 3-OS matrix.** `.github/workflows/ci.yml` runs
  `[ubuntu-latest, macos-latest, windows-latest]`; the prior single-OS build limitation no longer
  applies.

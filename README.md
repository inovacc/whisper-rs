# whisper-rs
<!-- rev:003 -->

A feature-rich, safe Rust wrapper over [whisper.cpp](https://github.com/ggml-org/whisper.cpp) — local, offline speech-to-text with word-level timestamps.

## Status

**Beyond foundation.** What ships today: batch ASR over a WAV file, word-level timestamps with
monotonic enforcement, a high-level `Pipeline`, post-processing (number normalization, repeat
collapse, filler removal, hallucination flagging), audio preprocessing (levels 0–4 + energy VAD),
a real HTTPS model downloader, and the pure/core slices of diarization (types, timeline merge,
agglomerative clustering) and streaming (`LocalAgreement2`/`TwoPass` policies, `StreamSession`).
**Still blocked:** ONNX-model-backed diarization inference (`pyannote-segmentation-3.0` + speaker
embeddings) and a Silero ONNX VAD upgrade — both need HuggingFace-gated models a maintainer must
accept and place under `models/`. See `docs/ROADMAP.md` for the phased build order and exact status
per phase.

This crate is **local use only**: `publish = false` in `Cargo.toml`, not published to crates.io.

## Features

Works today:
- Decode a WAV file (16-bit PCM or float) and normalize to whisper.cpp's required mono 16 kHz `f32` PCM.
- Batch transcription via whisper.cpp (`Transcriber`), single-shot per audio buffer.
- Word-level timestamps derived from whisper token data, with monotonicity enforcement
  (`timestamps::words_from_tokens`, `timestamps::enforce_monotonic`).
- A high-level `Pipeline` that composes decode → resample → transcribe into one call.
- Structured output types (`Transcript`, `Segment`, `Word`) with a `plain_text()` convenience.
- `download` — `ModelRef::download(id)` fetches a GGML model over HTTPS via `ureq` and caches it
  locally (behind `feature = "download"`, enabled by default).
- `diarization` (core) — `SpeakerTurn`/`DiarizeConfig` types, `merge(words, turns)` timeline join,
  and agglomerative speaker clustering — all pure and tested. Model-backed inference is not wired
  into `Pipeline` yet (blocked on gated ONNX models).
- `streaming` (core) — `StreamPolicy` (`LocalAgreement2`, `TwoPass`) and a synchronous
  `StreamSession` (push/poll/finalize) plus `Pipeline::into_stream`. VAD-boundary incremental
  decoding and a worker-thread/mic-capture variant are still planned.

### Post-processing & preprocessing

- `postprocess::normalize_numbers`, `postprocess::collapse_repeats`, `postprocess::remove_fillers`,
  `postprocess::PostConfig` — text-level cleanup, wired into `Pipeline`.
- `postprocess::hallucination` — cross-pass comparison heuristic + `apply_flags`.
- `audio::preprocess::{preprocess, remove_dc, normalize_peak, noise_gate}` with `PreprocessLevel`
  (levels 0–4, the Galle scheme) — wired into `Pipeline`.
- `audio::vad::segment` — pure energy-based VAD (`VadConfig`). A Silero ONNX upgrade is planned but
  blocked on a gated model.

### Still blocked (needs HuggingFace-gated ONNX models)

- Diarization's `ort` + `pyannote-segmentation-3.0` segmentation inference and speaker-embedding
  inference — the pure clustering/merge core above works today, but `Pipeline::diarization(cfg)`
  isn't wired up until this lands.
- Silero ONNX VAD upgrade (shares the diarization `ort` dependency).

## Build requirements

- Rust 1.86+ (MSRV, edition 2021) — the `ureq`/`url`/`idna`/`icu` chain requires rustc ≥ 1.86.
- **`libclang`** — required by `bindgen` to generate FFI bindings from whisper.cpp's C header.
- A C/C++ toolchain (whisper.cpp/ggml are compiled from source via `cc`).
  - **Windows:** MSVC Build Tools (the `build.rs` detects the `-msvc` target and adjusts flags/links
    `advapi32` accordingly).
  - Linux/macOS: a working `cc`/`clang` install; `-pthread` is passed on non-MSVC targets.
- The `vendor/whisper.cpp` git submodule must be checked out — it's the actual C++ source `build.rs`
  compiles (whisper.cpp core + ggml CPU backend), pinned at v1.7.4.

## Install

Not published to crates.io. Use as a path or git dependency:

```toml
[dependencies]
whisper-rs = { path = "../whisper-rs" }
```

Then initialize the vendored submodule before building:

```powershell
git submodule update --init --recursive
cargo build
```

## Usage

### High-level: `Pipeline`

```no_run
use whisper_rs::prelude::*;

fn main() -> whisper_rs::Result<()> {
    let mut pipeline = Pipeline::builder()
        .whisper_model(ModelRef::path("models/ggml-tiny.en.bin"))
        .language(Some("en".into()))
        .build()?;

    let transcript = pipeline.transcribe_file("audio.wav")?;
    println!("{}", transcript.plain_text());
    for segment in &transcript.segments {
        println!("[{:.2}-{:.2}] {}", segment.start, segment.end, segment.text);
    }
    Ok(())
}
```

### Composable stages

```no_run
use whisper_rs::asr::{AsrOptions, Transcriber};
use whisper_rs::audio::AudioInput;

fn main() -> whisper_rs::Result<()> {
    let pcm = AudioInput::from_wav_file("audio.wav")?.to_mono_16k()?;

    let mut transcriber = Transcriber::from_model_file("models/ggml-tiny.en.bin")?;
    let segments = transcriber.transcribe(&pcm, &AsrOptions::default())?;

    for segment in &segments {
        for word in &segment.words {
            println!("{} [{:.2}-{:.2}] ({:.2})", word.text, word.start, word.end, word.confidence);
        }
    }
    Ok(())
}
```

## Models

Models are **not bundled** — get a GGML model either via whisper.cpp's own tooling:

```powershell
# from a whisper.cpp checkout
./models/download-ggml-model.sh tiny.en
```

and point `ModelRef::path(...)` (or `Transcriber::from_model_file(...)`) at the resulting
`ggml-*.bin` file; or, with `feature = "download"` (default), call `ModelRef::download(id)` to fetch
and cache a GGML model over HTTPS via `ureq` directly. Without the `download` feature enabled,
`ModelRef::download(...)` returns `WhisperError::Config`.

## Audio input

WAV files today, 16-bit integer or 32-bit float PCM, any channel count / sample rate — `AudioInput`
downmixes to mono and resamples to 16 kHz via `rubato` internally.

## Testing

```powershell
cargo test
```

Model-gated tests (anything that needs a real GGML model + audio fixture) are `#[ignore]`d by
default so `cargo test` works without extra setup. To run them, place a model at
`models/ggml-tiny.en.bin` and run:

```powershell
cargo test -- --ignored
```

## License

BSD-3-Clause.

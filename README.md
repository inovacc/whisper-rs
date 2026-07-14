# whisper-rs
<!-- rev:001 -->

A feature-rich, safe Rust wrapper over [whisper.cpp](https://github.com/ggml-org/whisper.cpp) — local, offline speech-to-text with word-level timestamps.

## Status

**Foundation.** What ships today: batch ASR over a WAV file, word-level timestamps with monotonic
enforcement, and a high-level `Pipeline`. **Diarization, streaming, audio-preprocessing polish, and
a model downloader are planned but not implemented** — their feature flags exist and are wired into
`Cargo.toml`, but the code behind them is empty stubs (or, for the downloader, an explicit
`WhisperError::Config` error). See `docs/ROADMAP.md` for the phased build order.

This crate is **local use only**: `publish = false` in `Cargo.toml`, not published to crates.io.

## Features

Works today:
- Decode a WAV file (16-bit PCM or float) and normalize to whisper.cpp's required mono 16 kHz `f32` PCM.
- Batch transcription via whisper.cpp (`Transcriber`), single-shot per audio buffer.
- Word-level timestamps derived from whisper token data, with monotonicity enforcement
  (`timestamps::words_from_tokens`, `timestamps::enforce_monotonic`).
- A high-level `Pipeline` that composes decode → resample → transcribe into one call.
- Structured output types (`Transcript`, `Segment`, `Word`) with a `plain_text()` convenience.

Planned, not yet implemented (feature-flagged, empty today):
- `diarization` — speaker attribution (`Segment::speaker` exists in the type but is always `None`).
- `streaming` — incremental/live transcription.
- `download` — `ModelRef::download(...)` currently always returns `WhisperError::Config`; models
  must be supplied by local path.

## Build requirements

- Rust 1.75+ (MSRV, edition 2021).
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

Models are **bring-your-own** — this crate does not bundle or download them today. Get a GGML model
from whisper.cpp's own tooling, e.g.:

```powershell
# from a whisper.cpp checkout
./models/download-ggml-model.sh tiny.en
```

Then point `ModelRef::path(...)` (or `Transcriber::from_model_file(...)`) at the resulting
`ggml-*.bin` file. `ModelRef::download(...)` exists in the API surface but currently always returns
`WhisperError::Config` — automatic downloading is a later phase.

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

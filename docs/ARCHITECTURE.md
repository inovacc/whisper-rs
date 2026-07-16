# Architecture — whisper-rs
<!-- rev:001 -->

`whisper-rs` is a **layered, safe-by-construction** wrapper over whisper.cpp. Each layer is a small,
focused module; the high-level `Pipeline` is a thin composition of the lower stages, not a place for
its own logic. `unsafe` is confined to a single module (`src/ffi/`) — see
[ADR-0001](adr/0001-unsafe-confined-to-ffi.md).

## Layer overview

```mermaid
flowchart TB
    subgraph consumer["Consumer API"]
        prelude["prelude — convenience re-exports"]
        pipeline["pipeline::Pipeline — one-call transcription"]
    end
    subgraph stages["Composable stages (safe Rust)"]
        audio["audio — WAV/ffmpeg decode, downmix, 16 kHz resample, preprocess, VAD"]
        asr["asr::Transcriber — full() over FFI → Segment/Word"]
        timestamps["timestamps — word times + monotonic enforcement"]
        postprocess["postprocess — numbers, repeats, fillers, hallucination"]
        output["output — Transcript/Segment/Word, SRT/VTT"]
        diarize["diarize — merge + agglomerative clustering (pure core)"]
        stream["stream — StreamPolicy + StreamSession"]
        models["models — HTTPS downloader + cache (feature=download)"]
    end
    subgraph boundary["Unsafe boundary"]
        ffi["ffi — bindgen bindings + RAII Context (the ONLY unsafe module)"]
    end
    whispercpp["vendor/whisper.cpp (v1.7.4) + ggml CPU backend — compiled by build.rs"]

    pipeline --> audio --> asr --> timestamps
    pipeline --> postprocess --> output
    asr --> output
    pipeline -.optional.-> models
    pipeline -.feature=streaming.-> stream
    stream --> asr
    diarize --> output
    asr --> ffi --> whispercpp
    trace["trace — zero-cost tracing facade (feature=tracing)"] -.instruments.-> asr
    trace -.instruments.-> models
```

## Request flow — `Pipeline::transcribe_file`

```mermaid
sequenceDiagram
    participant C as Consumer
    participant P as Pipeline
    participant A as audio
    participant T as asr::Transcriber
    participant F as ffi::Context
    participant W as whisper.cpp

    C->>P: transcribe_file(path)
    P->>A: from_wav_file → to_mono_16k (downmix + resample)
    A-->>P: Vec<f32> @ 16 kHz mono
    P->>A: preprocess(level)
    P->>T: transcribe(pcm, opts)
    T->>F: full(lang, threads, pcm)
    F->>W: whisper_full(...) [unsafe]
    W-->>F: segments + tokens
    F-->>T: n_segments / segment_text / tokens
    T->>T: timestamps::words_for_segment (monotonic)
    T-->>P: Vec<Segment>
    P->>P: postprocess.apply (optional)
    P-->>C: Transcript
```

## Key invariants

- **`unsafe` only in `src/ffi/`** ([ADR-0001](adr/0001-unsafe-confined-to-ffi.md)). Every other module
  is safe Rust; `ffi::Context` is the RAII owner of the whisper.cpp state pointer.
- **One crate-wide error type**, `WhisperError` → `Result<T>`
  ([ADR-0002](adr/0002-single-error-type.md)). It is `#[non_exhaustive]`.
- **Optional capabilities are Cargo features**, off-by-default where they add native deps
  ([ADR-0003](adr/0003-feature-gated-optional-capabilities.md)): `download`, `diarization`,
  `streaming` (on by default), `ffmpeg`, `raw-api`, `tracing` (opt-in).
- **`Transcriber` is `Send` but not `Sync`** — whisper.cpp state is single-threaded; use one per thread.

## Build

`build.rs` compiles `vendor/whisper.cpp` (pinned v1.7.4 — core + ggml CPU backend) via `cc`: the `.c`
sources compile as C (with `_GNU_SOURCE` for glibc affinity symbols on non-MSVC), the `.cpp` sources as
C++17, folded into one static archive. It then runs `bindgen` over `wrapper.h` to generate
`OUT_DIR/bindings.rs`, consumed only by `src/ffi/`.

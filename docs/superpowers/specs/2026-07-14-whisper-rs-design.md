# Design: whisper-rs — Feature-Enriched whisper.cpp Wrapper

**Status:** APPROVED — decisions made interactively with the user via `/discover` →
`superpowers:brainstorming` on 2026-07-14. Supersedes the earlier auto-generated DRAFT of the
same date.

**Scope:** A single local-use Rust crate (`whisper-rs`, not published to crates.io) that wraps
`whisper.cpp` behind a safe, ergonomic, feature-rich API: core ASR + audio preprocessing +
word/segment timestamps + speaker diarization + real-time streaming + structured output, all in
v1.

Grounded in `docs/discovery/IDEA-BRIEF.md` (deep research: 13 reference URLs + competitive
teardown of tazz4843/whisper-rs, WhisperX, whisper-diarization). Market gap targeted: no Rust or
Python tool ships batteries-included real-time diarized transcription with ergonomic word
timestamps.

## Locked decisions

| # | Decision | Choice |
|---|----------|--------|
| 1 | API shape | Layered — composable stages + a high-level `Pipeline` on top |
| 2 | Cargo features | Feature-gated modules, all in the `default` set (feature-rich out of the box, opt-out to slim) |
| 3 | Diarization backend | ONNX pyannote via `ort` (`pyannote-segmentation-3.0` + embeddings). Stereo channel-split fast-path deferred to backlog |
| 4 | Streaming policy | `StreamPolicy` trait with two impls — LocalAgreement-2 and two-pass tiny/medium — both configurable |
| 5 | v1 stretch features | Hallucination flagging + number normalization IN v1; DER-metrics hooks + multi-mic DOA/TDOA DEFERRED |
| 6 | Model management | Local paths by default + opt-in downloader/cache behind the `download` feature |
| 7 | Overlapping speech | Out of scope for v1 — documented known limitation |
| 8 | Crate structure | Single crate, `unsafe` quarantined behind an internal `ffi` module |

## Architecture & module layout

Single crate, one focused module per concern; `unsafe` lives only in `ffi`.

```
whisper-rs/
├── build.rs                 # bindgen + cc: compile & bind whisper.cpp (vendored/submodule)
├── Cargo.toml               # feature flags (below)
└── src/
    ├── lib.rs               # crate root, re-exports, top-level docs
    ├── ffi/                 # ONLY place with `unsafe` — raw whisper.cpp bindings + RAII wrappers
    ├── error.rs             # WhisperError (thiserror) — one crate-wide error enum
    ├── audio/               # decode, resample→16kHz mono (rubato), stereo→mono downmix, VAD
    │                        #   (silero), tiered preprocessing levels 0–4 (Galle scheme)
    ├── asr/                 # core transcription over ffi
    ├── timestamps/          # DTW word/segment timestamps (safely surfaces whisper.cpp t_dtw)
    ├── diarize/             # [feature] ort + pyannote-segmentation-3.0 + embeddings + clustering
    ├── stream/              # [feature] StreamPolicy trait + LocalAgreement2 + TwoPass impls
    ├── postprocess/         # hallucination flagging + number normalization
    ├── models/              # path resolution + [feature] optional downloader/cache
    ├── output.rs            # Transcript / Segment / Word / SpeakerId structured types
    ├── pipeline.rs          # high-level Pipeline + PipelineBuilder (layer over the stages)
    └── prelude.rs           # convenient re-exports
```

**Cargo features** (`default` = feature-rich; opt-out to slim):
- `diarization` → `ort` + ONNX models (the heavy dependency)
- `streaming` → `cpal` (mic capture) + streaming machinery
- `download` → optional model downloader/cache (network)
- `default = ["diarization", "streaming", "download"]`

**Key dependencies:** `bindgen`/`cc` (build), `rubato` (resample), `hound`/`symphonia` (file
I/O), `ort` (ONNX diarization — pinned to the pre-release rc, tracked as a risk), `cpal` (mic),
`tokio` (streaming glue), `thiserror` (errors).

## Public API — the layered surface

**Lower layer — composable stages** (each independently usable/testable):

```rust
let pcm = AudioInput::from_file("call.wav")?.to_mono_16k()?;
let segments = Vad::new(cfg).segment(&pcm)?;

let asr = Transcriber::new(&model)?;
let raw = asr.transcribe(&pcm, &opts)?;
let timed = timestamps::align(&raw, &pcm)?;

let turns = Diarizer::new(&dia_models)?.diarize(&pcm)?;   // feature = "diarization"
let transcript = merge(timed, turns);
```

**Upper layer — high-level `Pipeline`** (builder-configured, wraps the stages):

```rust
let pipeline = Pipeline::builder()
    .whisper_model(ModelRef::path("ggml-medium.bin"))    // or ModelRef::download(Model::Medium)
    .preprocess(PreprocessLevel::L2)                      // Galle's 0–4 tiers
    .diarization(DiarizeConfig::default())               // omit → no diarization
    .postprocess(Post::HALLUCINATION | Post::NUMBERS)
    .build()?;

let transcript: Transcript = pipeline.transcribe_file("call.wav")?;      // batch

let mut stream = pipeline.stream(StreamPolicy::local_agreement_2());     // or ::two_pass(...)
stream.push(audio_chunk)?;
for event in stream.poll() { /* PartialText | CommittedSegment | SpeakerTurn | Error */ }
```

**Output type** — one analytics-ready structure:

```rust
struct Transcript { segments: Vec<Segment> }
struct Segment { speaker: Option<SpeakerId>, text: String, start: f32, end: f32,
                 words: Vec<Word>, flags: SegmentFlags /* hallucination_suspect, … */ }
struct Word { text: String, start: f32, end: f32, confidence: f32 }
```

## Data flow

**Batch** (`pipeline.transcribe_file()`):

```
audio file/bytes
  → audio::decode (symphonia/hound)
  → audio::to_mono_16k (rubato)   # v1: downmix stereo→mono (per-channel diarization fast-path deferred)
  → audio::preprocess (level 0–4)
  → audio::vad (silero) → speech segments
  → asr::transcribe (whisper.cpp via ffi) → text + segments
  → timestamps::align (DTW) → word timings
  → diarize (ort/pyannote) → speaker turns          [feature: diarization]
  → merge(words, turns) → Transcript
  → postprocess (hallucination flags, number normalization)
  → Transcript
```

**Streaming** (`pipeline.stream()`):

```
mic (cpal) / pushed chunks
  → ring buffer + vad (boundary-driven, not fixed windows)
  → StreamPolicy (LocalAgreement2 | TwoPass) governs when text is committed
  → emits: PartialText → CommittedSegment → (optional) SpeakerTurn → Error
  → incremental diarization on committed audio; degrades without clear pauses (documented)
```

**Model flow:** `ModelRef` resolves to a local path — supplied directly, or (feature `download`)
fetched to a cache dir on first use. Diarization needs two ONNX models (segmentation + embedding);
whisper needs one GGML model. Missing/gated model → typed error, never a panic.

**Concurrency:** batch is synchronous per call (whisper.cpp state is not `Sync` — one
`Transcriber` per thread); streaming runs a `tokio` task with channels between capture →
inference → event consumer.

## Error handling

One crate-wide `WhisperError` (`thiserror`); no panics on expected-failure paths.

```rust
pub enum WhisperError {
    Io(std::io::Error),
    AudioDecode(String),
    Resample(String),
    ModelNotFound { kind: ModelKind, path: PathBuf },
    ModelDownload(String),   // feature "download"
    Ffi(i32),                // non-zero whisper.cpp return codes
    Onnx(String),            // feature "diarization"
    Vad(String),
    Config(String),          // invalid builder combination, caught at build()
}
pub type Result<T> = std::result::Result<T, WhisperError>;
```

- `ffi` converts every whisper.cpp status code into `WhisperError::Ffi`; raw codes never escape.
- `PipelineBuilder::build()` validates config up front (e.g. diarization requested but feature
  disabled → `Config` error, not a late failure).
- Absent/gated models are typed errors carrying the offending path, so callers can prompt/download.
- Streaming transient inference errors surface as an `Error` event, not a task-killing panic.

## Testing strategy

- **Unit tests per stage** — `audio` (resample rate/length invariants, channel split),
  `timestamps` (monotonic non-overlapping word times), `postprocess` (number-normalization table,
  hallucination heuristic on known inputs), `stream` policies (LocalAgreement-2 commit logic on
  synthetic inference sequences — no model needed).
- **Golden fixtures** — a short bundled WAV (few seconds, speech + a clear speaker change) → assert
  transcript text (fuzzy), word-timestamp ordering, ≥2 detected speakers. Small enough for CI
  without a GPU.
- **Feature-matrix build** — CI compiles `--no-default-features`, each feature alone, and
  `--all-features` so opt-out actually works.
- **FFI smoke test** — load a tiny GGML model, transcribe silence + a known clip, assert no
  leak/crash (validates `build.rs` + bindings).
- **Model-gated tests** — diarization/download tests needing large or HF-gated models are
  `#[ignore]`d by default, run when models are present.
- **TDD order** (see the implementation plan): stage units → pipeline integration → streaming.

## Known limitations (v1)

- **Overlapping/simultaneous speech** is not handled — documented limitation; diarization degrades
  without clear speaker pauses.
- **`ort` is pre-release** (2.0.0-rc.x) — pinned and tracked; an API-stability risk.
- **whisper.cpp tinydiarize is not used** — it is turn-detection only, not real diarization; do not
  conflate.

## Deferred to backlog (post-v1)

- Stereo channel-split diarization fast-path (near-zero clustering error for dual-channel audio).
- DER (Diarization Error Rate) metrics hooks.
- Multi-mic DOA/TDOA spatial diarization.
- Pure-Rust Burn-based reimplementation (whisper-burn) as an FFI alternative — only if whisper.cpp
  build friction becomes a real blocker.

## Open provenance notes

- The crate is local-use-only; the crates.io name collision with the existing `whisper-rs` is a
  non-issue by the user's decision.
- Reference material: `idea.txt` (goal + 13 URLs) and two saved articles (Galle pipeline, HTX-DSAI
  ASR readability). Full evidence in `docs/discovery/IDEA-BRIEF.md` and the discovery evidence files.

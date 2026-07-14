# whisper-rs v1 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational slice of the `whisper-rs` crate — batch transcription of an audio file into a structured, word-timestamped `Transcript`, over `whisper.cpp` via a safe FFI layer.

**Architecture:** Single crate; `unsafe` quarantined in an internal `ffi` module. `build.rs` compiles vendored whisper.cpp with `cc` and generates bindings with `bindgen`. Safe stages (`audio`, `asr`, `timestamps`) sit above `ffi`; a high-level `Pipeline` wires them. Diarization, streaming, post-processing, and the model downloader are later plans (feature-gated, out of scope here).

**Tech Stack:** Rust (edition 2021), `bindgen` + `cc` (build), `hound` (WAV I/O), `rubato` (resample), `thiserror` (errors). whisper.cpp vendored as a git submodule.

## Global Constraints

- Crate name: `whisper-rs`; local use only (NOT published to crates.io) — copied verbatim from spec decision #8.
- Rust edition: 2021. MSRV: 1.75+.
- `unsafe` code appears ONLY in `src/ffi/`. Every other module is `#![forbid(unsafe_code)]`-clean by convention.
- One crate-wide error type `WhisperError` (`thiserror`); no `panic!`/`unwrap`/`expect` on expected-failure paths (model missing, decode error, FFI non-zero code).
- Audio fed to whisper.cpp MUST be 16 kHz mono `f32` PCM — copied verbatim from spec data-flow.
- Feature set (declared now, modules land in later plans): `default = ["diarization", "streaming", "download"]`. This plan implements none of those features' bodies — only the always-on core.
- whisper.cpp C API names used here are the stable public API: `whisper_init_from_file_with_params`, `whisper_full_with_state`/`whisper_full`, `whisper_full_n_segments`, `whisper_full_get_segment_text`, `whisper_full_get_segment_t0/t1`, `whisper_full_n_tokens`, `whisper_full_get_token_data`, `whisper_free`, `whisper_print_system_info`.

---

### Task 1: Project scaffold + git

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src/lib.rs`
- Test: (build check only)

**Interfaces:**
- Consumes: nothing.
- Produces: a compiling empty crate named `whisper_rs` with the feature flags declared.

- [ ] **Step 1: Initialize git (folder is not yet a repo)**

Run:
```
git init
git config user.name  "<your name>"   # if not already globally set
git config user.email "<your email>"
```
Expected: `Initialized empty Git repository`.

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "whisper-rs"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
publish = false            # local use only (spec decision #8)
license = "BSD-3-Clause"

[features]
default = ["diarization", "streaming", "download"]
diarization = []           # bodies land in a later plan
streaming = []             # bodies land in a later plan
download = []              # bodies land in a later plan

[dependencies]
thiserror = "2"
hound = "3"
rubato = "0.15"

[build-dependencies]
bindgen = "0.70"
cc = "1"
```

- [ ] **Step 3: Write `.gitignore`**

```
/target
Cargo.lock          # library crate; do not pin for consumers
/models/*.bin
/models/*.onnx
```

- [ ] **Step 4: Write `src/lib.rs`**

```rust
//! whisper-rs — a feature-rich, safe Rust wrapper over whisper.cpp (local use).

pub mod error;

pub use error::{Result, WhisperError};
```

Create a placeholder `src/error.rs` so it compiles (real body in Task 3):
```rust
//! Crate-wide error type. Full definition in Task 3.
#[derive(Debug)]
pub struct WhisperError;
pub type Result<T> = std::result::Result<T, WhisperError>;
impl std::fmt::Display for WhisperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "WhisperError") }
}
impl std::error::Error for WhisperError {}
```

- [ ] **Step 5: Verify it builds**

Run: `cargo build`
Expected: compiles with no errors (warnings about unused are fine).

- [ ] **Step 6: Commit**

```
git add Cargo.toml .gitignore src/
git commit -m "chore: scaffold whisper-rs crate + git init"
```

---

### Task 2: Vendor whisper.cpp + build.rs + raw FFI bindings

**Files:**
- Create: `vendor/whisper.cpp` (git submodule)
- Create: `build.rs`
- Create: `src/ffi/mod.rs`
- Create: `wrapper.h`
- Test: `tests/ffi_smoke.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `ffi` module exposing the raw bindgen symbols (via `include!`) plus a safe helper `ffi::system_info() -> String`.

- [ ] **Step 1: Add whisper.cpp as a submodule**

Run:
```
git submodule add https://github.com/ggml-org/whisper.cpp vendor/whisper.cpp
git -C vendor/whisper.cpp checkout v1.7.4
```
Expected: submodule cloned at tag `v1.7.4` (pin a known tag for reproducibility).

- [ ] **Step 2: Write the bindgen header `wrapper.h`**

```c
#include "whisper.h"
```

- [ ] **Step 3: Write `build.rs`**

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let wcpp = root.join("vendor/whisper.cpp");

    // Compile ggml + whisper.cpp (C/C++). whisper.cpp ships its own sources under src/ and ggml/src/.
    cc::Build::new()
        .cpp(true)
        .include(wcpp.join("include"))
        .include(wcpp.join("ggml/include"))
        .include(wcpp.join("ggml/src"))
        .file(wcpp.join("src/whisper.cpp"))
        .file(wcpp.join("ggml/src/ggml.c"))
        .file(wcpp.join("ggml/src/ggml-alloc.c"))
        .file(wcpp.join("ggml/src/ggml-backend.cpp"))
        .file(wcpp.join("ggml/src/ggml-quants.c"))
        .flag_if_supported("-std=c++17")
        .warnings(false)
        .compile("whisper");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", wcpp.join("include").display()))
        .clang_arg(format!("-I{}", wcpp.join("ggml/include").display()))
        .allowlist_function("whisper_.*")
        .allowlist_type("whisper_.*")
        .allowlist_var("WHISPER_.*")
        .generate()
        .expect("bindgen failed to generate whisper.cpp bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out.join("bindings.rs")).expect("write bindings.rs");
}
```

> Note for the implementer: whisper.cpp's exact source file list can shift between tags. If a `.file(...)` path does not exist at `v1.7.4`, run `ls vendor/whisper.cpp/ggml/src` and adjust the file list to the actual `.c`/`.cpp` sources — do not add features, just make the listed set match what's on disk. This is expected build-glue tuning, not a design change.

- [ ] **Step 4: Write `src/ffi/mod.rs`**

```rust
//! Raw whisper.cpp FFI bindings. The ONLY module in this crate allowed to use `unsafe`.
#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::CStr;

/// Safe wrapper over `whisper_print_system_info` — proves the library links & calls without a model.
pub fn system_info() -> String {
    // SAFETY: whisper_print_system_info returns a pointer to a static, NUL-terminated C string.
    unsafe {
        let ptr = whisper_print_system_info();
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
```

Add `pub mod ffi;` to `src/lib.rs` (keep it `#[doc(hidden)]` so it isn't part of the public API):
```rust
#[doc(hidden)]
pub mod ffi;
```

- [ ] **Step 5: Write the failing smoke test `tests/ffi_smoke.rs`**

```rust
#[test]
fn links_and_reports_system_info() {
    let info = whisper_rs::ffi::system_info();
    // whisper_print_system_info always reports capability flags like "AVX = 0/1".
    assert!(info.contains("="), "expected capability report, got: {info:?}");
}
```

- [ ] **Step 6: Run it — verify it fails to compile first (no bindings yet if run before build)**

Run: `cargo test --test ffi_smoke`
Expected: on a clean tree this triggers `build.rs`; the FIRST failure to watch for is a build/link error if the source file list is wrong (fix per the Step 3 note). Once it links, the test should PASS.

- [ ] **Step 7: Make it pass**

Iterate the `build.rs` file list until `cargo test --test ffi_smoke` PASSES:
```
running 1 test
test links_and_reports_system_info ... ok
```

- [ ] **Step 8: Commit**

```
git add .gitmodules vendor/whisper.cpp build.rs wrapper.h src/ffi/ src/lib.rs tests/ffi_smoke.rs
git commit -m "feat(ffi): vendor whisper.cpp, add build.rs + raw bindings + smoke test"
```

---

### Task 3: Crate-wide error type

**Files:**
- Modify: `src/error.rs` (replace the Task 1 placeholder)
- Test: `tests/error.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `WhisperError` enum + `Result<T>` used by every later task. Variants: `Io`, `AudioDecode(String)`, `Resample(String)`, `ModelNotFound { kind: ModelKind, path: PathBuf }`, `Ffi(i32)`, `Config(String)`. (`ModelDownload`, `Onnx`, `Vad` variants are added by later plans.)

- [ ] **Step 1: Write the failing test `tests/error.rs`**

```rust
use std::path::PathBuf;
use whisper_rs::{WhisperError, error::ModelKind};

#[test]
fn model_not_found_displays_path_and_kind() {
    let e = WhisperError::ModelNotFound { kind: ModelKind::Whisper, path: PathBuf::from("/x/ggml.bin") };
    let s = e.to_string();
    assert!(s.contains("/x/ggml.bin"));
    assert!(s.contains("whisper"));
}

#[test]
fn io_error_converts_with_question_mark() {
    fn inner() -> whisper_rs::Result<()> {
        std::fs::File::open("/definitely/missing/file")?; // io::Error -> WhisperError via From
        Ok(())
    }
    assert!(matches!(inner(), Err(WhisperError::Io(_))));
}
```

- [ ] **Step 2: Run it — verify it fails**

Run: `cargo test --test error`
Expected: FAIL — `ModelKind` / new variants not defined.

- [ ] **Step 3: Replace `src/error.rs`**

```rust
//! Crate-wide error type.
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind { Whisper, DiarizeSegmentation, DiarizeEmbedding }

impl std::fmt::Display for ModelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ModelKind::Whisper => "whisper",
            ModelKind::DiarizeSegmentation => "diarize-segmentation",
            ModelKind::DiarizeEmbedding => "diarize-embedding",
        };
        f.write_str(s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WhisperError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("audio decode error: {0}")]
    AudioDecode(String),
    #[error("resample error: {0}")]
    Resample(String),
    #[error("{kind} model not found at {path}")]
    ModelNotFound { kind: ModelKind, path: PathBuf },
    #[error("whisper.cpp returned non-zero code {0}")]
    Ffi(i32),
    #[error("invalid configuration: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, WhisperError>;
```

Update `src/lib.rs` re-exports to include the module path:
```rust
pub mod error;
pub use error::{Result, WhisperError};
```

- [ ] **Step 4: Run tests — verify pass**

Run: `cargo test --test error`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```
git add src/error.rs src/lib.rs tests/error.rs
git commit -m "feat(error): crate-wide WhisperError + ModelKind"
```

---

### Task 4: Structured output types

**Files:**
- Create: `src/output.rs`
- Modify: `src/lib.rs`
- Test: `tests/output.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Transcript { segments: Vec<Segment> }`, `Segment { speaker: Option<SpeakerId>, text: String, start: f32, end: f32, words: Vec<Word>, flags: SegmentFlags }`, `Word { text: String, start: f32, end: f32, confidence: f32 }`, `SpeakerId(u32)`, `SegmentFlags` (bitflags-free: a plain struct with `hallucination_suspect: bool`). `Transcript::plain_text() -> String`.

- [ ] **Step 1: Write the failing test `tests/output.rs`**

```rust
use whisper_rs::output::{Segment, SegmentFlags, Transcript, Word};

#[test]
fn plain_text_joins_segment_text_in_order() {
    let t = Transcript { segments: vec![
        Segment { speaker: None, text: "hello".into(), start: 0.0, end: 1.0, words: vec![], flags: SegmentFlags::default() },
        Segment { speaker: None, text: "world".into(), start: 1.0, end: 2.0, words: vec![], flags: SegmentFlags::default() },
    ]};
    assert_eq!(t.plain_text(), "hello world");
}

#[test]
fn word_fields_roundtrip() {
    let w = Word { text: "hi".into(), start: 0.1, end: 0.3, confidence: 0.9 };
    assert_eq!(w.text, "hi");
    assert!((w.end - w.start - 0.2).abs() < 1e-6);
}
```

- [ ] **Step 2: Run it — verify it fails**

Run: `cargo test --test output`
Expected: FAIL — `output` module missing.

- [ ] **Step 3: Write `src/output.rs`**

```rust
//! Structured, analytics-ready transcript types.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeakerId(pub u32);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SegmentFlags {
    /// Set by the (later) post-processing stage when a segment looks hallucinated.
    pub hallucination_suspect: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub speaker: Option<SpeakerId>,
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub words: Vec<Word>,
    pub flags: SegmentFlags,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcript {
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// Concatenate segment text with single spaces, trimmed.
    pub fn plain_text(&self) -> String {
        self.segments.iter().map(|s| s.text.trim()).collect::<Vec<_>>().join(" ")
    }
}
```

Add to `src/lib.rs`:
```rust
pub mod output;
```

- [ ] **Step 4: Run tests — verify pass**

Run: `cargo test --test output`
Expected: PASS.

- [ ] **Step 5: Commit**

```
git add src/output.rs src/lib.rs tests/output.rs
git commit -m "feat(output): Transcript/Segment/Word structured types"
```

---

### Task 5: Audio decode + resample to 16 kHz mono

**Files:**
- Create: `src/audio/mod.rs`
- Modify: `src/lib.rs`
- Test: `tests/audio.rs`
- Test fixture: `tests/fixtures/sine_8k_stereo.wav` (generated in Step 1)

**Interfaces:**
- Consumes: `WhisperError`.
- Produces: `AudioInput` with `AudioInput::from_wav_file(path) -> Result<AudioInput>` and `AudioInput::to_mono_16k(&self) -> Result<Vec<f32>>`. Internally holds `samples: Vec<f32>` (interleaved), `channels: u16`, `sample_rate: u32`.

- [ ] **Step 1: Generate a WAV fixture with a tiny helper**

Create `tests/fixtures/` and write `tests/gen_fixture.rs` as an ignored test that writes the fixture (run once):
```rust
#[test]
#[ignore = "run once to (re)generate the fixture"]
fn generate_sine_fixture() {
    let spec = hound::WavSpec { channels: 2, sample_rate: 8000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create("tests/fixtures/sine_8k_stereo.wav", spec).unwrap();
    for n in 0..8000 { // 1 second
        let s = ((n as f32 / 8000.0) * 440.0 * std::f32::consts::TAU).sin();
        let v = (s * i16::MAX as f32) as i16;
        w.write_sample(v).unwrap();   // L
        w.write_sample(v).unwrap();   // R
    }
    w.finalize().unwrap();
}
```
Run: `cargo test --test gen_fixture -- --ignored` to create the file, then `git add` it.

- [ ] **Step 2: Write the failing test `tests/audio.rs`**

```rust
use whisper_rs::audio::AudioInput;

#[test]
fn decodes_and_resamples_to_16k_mono() {
    let a = AudioInput::from_wav_file("tests/fixtures/sine_8k_stereo.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    // 1s of 8kHz -> ~1s of 16kHz => ~16000 samples (allow small resampler edge tolerance).
    assert!((pcm.len() as i32 - 16000).abs() < 400, "got {} samples", pcm.len());
    // sine stays within [-1, 1].
    assert!(pcm.iter().all(|s| s.abs() <= 1.001));
}
```

- [ ] **Step 3: Run it — verify it fails**

Run: `cargo test --test audio`
Expected: FAIL — `audio` module missing.

- [ ] **Step 4: Write `src/audio/mod.rs`**

```rust
//! Audio decode + normalization to whisper.cpp's required 16 kHz mono f32 PCM.
use crate::error::{Result, WhisperError};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::path::Path;

const TARGET_RATE: u32 = 16_000;

pub struct AudioInput {
    samples: Vec<f32>, // interleaved
    channels: u16,
    sample_rate: u32,
}

impl AudioInput {
    /// Decode a PCM WAV (int or float) into interleaved f32 in [-1, 1].
    pub fn from_wav_file<P: AsRef<Path>>(path: P) -> Result<AudioInput> {
        let mut reader = hound::WavReader::open(path).map_err(|e| WhisperError::AudioDecode(e.to_string()))?;
        let spec = reader.spec();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
                reader.samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max).map_err(|e| WhisperError::AudioDecode(e.to_string())))
                    .collect::<Result<Vec<_>>>()?
            }
            hound::SampleFormat::Float => reader.samples::<f32>()
                .map(|s| s.map_err(|e| WhisperError::AudioDecode(e.to_string())))
                .collect::<Result<Vec<_>>>()?,
        };
        Ok(AudioInput { samples, channels: spec.channels, sample_rate: spec.sample_rate })
    }

    /// Downmix to mono and resample to 16 kHz.
    pub fn to_mono_16k(&self) -> Result<Vec<f32>> {
        let mono = self.downmix_mono();
        if self.sample_rate == TARGET_RATE {
            return Ok(mono);
        }
        self.resample(mono)
    }

    fn downmix_mono(&self) -> Vec<f32> {
        let ch = self.channels as usize;
        if ch <= 1 { return self.samples.clone(); }
        self.samples.chunks(ch).map(|frame| frame.iter().sum::<f32>() / ch as f32).collect()
    }

    fn resample(&self, mono: Vec<f32>) -> Result<Vec<f32>> {
        let ratio = TARGET_RATE as f64 / self.sample_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 256, f_cutoff: 0.95, oversampling_factor: 256,
            interpolation: SincInterpolationType::Linear, window: WindowFunction::BlackmanHarris2,
        };
        let mut rs = SincFixedIn::<f32>::new(ratio, 2.0, params, mono.len(), 1)
            .map_err(|e| WhisperError::Resample(e.to_string()))?;
        let out = rs.process(&[mono], None).map_err(|e| WhisperError::Resample(e.to_string()))?;
        Ok(out.into_iter().next().unwrap_or_default())
    }
}
```

Add `pub mod audio;` to `src/lib.rs`.

- [ ] **Step 5: Run tests — verify pass**

Run: `cargo test --test audio`
Expected: PASS. (If sample count is off, confirm the fixture was regenerated in Step 1.)

- [ ] **Step 6: Commit**

```
git add src/audio/ src/lib.rs tests/audio.rs tests/gen_fixture.rs tests/fixtures/sine_8k_stereo.wav
git commit -m "feat(audio): WAV decode + downmix + 16kHz resample"
```

---

### Task 6: Core ASR over FFI

**Files:**
- Create: `src/asr/mod.rs`
- Modify: `src/ffi/mod.rs` (add safe RAII context wrapper)
- Modify: `src/lib.rs`
- Test: `tests/asr.rs`

**Interfaces:**
- Consumes: `ffi`, `WhisperError`, `output::Segment`.
- Produces: `Transcriber` with `Transcriber::from_model_file(path) -> Result<Transcriber>` and `Transcriber::transcribe(&mut self, pcm: &[f32], opts: &AsrOptions) -> Result<Vec<Segment>>` (segments carry text + start/end seconds; `words` empty until Task 7). `AsrOptions { language: Option<String>, threads: i32 }` with `Default`.

- [ ] **Step 1: Add a safe RAII context wrapper to `src/ffi/mod.rs`**

Append:
```rust
use std::ffi::CString;
use std::path::Path;

/// Owns a `whisper_context`, freeing it on drop.
pub struct Context(*mut whisper_context);

// SAFETY: a Context is used single-threaded (one per Transcriber; whisper state is not Sync).
unsafe impl Send for Context {}

impl Context {
    pub fn from_file(path: &Path) -> crate::error::Result<Context> {
        let c = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| crate::error::WhisperError::Config(e.to_string()))?;
        // SAFETY: default params; c is a valid NUL-terminated path for the duration of the call.
        let ctx = unsafe {
            let params = whisper_context_default_params();
            whisper_init_from_file_with_params(c.as_ptr(), params)
        };
        if ctx.is_null() {
            return Err(crate::error::WhisperError::ModelNotFound {
                kind: crate::error::ModelKind::Whisper, path: path.to_path_buf(),
            });
        }
        Ok(Context(ctx))
    }
    pub fn as_ptr(&self) -> *mut whisper_context { self.0 }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: self.0 came from whisper_init_* and is freed exactly once.
        unsafe { whisper_free(self.0) }
    }
}
```

- [ ] **Step 2: Write the failing test `tests/asr.rs`** (model-gated → `#[ignore]` by default)

```rust
use whisper_rs::asr::{AsrOptions, Transcriber};

// Requires a tiny GGML model at models/ggml-tiny.en.bin and a known clip.
// Download once: bash vendor/whisper.cpp/models/download-ggml-model.sh tiny.en
#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn transcribes_known_clip() {
    let mut t = Transcriber::from_model_file("models/ggml-tiny.en.bin").unwrap();
    let a = whisper_rs::audio::AudioInput::from_wav_file("tests/fixtures/jfk.wav").unwrap();
    let pcm = a.to_mono_16k().unwrap();
    let segs = t.transcribe(&pcm, &AsrOptions::default()).unwrap();
    let text = segs.iter().map(|s| s.text.as_str()).collect::<String>().to_lowercase();
    assert!(text.contains("country"), "expected JFK clip text, got: {text:?}");
    assert!(segs.iter().all(|s| s.end >= s.start));
}

#[test]
fn missing_model_is_typed_error() {
    let err = Transcriber::from_model_file("models/does-not-exist.bin").unwrap_err();
    assert!(matches!(err, whisper_rs::WhisperError::ModelNotFound { .. }));
}
```

- [ ] **Step 3: Run it — verify the non-ignored test fails**

Run: `cargo test --test asr`
Expected: FAIL — `asr` module missing.

- [ ] **Step 4: Write `src/asr/mod.rs`**

```rust
//! Core transcription over whisper.cpp.
use crate::error::{Result, WhisperError};
use crate::ffi;
use crate::output::{Segment, SegmentFlags};
use std::ffi::{CStr, CString};
use std::path::Path;

pub struct AsrOptions {
    pub language: Option<String>, // None => auto-detect
    pub threads: i32,
}
impl Default for AsrOptions {
    fn default() -> Self { Self { language: None, threads: num_cpus_or(4) } }
}
fn num_cpus_or(default: i32) -> i32 {
    std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(default)
}

pub struct Transcriber { ctx: ffi::Context }

impl Transcriber {
    pub fn from_model_file<P: AsRef<Path>>(path: P) -> Result<Transcriber> {
        Ok(Transcriber { ctx: ffi::Context::from_file(path.as_ref())? })
    }

    pub fn transcribe(&mut self, pcm: &[f32], opts: &AsrOptions) -> Result<Vec<Segment>> {
        let lang = opts.language.as_deref().unwrap_or("auto");
        let clang = CString::new(lang).map_err(|e| WhisperError::Config(e.to_string()))?;
        // SAFETY: pcm outlives the whisper_full call; params reference clang which is kept alive.
        let segs = unsafe {
            let mut params = ffi::whisper_full_default_params(
                ffi::whisper_sampling_strategy_WHISPER_SAMPLING_GREEDY);
            params.language = clang.as_ptr();
            params.n_threads = opts.threads;
            params.print_progress = false;
            params.print_realtime = false;
            params.token_timestamps = true; // enables per-token times used in Task 7

            let rc = ffi::whisper_full(self.ctx.as_ptr(), params, pcm.as_ptr(), pcm.len() as i32);
            if rc != 0 { return Err(WhisperError::Ffi(rc)); }

            let n = ffi::whisper_full_n_segments(self.ctx.as_ptr());
            let mut out = Vec::with_capacity(n as usize);
            for i in 0..n {
                let ptr = ffi::whisper_full_get_segment_text(self.ctx.as_ptr(), i);
                let text = if ptr.is_null() { String::new() }
                           else { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
                let t0 = ffi::whisper_full_get_segment_t0(self.ctx.as_ptr(), i) as f32 / 100.0; // centiseconds
                let t1 = ffi::whisper_full_get_segment_t1(self.ctx.as_ptr(), i) as f32 / 100.0;
                out.push(Segment { speaker: None, text, start: t0, end: t1, words: vec![], flags: SegmentFlags::default() });
            }
            out
        };
        Ok(segs)
    }
}
```

Add `pub mod asr;` to `src/lib.rs`.

- [ ] **Step 5: Run tests — verify the non-ignored test passes**

Run: `cargo test --test asr`
Expected: `missing_model_is_typed_error` PASSES; the ignored test is skipped.

- [ ] **Step 6: Run the model-gated test once locally**

Run:
```
bash vendor/whisper.cpp/models/download-ggml-model.sh tiny.en    # writes ggml-tiny.en.bin
mv vendor/whisper.cpp/models/ggml-tiny.en.bin models/
cp vendor/whisper.cpp/samples/jfk.wav tests/fixtures/jfk.wav
cargo test --test asr -- --ignored
```
Expected: `transcribes_known_clip` PASSES (text contains "country").

- [ ] **Step 7: Commit**

```
git add src/ffi/mod.rs src/asr/ src/lib.rs tests/asr.rs
git commit -m "feat(asr): Transcriber over whisper.cpp with RAII context"
```

---

### Task 7: Word/segment timestamps (DTW)

**Files:**
- Modify: `src/asr/mod.rs` (populate `Segment::words`)
- Create: `src/timestamps/mod.rs`
- Modify: `src/lib.rs`
- Test: `tests/timestamps.rs`

**Interfaces:**
- Consumes: `ffi` token-timestamp data, `output::{Segment, Word}`.
- Produces: `timestamps::words_for_segment(ctx, seg_index) -> Vec<Word>` and integration so `Transcriber::transcribe` fills `Segment::words`. Invariant guaranteed: within a segment, words are ordered and non-overlapping (`w[k].end <= w[k+1].start`).

- [ ] **Step 1: Write the failing test `tests/timestamps.rs`** (pure invariant test — no model)

```rust
use whisper_rs::timestamps::enforce_monotonic;
use whisper_rs::output::Word;

#[test]
fn enforce_monotonic_fixes_overlap_and_order() {
    let words = vec![
        Word { text: "a".into(), start: 0.0, end: 0.5, confidence: 1.0 },
        Word { text: "b".into(), start: 0.4, end: 0.9, confidence: 1.0 }, // overlaps a
        Word { text: "c".into(), start: 0.8, end: 0.7, confidence: 1.0 }, // end < start
    ];
    let fixed = enforce_monotonic(words);
    for pair in fixed.windows(2) {
        assert!(pair[0].end <= pair[1].start + 1e-6, "overlap remains: {pair:?}");
    }
    assert!(fixed.iter().all(|w| w.end >= w.start));
}
```

- [ ] **Step 2: Run it — verify it fails**

Run: `cargo test --test timestamps`
Expected: FAIL — `timestamps` module missing.

- [ ] **Step 3: Write `src/timestamps/mod.rs`**

```rust
//! Word-level timestamps extracted from whisper.cpp token data, with monotonicity enforcement.
use crate::ffi;
use crate::output::Word;

/// Clamp a word list to be ordered and non-overlapping in place-order.
pub fn enforce_monotonic(mut words: Vec<Word>) -> Vec<Word> {
    let mut cursor = 0.0f32;
    for w in words.iter_mut() {
        if w.start < cursor { w.start = cursor; }
        if w.end < w.start { w.end = w.start; }
        cursor = w.end;
    }
    words
}

/// Build words for one segment from whisper.cpp per-token data.
///
/// # Safety
/// `ctx` must be a live whisper context that has just completed a `whisper_full` run.
pub(crate) unsafe fn words_for_segment(ctx: *mut ffi::whisper_context, seg: i32) -> Vec<Word> {
    let n = ffi::whisper_full_n_tokens(ctx, seg);
    let mut words = Vec::new();
    for j in 0..n {
        let td = ffi::whisper_full_get_token_data(ctx, seg, j);
        let tptr = ffi::whisper_full_get_token_text(ctx, seg, j);
        if tptr.is_null() { continue; }
        let text = std::ffi::CStr::from_ptr(tptr).to_string_lossy().into_owned();
        // whisper marks special tokens with a leading '[' or "<|...|>"; skip them.
        if text.starts_with('[') || text.starts_with("<|") { continue; }
        words.push(Word {
            text: text.trim().to_string(),
            start: td.t0 as f32 / 100.0, // centiseconds
            end: td.t1 as f32 / 100.0,
            confidence: td.p,
        });
    }
    enforce_monotonic(words)
}
```

Add `pub mod timestamps;` to `src/lib.rs`.

- [ ] **Step 4: Populate words in `src/asr/mod.rs`**

In the segment loop of `transcribe`, replace `words: vec![]` with:
```rust
words: crate::timestamps::words_for_segment(self.ctx.as_ptr(), i),
```
(This call is inside the existing `unsafe` block, so no new `unsafe` is added.)

- [ ] **Step 5: Run tests — verify pass**

Run: `cargo test --test timestamps`
Expected: PASS. Then re-run the model-gated asr test to confirm words populate:
```
cargo test --test asr -- --ignored
```
Add a temporary assert (or observe) that `segs[0].words` is non-empty, then remove it.

- [ ] **Step 6: Commit**

```
git add src/timestamps/ src/asr/mod.rs src/lib.rs tests/timestamps.rs
git commit -m "feat(timestamps): DTW word timestamps + monotonic enforcement"
```

---

### Task 8: Batch Pipeline (builder + transcribe_file)

**Files:**
- Create: `src/pipeline.rs`
- Create: `src/prelude.rs`
- Modify: `src/lib.rs`
- Test: `tests/pipeline.rs`

**Interfaces:**
- Consumes: `audio::AudioInput`, `asr::{Transcriber, AsrOptions}`, `output::Transcript`, `error`.
- Produces: `Pipeline` + `PipelineBuilder`. `Pipeline::builder() -> PipelineBuilder`; `.whisper_model(ModelRef) -> Self`; `.language(Option<String>) -> Self`; `.build() -> Result<Pipeline>`; `Pipeline::transcribe_file(&mut self, path) -> Result<Transcript>`. `ModelRef::path(P)`; `ModelRef::download(_)` returns `WhisperError::Config` in this plan (implemented in the download plan). Diarization/streaming builder methods are added by later plans.

- [ ] **Step 1: Write the failing test `tests/pipeline.rs`**

```rust
use whisper_rs::pipeline::{ModelRef, Pipeline};

#[test]
fn build_requires_a_whisper_model() {
    let err = Pipeline::builder().build().unwrap_err();
    assert!(matches!(err, whisper_rs::WhisperError::Config(_)));
}

#[test]
#[ignore = "needs models/ggml-tiny.en.bin + tests/fixtures/jfk.wav"]
fn transcribe_file_returns_timestamped_transcript() {
    let mut p = Pipeline::builder()
        .whisper_model(ModelRef::path("models/ggml-tiny.en.bin"))
        .language(Some("en".into()))
        .build().unwrap();
    let t = p.transcribe_file("tests/fixtures/jfk.wav").unwrap();
    assert!(!t.segments.is_empty());
    assert!(t.plain_text().to_lowercase().contains("country"));
    assert!(t.segments.iter().flat_map(|s| &s.words).count() > 0);
}
```

- [ ] **Step 2: Run it — verify it fails**

Run: `cargo test --test pipeline`
Expected: FAIL — `pipeline` module missing.

- [ ] **Step 3: Write `src/pipeline.rs`**

```rust
//! High-level batch pipeline layered over the composable stages.
use crate::asr::{AsrOptions, Transcriber};
use crate::audio::AudioInput;
use crate::error::{Result, WhisperError};
use crate::output::Transcript;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub enum ModelRef {
    Path(PathBuf),
    #[allow(dead_code)]
    Download(&'static str), // resolved by the `download` plan; errors here.
}
impl ModelRef {
    pub fn path<P: AsRef<Path>>(p: P) -> ModelRef { ModelRef::Path(p.as_ref().to_path_buf()) }
    pub fn download(id: &'static str) -> ModelRef { ModelRef::Download(id) }

    fn resolve(&self) -> Result<PathBuf> {
        match self {
            ModelRef::Path(p) => Ok(p.clone()),
            ModelRef::Download(_) => Err(WhisperError::Config(
                "model download requires the `download` feature (not in this build)".into())),
        }
    }
}

#[derive(Default)]
pub struct PipelineBuilder {
    whisper_model: Option<ModelRef>,
    language: Option<String>,
}
impl PipelineBuilder {
    pub fn whisper_model(mut self, m: ModelRef) -> Self { self.whisper_model = Some(m); self }
    pub fn language(mut self, l: Option<String>) -> Self { self.language = l; self }
    pub fn build(self) -> Result<Pipeline> {
        let model = self.whisper_model
            .ok_or_else(|| WhisperError::Config("whisper_model is required".into()))?;
        let path = model.resolve()?;
        let transcriber = Transcriber::from_model_file(&path)?;
        Ok(Pipeline { transcriber, opts: AsrOptions { language: self.language, ..Default::default() } })
    }
}

pub struct Pipeline { transcriber: Transcriber, opts: AsrOptions }

impl Pipeline {
    pub fn builder() -> PipelineBuilder { PipelineBuilder::default() }

    pub fn transcribe_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Transcript> {
        let pcm = AudioInput::from_wav_file(path)?.to_mono_16k()?;
        let segments = self.transcriber.transcribe(&pcm, &self.opts)?;
        Ok(Transcript { segments })
    }
}
```

Write `src/prelude.rs`:
```rust
//! Convenient re-exports.
pub use crate::error::{Result, WhisperError};
pub use crate::output::{Segment, Transcript, Word};
pub use crate::pipeline::{ModelRef, Pipeline};
```

Add to `src/lib.rs`:
```rust
pub mod pipeline;
pub mod prelude;
```

- [ ] **Step 4: Run tests — verify pass**

Run: `cargo test --test pipeline`
Expected: `build_requires_a_whisper_model` PASSES; ignored test skipped.

- [ ] **Step 5: Run the full model-gated flow once**

Run: `cargo test --test pipeline -- --ignored`
Expected: `transcribe_file_returns_timestamped_transcript` PASSES.

- [ ] **Step 6: Feature-matrix build check**

Run:
```
cargo build --no-default-features
cargo build --all-features
cargo test
```
Expected: all compile; non-ignored tests pass. (Features are empty stubs now, so this only proves the matrix wiring.)

- [ ] **Step 7: Commit**

```
git add src/pipeline.rs src/prelude.rs src/lib.rs tests/pipeline.rs
git commit -m "feat(pipeline): batch Pipeline builder + transcribe_file"
```

---

## Self-Review

**Spec coverage (foundation slice):**
- FFI isolation / single crate → Task 2 + `unsafe`-only-in-`ffi` constraint. ✓
- Error model (`WhisperError`, typed model-missing, `Ffi(i32)`, `Config`) → Task 3, exercised in Tasks 6 & 8. ✓
- Structured output types → Task 4. ✓
- 16 kHz mono requirement + resample → Task 5. ✓
- Core ASR + word timestamps → Tasks 6–7. ✓
- Layered API: composable stages (`AudioInput`, `Transcriber`, `timestamps`) + high-level `Pipeline` → Tasks 5–8. ✓
- Feature-matrix builds → Task 8 Step 6. ✓
- **Deferred (correctly not in this plan):** diarization, streaming, preprocessing levels 0–4, VAD, post-processing (hallucination/number-normalization), model downloader. These are the subjects of plans 2–4. Noted so coverage gaps are intentional, not accidental.

**Placeholder scan:** No "TBD/handle edge cases/similar to Task N" — every code step carries real code. The one implementer-judgment note (Task 2 whisper.cpp source file list) is explicitly build-glue tuning against on-disk reality, not a design placeholder.

**Type consistency:** `AsrOptions`, `Transcriber::transcribe(&mut self, &[f32], &AsrOptions)`, `Segment`/`Word` fields, `ModelRef::path/download`, `Pipeline::builder().whisper_model().language().build()` — names/signatures match across Tasks 4–8. `ffi::Context::as_ptr()` used consistently in Tasks 6–7.

---

## Follow-on plans (not in this foundation)

- **Plan 2 — Diarization** (`feature = "diarization"`): `ort` + pyannote-segmentation-3.0 + embedding + clustering → `Diarizer::diarize(pcm) -> Vec<SpeakerTurn>`, `merge(words, turns)`, wire `Pipeline::diarization(cfg)`.
- **Plan 3 — Streaming** (`feature = "streaming"`): `StreamPolicy` trait + `LocalAgreement2` + `TwoPass`, `cpal` capture, `tokio` glue, `Pipeline::stream(policy)` emitting events.
- **Plan 4 — Post-processing + audio preprocessing + model downloader**: VAD + preprocessing levels 0–4; hallucination flagging + number normalization; `ModelRef::download` + cache behind `feature = "download"`.

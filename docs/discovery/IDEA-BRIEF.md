# Idea Brief — whisper-rs (2026-07-14)

## What this folder is
A greenfield planning folder for a new, local-use-only Rust crate (`dyammarcano/whisper-rs`)
that wraps `whisper.cpp` behind a Rust glue layer to create a "feature enrich[ed]" crate usable
from other projects, per the goal statement verbatim in `idea.txt` line 19 (content-analyst
evidence). The folder contains no code — only `idea.txt` (a 1-line goal + a 13-URL reading
list of prior-art repos, models, and discussion threads) and two saved articles (Rafael
Galle's "Custom Scalable Audio Transcription Pipeline" and HTX-DSAI's "Beyond Speech-To-Text").
A pre-existing `docs/` tree from a prior auto-run is stale prior output, not source material,
and is superseded by this brief.

The user has already decided: (1) v1 ships **feature-rich from the start** — core ASR,
streaming, diarization, timestamps, and preprocessing all together, not staged; (2) the crate
is for **local use only**, not published to crates.io, so the name collision with the existing
`whisper-rs` crate on crates.io is a non-issue and not treated as a risk here.

## Capabilities found
- No existing code, build system, or binaries in this folder — it is pure planning material
  (scanner inline summary; content-analyst-source.md file list).
- `idea.txt` supplies the goal statement and a curated reading list of 13 URLs spanning the
  target library (`whisper.cpp`), the closest existing Rust wrapper (`tazz4843/whisper-rs`),
  a non-FFI alternative (`whisper-burn`), two streaming/real-time reference implementations
  (`optiummusic/Whisper-Real-Time-Transcription`, `ayakashi-labs/yamabiko-whisper`), two
  diarization pipelines (`MahmoudAshraf97/whisper-diarization`, `m-bain/whisperx`), two ONNX
  models for diarization/timestamps, and Rafael Galle's pipeline repo/demo
  (content-analyst-source.md lines 9-26).
- Article 1 (Galle) documents a working, production-deployed pipeline: multi-source ingestion
  → 16kHz mono PCM conversion → stereo/mono channel split → tiered preprocessing (levels 0-4)
  → Faster-Whisper transcription → Pyannote diarization → timeline merge into a structured,
  speaker+timestamp-attributed transcript; claims ~90% cost reduction vs a third-party
  provider and 5 min stereo audio processed in 4.6s on an L40S GPU
  (content-analyst-source.md lines 28-46; web-research.md line 26).
- Article 2 (HTX-DSAI) documents concrete, named engineering problems and mitigations:
  Whisper hallucination causes/mitigation via cross-method disagreement flagging; spoken-form
  vs digit-form number normalization; and a deep treatment of online/streaming diarization
  (DER metric definition, Diart attractor-based and Sortformer arrival-order-cache approaches,
  speaker-enrollment cold-start mitigation, and multi-mic DOA/TDOA spatial diarization as an
  alternative to embedding-based diarization) (content-analyst-source.md lines 48-76).

## Gaps & opportunities
- `tazz4843/whisper-rs` (closest prior art) has **no diarization, no streaming, no
  ergonomic word-level timestamps** — DTW timestamp fields exist only behind its unsafe
  `raw-api` feature, an open ask since issue #71 (web-research.md lines 13, 16, 65, 71).
- No mainstream pipeline, Python or Rust, ships **batteries-included real-time diarized
  transcription** — WhisperX issues #1065 and #1289 remain open feature requests for exactly
  this (web-research.md line 64). This is the strongest market gap found.
- Overlapping speech handling is a recurring unsolved weak point across all tools surveyed
  (WhisperX docs, whisper-diarization issues, OpenAI community thread) — worth scoping as a
  known limitation rather than a v1 promise (web-research.md line 63).
- whisper.cpp itself already ships Silero VAD integration, experimental DTW word-timestamps,
  and an experimental "tinydiarize" speaker-turn marker — none of these are exposed at a safe,
  ergonomic level in the existing Rust wrapper, so there's a low-effort/high-leverage
  opportunity to surface native capability rather than reinvent it (web-research.md line 45).
- Rust-native (non-Python) diarization is now genuinely possible via ONNX Runtime
  (`pyannote-rs`, `speakrs`) but is pre-1.0 / uses a pre-release `ort` (2.0.0-rc.12) — an API
  stability risk to flag, not a blocker (web-research.md lines 36-38, 75).

## Landscape (external research)
**Prior art**: `tazz4843/whisper-rs` (Codeberg) is a mature two-tier bindgen FFI wrapper
(`whisper-rs-sys` + `whisper-rs`) but lacks diarization/streaming/first-class timestamps.
WhisperX and `whisper-diarization` (Python) both bolt Whisper + forced alignment + pyannote
clustering together with real production tradeoffs (HF-gated models, GPU dependency, no
real-time mode). `whisper-diarization-advanced` (rafaelgalle) is the closest full-pipeline
template, including tiered preprocessing and stereo channel-split diarization.
`yamabiko-whisper` is architecturally the closest streaming precedent, built directly on
`whisper-rs` with a LocalAgreement-2 commit policy. `optiummusic`'s app demonstrates a
two-pass tiny+medium streaming UX. `whisper-burn` is a viable but less mature non-FFI
alternative (web-research.md lines 8-31).

**Tech**: bindgen + `cc`/build.rs remains correct for whisper.cpp's C header surface (not
`cxx`, which targets idiomatic C++). `ort` (ONNX Runtime bindings) is the confirmed path for
both native-Rust diarization (`pyannote-segmentation-3.0` + speaker embeddings, per
`pyannote-rs`/`speakrs`) and VAD (`silero-vad-rs`) if not using whisper.cpp's own built-in
Silero VAD. `rubato` is the standard resampler; `cpal` for mic capture; `symphonia`/`hound`
for file I/O; `tokio` channels for streaming glue (web-research.md lines 35-44).

**Best practices**: convergent pipeline shape across every source studied — (optional
stereo-split or source separation) → resample/normalize to 16kHz mono → VAD segmentation →
ASR → word-timestamp alignment → speaker embedding + clustering → timeline merge → structured
transcript. For streaming specifically, two validated patterns: LocalAgreement-2
(yamabiko-whisper) and two-pass tiny/medium model (optiummusic), both VAD-boundary-driven
rather than fixed windows (web-research.md lines 49-56).

**Market/user needs**: users consistently describe manual Whisper+pyannote composition as
"complicated" and "not even close to ideal"; there is explicit demand for faster diarization
inference (Rust ONNX approaches claim 50-900x realtime vs stock pyannote) and for real-time
diarized streaming, which nothing ships today (web-research.md lines 60-65).

*(Security/compliance dimension was explicitly not requested and is omitted per the
questionnaire.)*

## Feature-rich idea candidates (ranked)

1. **The combined feature-rich whisper.cpp Rust wrapper crate (top pick)** — a single crate
   unifying core ASR, streaming, diarization, word-level timestamps, and audio preprocessing
   behind one coherent API, built on top of whisper.cpp via bindgen FFI.
   - *Why it fits*: directly matches the verbatim goal statement (content-analyst-source.md
     line 7) and the user's explicit v1 decision to ship all capabilities together rather than
     stage them. It's built on the convergent pipeline pattern found in every reference
     source (web-research.md lines 49-56) and fills the exact gap — no existing tool, Rust or
     Python, ships batteries-included real-time diarized transcription (web-research.md
     line 64).
   - *Rough scope, as internal modules of the one crate*:
     - **Core ASR** — bindgen FFI over whisper.cpp's C API, following `tazz4843/whisper-rs`'s
       proven two-tier `-sys`/safe-wrapper structure as the FFI convention baseline
       (web-research.md line 35).
     - **Preprocessing** — unified module: resample to 16kHz mono via `rubato`, VAD via
       whisper.cpp's built-in Silero support or `silero-vad-rs`, stereo/channel handling and
       tiered preprocessing levels modeled on Galle's 0-4 scheme (content-analyst-source.md
       line 81; web-research.md line 68), shared by both batch and streaming paths.
     - **Timestamps** — expose whisper.cpp's existing DTW token-timestamp fields
       (`whisper_context_params.dtw_token_timestamps`, `t_dtw`) as a safe, ergonomic API —
       answers a 5+ year open community ask (web-research.md lines 45, 71).
     - **Diarization** — native Rust ONNX diarization via `ort` + `pyannote-segmentation-3.0`
       + speaker embeddings, per `pyannote-rs`/`speakrs` precedent (web-research.md lines
       36-38, 69), plus a stereo channel-split fast-path for dual-channel/call-center audio
       as a near-zero-clustering-error alternative (content-analyst-source.md lines 82, 84;
       web-research.md lines 26, 72). Speaker-enrollment API to seed embeddings and avoid
       cold-start misclassification (content-analyst-source.md line 86, from Article 2's
       enrollment trick).
     - **Streaming** — LocalAgreement-2 commit policy (yamabiko-whisper pattern) as primary,
       with the two-pass tiny/medium model (optiummusic pattern) as a documented alternative
       streaming mode; VAD-boundary-driven chunking, not fixed windows (web-research.md
       lines 52-56).
     - **Output** — structured, analytics-ready transcript type: speaker ID, text, word
       timestamps, segment duration (content-analyst-source.md line 84).
     - **Optional/stretch**: hallucination flagging via cross-method disagreement, spoken→digit
       number normalization, DER-aware diarization metrics hooks, multi-mic DOA/TDOA spatial
       diarization (content-analyst-source.md lines 88-90; content-analyst-source.md line 87)
       — real capabilities cited in Article 2, but heavier/more speculative; candidates for
       later phases within the same crate rather than separate crates.
   - *Target stack fit*: Rust, matches exactly (bindgen/cc for FFI, `ort` for ONNX,
     `rubato`/`cpal`/`symphonia`/`hound`/`tokio` for the rest — all confirmed idiomatic
     choices per web-research.md lines 35-44).
   - *Risks/unknowns*: `ort` 2.0.0-rc.12 is pre-release, pin/track carefully
     (web-research.md line 75); overlapping speech remains unsolved industry-wide, scope
     expectations accordingly (web-research.md line 63); whisper.cpp's tinydiarize is
     turn-detection only, do not conflate with full diarization (web-research.md line 78);
     streaming diarization specifically degrades without clear speaker pauses (Article 2,
     content-analyst-source.md line 70) — document as a known limitation, not a bug.

2. **Runner-up / rejected-for-now alternative — pure-Rust Burn-based reimplementation**
   (`whisper-burn` as the base instead of whisper.cpp FFI) — avoids FFI entirely but is less
   mature (~356 stars, no formal releases, no built-in streaming/timestamp support found)
   (web-research.md line 29). Worth revisiting only if FFI/build-system friction with
   whisper.cpp becomes a real blocker; not recommended as the v1 foundation given the user's
   explicit "glue layer over whisper.cpp" framing (content-analyst-source.md line 7).

3. **Runner-up / lighter-weight variant — minimal FFI core with diarization/streaming as
   optional Cargo feature flags** — same module set as the top pick, but diarization
   (`ort` dependency) and streaming gated behind feature flags so a consumer can build a
   smaller core-ASR-only binary. Technically compatible with everything in the evidence base
   (feature-flag pattern is how `tazz4843/whisper-rs` already exposes GPU backends,
   web-research.md line 17) but **conflicts with the user's explicit v1 decision** that all
   capabilities ship together "from the start" — noted here only as an implementation-detail
   option (features vs. always-on) for brainstorming to weigh, not a scope change.

## Recommended direction
Candidate 1, the combined feature-rich whisper.cpp Rust wrapper crate, as a single crate with
the module breakdown above. It is the only candidate that satisfies the user's explicit goal
statement and locked-in v1 scope decision, it is grounded in a pipeline shape independently
converged on by every prior-art source studied, and it targets a genuinely open market gap
(real-time diarized transcription with ergonomic word timestamps) that no existing Rust or
Python tool currently ships. Candidates 2 and 3 are documented as real alternatives for
brainstorming to explicitly weigh and reject/accept, not as equal-weight options.

## Open questions for brainstorming
- Feature-flag structure: should diarization/streaming/preprocessing be always-on modules or
  Cargo feature-gated (runner-up 3) even though all are compiled into v1 by default?
- Diarization backend default: `ort`+pyannote (native Rust, but pre-release `ort`) vs.
  whisper.cpp's built-in experimental tinydiarize (coarse, turn-only) as a lighter fallback —
  which is the v1 default, and is the other kept as an alternate mode?
- Streaming policy default: LocalAgreement-2 vs. two-pass tiny/medium — pick one as default or
  expose both as configurable strategies?
- How much of Article 2's "stretch" list (hallucination flagging, number normalization, DER
  metrics, multi-mic DOA) ships in v1 proper vs. is explicitly deferred, given "feature-rich
  from the start" is the mandate but these are the least-validated-in-Rust of all the ideas?
- Public API shape: does this crate expose a single high-level `Pipeline` type, or discrete
  composable stages (preprocess/transcribe/diarize/align) that consumers wire themselves?
- Model management: does the crate handle whisper.cpp/ONNX model download & caching, or
  assume the consumer supplies local model paths?
- Overlapping-speech handling: explicitly out of scope for v1, or a "best-effort" mode given
  it's unsolved industry-wide?

## Evidence index
- Goal statement, reading list, article summaries, distilled feature ideas: `content-analyst-source.md` (evidence file, referenced by line throughout).
- Original article HTML: `D:\new_page\whisper-rs\2026-07-14T03-13-24-115Z-f8a7ce54-full.html` (Galle), `D:\new_page\whisper-rs\2026-07-14T03-16-03-612Z-0709874b-full.html` (HTX-DSAI).
- Competitive teardown, tech landscape, best practices, market gaps, sources list: `web-research.md` (evidence file, referenced by line throughout).
- Folder inventory (no code/binaries present, stale prior docs/ tree): scanner inline summary (no separate evidence file — scanner had no Write tool).
- Questionnaire decisions (feature-rich v1, local-use-only, Rust target, dimensions requested): dispatch prompt, as relayed by the coordinator.

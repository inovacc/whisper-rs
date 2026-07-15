//! Ad-hoc runner: transcribe a 16 kHz-decodable WAV with a whisper GGML model.
//!
//! Usage:
//!   cargo run --release --example transcribe -- <model-id-or-path> <file.wav> [lang]
//!
//! `<model-id-or-path>` is either a path to an existing `ggml-*.bin`, or a whisper
//! model id (e.g. `tiny`, `tiny.en`, `base`) which is fetched + cached via the
//! `download` feature. `[lang]` is an optional ISO code (e.g. `pt`, `en`); omit to
//! let whisper auto-detect.
use std::path::Path;
use whisper_rs::pipeline::{ModelRef, Pipeline};

fn main() -> whisper_rs::Result<()> {
    let mut args = std::env::args().skip(1);
    let model_arg = args.next().expect("usage: transcribe <model-id-or-path> <file.wav> [lang]");
    let wav = args.next().expect("usage: transcribe <model-id-or-path> <file.wav> [lang]");
    let lang = args.next();

    let model = if Path::new(&model_arg).exists() {
        ModelRef::path(&model_arg)
    } else {
        #[cfg(feature = "download")]
        {
            let cache = whisper_rs::models::default_cache_dir();
            eprintln!("resolving model id {model_arg:?} -> {} (larger models are a big one-time download)...", cache.display());
            let p = whisper_rs::models::download_model(&model_arg, &cache)?;
            eprintln!("model ready: {}", p.display());
            ModelRef::path(p)
        }
        #[cfg(not(feature = "download"))]
        {
            return Err(whisper_rs::WhisperError::Config(format!("no model at {model_arg:?} and `download` feature is off")));
        }
    };

    let mut pipe = Pipeline::builder().whisper_model(model).language(lang.clone()).build()?;
    eprintln!("transcribing {wav} (lang={})...", lang.as_deref().unwrap_or("auto"));
    let t0 = std::time::Instant::now();
    let is_wav = Path::new(&wav).extension().map(|e| e.eq_ignore_ascii_case("wav")).unwrap_or(false);
    let transcript = if is_wav {
        pipe.transcribe_file(&wav)?
    } else {
        #[cfg(feature = "ffmpeg")]
        {
            pipe.transcribe_media_file(&wav)?
        }
        #[cfg(not(feature = "ffmpeg"))]
        {
            return Err(whisper_rs::WhisperError::Config(format!(
                "{wav} is not a .wav — rebuild with `--features ffmpeg` to decode m4a/mp3/etc."
            )));
        }
    };
    eprintln!("done in {:.1}s, {} segment(s)", t0.elapsed().as_secs_f32(), transcript.segments.len());

    for seg in &transcript.segments {
        let flag = if seg.flags.hallucination_suspect { " [?]" } else { "" };
        println!("[{:>7.2} -> {:>7.2}]{flag} {}", seg.start, seg.end, seg.text.trim());
    }

    // Also write subtitle sidecars next to the input (dogfoods Transcript::to_srt/to_vtt).
    let wav_path = Path::new(&wav);
    let srt = wav_path.with_extension("srt");
    let vtt = wav_path.with_extension("vtt");
    std::fs::write(&srt, transcript.to_srt()).map_err(whisper_rs::WhisperError::Io)?;
    std::fs::write(&vtt, transcript.to_vtt()).map_err(whisper_rs::WhisperError::Io)?;
    eprintln!("wrote {} and {}", srt.display(), vtt.display());
    Ok(())
}

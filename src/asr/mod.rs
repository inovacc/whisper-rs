//! Core transcription over whisper.cpp (safe layer; all unsafe lives in `crate::ffi`).
use crate::error::Result;
use crate::ffi;
use crate::output::{Segment, SegmentFlags};
use std::path::Path;

pub struct AsrOptions {
    pub language: Option<String>, // None => auto-detect
    pub threads: i32,
}
impl Default for AsrOptions {
    fn default() -> Self {
        Self { language: None, threads: num_cpus_or(4) }
    }
}
fn num_cpus_or(default: i32) -> i32 {
    std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(default)
}

#[derive(Debug)]
pub struct Transcriber {
    ctx: ffi::Context,
}

impl Transcriber {
    pub fn from_model_file<P: AsRef<Path>>(path: P) -> Result<Transcriber> {
        Ok(Transcriber { ctx: ffi::Context::from_file(path.as_ref())? })
    }

    pub fn transcribe(&mut self, pcm: &[f32], opts: &AsrOptions) -> Result<Vec<Segment>> {
        let lang = opts.language.as_deref().unwrap_or("auto");
        self.ctx.full(lang, opts.threads, true, pcm)?;
        let n = self.ctx.n_segments();
        let mut out = Vec::with_capacity(n as usize);
        for i in 0..n {
            out.push(Segment {
                speaker: None,
                text: self.ctx.segment_text(i),
                start: self.ctx.segment_t0(i) as f32 / 100.0,
                end: self.ctx.segment_t1(i) as f32 / 100.0,
                words: vec![], // filled in Task 7
                flags: SegmentFlags::default(),
            });
        }
        Ok(out)
    }
}

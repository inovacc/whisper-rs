//! High-level batch pipeline layered over the composable stages.
use crate::asr::{AsrOptions, Transcriber};
use crate::audio::AudioInput;
use crate::error::{Result, WhisperError};
use crate::output::Transcript;
use crate::postprocess::PostConfig;
use std::path::{Path, PathBuf};

#[derive(Clone)]
#[non_exhaustive]
pub enum ModelRef {
    Path(PathBuf),
    #[allow(dead_code)]
    Download(&'static str), // resolved by the future `download` plan; errors here.
}
impl ModelRef {
    pub fn path<P: AsRef<Path>>(p: P) -> ModelRef {
        ModelRef::Path(p.as_ref().to_path_buf())
    }
    pub fn download(id: &'static str) -> ModelRef {
        ModelRef::Download(id)
    }

    fn resolve(&self) -> Result<PathBuf> {
        match self {
            ModelRef::Path(p) => Ok(p.clone()),
            #[cfg(feature = "download")]
            ModelRef::Download(id) => crate::models::download_model(id, &crate::models::default_cache_dir()),
            #[cfg(not(feature = "download"))]
            ModelRef::Download(_) => Err(WhisperError::Config("model download requires the `download` feature".into())),
        }
    }
}

#[derive(Default)]
pub struct PipelineBuilder {
    whisper_model: Option<ModelRef>,
    language: Option<String>,
    post: Option<PostConfig>,
    preprocess: crate::audio::preprocess::PreprocessLevel,
}
impl PipelineBuilder {
    pub fn whisper_model(mut self, m: ModelRef) -> Self {
        self.whisper_model = Some(m);
        self
    }
    pub fn language(mut self, l: Option<String>) -> Self {
        self.language = l;
        self
    }
    pub fn postprocess(mut self, cfg: PostConfig) -> Self {
        self.post = Some(cfg);
        self
    }
    pub fn preprocess(mut self, level: crate::audio::preprocess::PreprocessLevel) -> Self {
        self.preprocess = level;
        self
    }
    pub fn build(self) -> Result<Pipeline> {
        let model = self.whisper_model.ok_or_else(|| WhisperError::Config("whisper_model is required".into()))?;
        let path = model.resolve()?;
        let transcriber = Transcriber::from_model_file(&path)?;
        Ok(Pipeline {
            transcriber,
            opts: AsrOptions { language: self.language, ..Default::default() },
            post: self.post,
            preprocess: self.preprocess,
        })
    }
}

#[derive(Debug)]
pub struct Pipeline {
    transcriber: Transcriber,
    opts: AsrOptions,
    post: Option<PostConfig>,
    preprocess: crate::audio::preprocess::PreprocessLevel,
}

impl Pipeline {
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::default()
    }

    pub fn transcribe_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Transcript> {
        let pcm = AudioInput::from_wav_file(path)?.to_mono_16k()?;
        self.transcribe_mono_16k(pcm)
    }

    /// Transcribe a non-WAV media file (m4a, mp3, flac, mp4, ...) by decoding it to 16 kHz mono via
    /// the ffmpeg libraries, then running the same preprocess → ASR → post-process pipeline.
    #[cfg(feature = "ffmpeg")]
    pub fn transcribe_media_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Transcript> {
        let pcm = crate::audio::media::decode_to_mono_16k(path)?;
        self.transcribe_mono_16k(pcm)
    }

    /// Shared tail: preprocess already-16 kHz-mono PCM, transcribe, and apply post-processing.
    fn transcribe_mono_16k(&mut self, pcm: Vec<f32>) -> Result<Transcript> {
        let pcm = crate::audio::preprocess::preprocess(&pcm, self.preprocess);
        let mut segments = self.transcriber.transcribe(&pcm, &self.opts)?;
        if let Some(post) = &self.post {
            for segment in &mut segments {
                segment.text = post.apply(&segment.text);
            }
        }
        Ok(Transcript { segments })
    }
}

#[cfg(feature = "streaming")]
impl Pipeline {
    /// Turn this pipeline into a streaming session using the given policy.
    pub fn into_stream(self, policy: Box<dyn crate::stream::StreamPolicy + Send>) -> crate::stream::StreamSession {
        crate::stream::StreamSession::new(self.transcriber, policy, self.opts)
    }
}

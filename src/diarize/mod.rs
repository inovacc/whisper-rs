//! Speaker diarization: model-independent types and pure algorithms.
//!
//! This module holds the parts of diarization that do not require an ONNX
//! runtime: shared types, the timeline-merge that assigns speaker turns to
//! transcript segments, and agglomerative clustering over embedding vectors.
//! The model-backed segmentation/embedding pipeline (a `Diarizer` struct
//! that runs ONNX inference) is added separately once `ort` is wired in.

use std::path::{Path, PathBuf};

use crate::output::SpeakerId;

pub mod cluster;
pub mod merge;

/// A single contiguous span of audio attributed to one speaker.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakerTurn {
    pub speaker: SpeakerId,
    pub start: f32,
    pub end: f32,
}

/// Configuration for a (future) model-backed diarization pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct DiarizeConfig {
    pub segmentation_model: PathBuf,
    pub embedding_model: PathBuf,
    pub max_speakers: Option<usize>,
}

impl DiarizeConfig {
    /// Create a new config pointing at the segmentation and embedding models.
    pub fn new<P: AsRef<Path>>(seg: P, emb: P) -> DiarizeConfig {
        DiarizeConfig {
            segmentation_model: seg.as_ref().to_path_buf(),
            embedding_model: emb.as_ref().to_path_buf(),
            max_speakers: None,
        }
    }

    /// Cap the number of distinct speakers the pipeline may emit.
    pub fn max_speakers(mut self, n: usize) -> Self {
        self.max_speakers = Some(n);
        self
    }
}

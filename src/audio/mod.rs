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
    pub fn from_wav_file<P: AsRef<Path>>(path: P) -> Result<AudioInput> {
        let mut reader = hound::WavReader::open(path).map_err(|e| WhisperError::AudioDecode(e.to_string()))?;
        let spec = reader.spec();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max).map_err(|e| WhisperError::AudioDecode(e.to_string())))
                    .collect::<Result<Vec<_>>>()?
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .map(|s| s.map_err(|e| WhisperError::AudioDecode(e.to_string())))
                .collect::<Result<Vec<_>>>()?,
        };
        Ok(AudioInput { samples, channels: spec.channels, sample_rate: spec.sample_rate })
    }

    pub fn to_mono_16k(&self) -> Result<Vec<f32>> {
        let mono = self.downmix_mono();
        if mono.is_empty() {
            return Ok(vec![]);
        }
        if self.sample_rate == TARGET_RATE {
            return Ok(mono);
        }
        self.resample(mono)
    }

    fn downmix_mono(&self) -> Vec<f32> {
        let ch = self.channels as usize;
        if ch <= 1 {
            return self.samples.clone();
        }
        self.samples.chunks(ch).map(|frame| frame.iter().sum::<f32>() / ch as f32).collect()
    }

    fn resample(&self, mono: Vec<f32>) -> Result<Vec<f32>> {
        let ratio = TARGET_RATE as f64 / self.sample_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            oversampling_factor: 256,
            interpolation: SincInterpolationType::Linear,
            window: WindowFunction::BlackmanHarris2,
        };
        let mut rs = SincFixedIn::<f32>::new(ratio, 2.0, params, mono.len(), 1)
            .map_err(|e| WhisperError::Resample(e.to_string()))?;
        let out = rs.process(&[mono], None).map_err(|e| WhisperError::Resample(e.to_string()))?;
        Ok(out.into_iter().next().unwrap_or_default())
    }
}

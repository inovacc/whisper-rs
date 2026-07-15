//! Audio decode + normalization to whisper.cpp's required 16 kHz mono f32 PCM.
use crate::error::{Result, WhisperError};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::path::Path;

#[cfg(feature = "ffmpeg")]
pub mod media;
pub mod preprocess;
pub mod vad;

pub use preprocess::{preprocess, PreprocessLevel};
pub use vad::{segment, VadConfig};

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
        fn params() -> SincInterpolationParameters {
            SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                oversampling_factor: 256,
                interpolation: SincInterpolationType::Linear,
                window: WindowFunction::BlackmanHarris2,
            }
        }
        let expected_len = (mono.len() as f64 * ratio).round() as usize;

        // `SincFixedIn` is configured below with a single fixed chunk covering the whole
        // input, so it processes the entire signal in one `process` call rather than in a
        // streaming loop. In that shape, draining via `process_partial` (the natural flush
        // API) zero-pads to a *full extra chunk* internally and mostly returns resampled
        // silence, which isn't useful here. Instead we read `output_delay()` from a throwaway
        // probe instance (it only depends on the interpolator/ratio, not on any processed
        // data) and use it below to size a trailing zero-pad that supplies the resampler with
        // enough lookahead to emit its true tail instead of truncating it.
        let probe = SincFixedIn::<f32>::new(ratio, 2.0, params(), 1, 1).map_err(|e| WhisperError::Resample(e.to_string()))?;
        let delay = probe.output_delay();
        drop(probe);

        // Internally `SincFixedIn` already pads its working buffer by `interpolator.len()` on
        // each side, so a single-chunk call correctly produces output starting at t=0 with no
        // front-side warm-up to trim. What it does need is enough *trailing* input to compute
        // the interpolation window near the end of the chunk — without it, `process` silently
        // stops emitting once it runs out of lookahead, truncating the last `output_delay()`
        // output samples (the pre-fix bug this plan addresses). So pad the input with enough
        // trailing zeros to supply that lookahead, then trim any surplus back to the exact
        // expected length.
        let pad = (2.0 * delay as f64 / ratio).ceil() as usize + 32;
        let mut padded = mono;
        padded.resize(padded.len() + pad, 0.0);

        let mut rs = SincFixedIn::<f32>::new(ratio, 2.0, params(), padded.len(), 1)
            .map_err(|e| WhisperError::Resample(e.to_string()))?;
        let out = rs.process(&[padded], None).map_err(|e| WhisperError::Resample(e.to_string()))?;
        let mut result = out.into_iter().next().unwrap_or_default();
        if result.len() < expected_len {
            // Should not normally happen given the safety margin above; fall back to
            // padding with silence so callers always get the expected length.
            result.resize(expected_len, 0.0);
        } else {
            result.truncate(expected_len);
        }
        Ok(result)
    }
}

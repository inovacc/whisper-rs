//! Decode any media file to 16 kHz mono PCM via the ffmpeg feature and report shape.
//! Usage: cargo run --features ffmpeg --example decode -- <media-file>
fn main() -> whisper_rs::Result<()> {
    #[cfg(feature = "ffmpeg")]
    {
        let path = std::env::args().nth(1).expect("usage: decode <media-file>");
        let t0 = std::time::Instant::now();
        let pcm = whisper_rs::audio::media::decode_to_mono_16k(&path)?;
        let secs = pcm.len() as f32 / 16_000.0;
        let peak = pcm.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        eprintln!(
            "decoded {} samples = {:.1}s @ 16 kHz mono in {:.2}s (peak amplitude {:.3})",
            pcm.len(),
            secs,
            t0.elapsed().as_secs_f32(),
            peak
        );
    }
    #[cfg(not(feature = "ffmpeg"))]
    eprintln!("build with --features ffmpeg to use this example");
    Ok(())
}

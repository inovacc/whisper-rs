//! Decode arbitrary media (m4a, mp3, flac, ogg, mp4, ...) to 16 kHz mono f32 PCM via the ffmpeg
//! libraries (feature = "ffmpeg").
//!
//! Requires the ffmpeg 8.x shared + dev libraries at build time (`FFMPEG_DIR` pointing at a dir with
//! `include/` + `lib/`) and the ffmpeg DLLs on `PATH` at runtime. This is the only module that links
//! ffmpeg; it stays fully behind the `ffmpeg` feature so the default build has no native-media deps.
//!
//! Conversion (downmix to mono, resample to 16 kHz, pack as f32) goes through a libavfilter graph
//! (`abuffer -> abuffersink` with output constraints), which is the pattern ffmpeg-next itself uses —
//! the frame-based `swresample` API trips "Output changed" under the FFmpeg 8 channel-layout API.
use crate::error::{Result, WhisperError};
use ffmpeg::format::sample::{Sample, Type as SampleType};
use ffmpeg::util::channel_layout::ChannelLayout;
use ffmpeg::{filter, frame};
use ffmpeg_next as ffmpeg;
use std::path::Path;

const TARGET_RATE: u32 = 16_000;

fn ff_err<E: std::fmt::Display>(e: E) -> WhisperError {
    WhisperError::AudioDecode(format!("ffmpeg: {e}"))
}

// The "in"/"out" pads are added in `build_graph`, so a miss means an unexpected libavfilter state
// on the decode boundary — a typed error, not a panic (keeps `src/` panic-free on fallible paths).
fn missing_in() -> WhisperError {
    WhisperError::AudioDecode("ffmpeg: 'in' filter pad missing".into())
}
fn missing_out() -> WhisperError {
    WhisperError::AudioDecode("ffmpeg: 'out' filter pad missing".into())
}

/// Decode `path` (any ffmpeg-supported audio/video container) to mono f32 PCM at 16 kHz — the format
/// whisper.cpp expects. The best audio stream is used; video is ignored. Samples are in `[-1, 1]`.
pub fn decode_to_mono_16k<P: AsRef<Path>>(path: P) -> Result<Vec<f32>> {
    ffmpeg::init().map_err(ff_err)?;

    let mut ictx = ffmpeg::format::input(path.as_ref()).map_err(ff_err)?;
    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .ok_or_else(|| WhisperError::AudioDecode("ffmpeg: no audio stream in input".into()))?;
    let stream_index = input.index();

    let mut decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())
        .map_err(ff_err)?
        .decoder()
        .audio()
        .map_err(ff_err)?;

    let mut graph = build_graph(&decoder)?;
    let mut out: Vec<f32> = Vec::new();

    for (s, packet) in ictx.packets() {
        if s.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet).map_err(ff_err)?;
        pull_decoded(&mut decoder, &mut graph, &mut out)?;
    }
    decoder.send_eof().map_err(ff_err)?;
    pull_decoded(&mut decoder, &mut graph, &mut out)?;

    // Flush the filter graph so any buffered (resampler-delayed) samples are emitted.
    graph.get("in").ok_or_else(missing_in)?.source().flush().map_err(ff_err)?;
    pull_filtered(&mut graph, &mut out)?;

    Ok(out)
}

/// Build `abuffer(in) -> abuffersink(out)` with the sink constrained to packed-f32 / mono / 16 kHz;
/// libavfilter auto-inserts the needed `aresample`/`aformat` conversion.
fn build_graph(decoder: &ffmpeg::decoder::Audio) -> Result<filter::Graph> {
    let mut graph = filter::Graph::new();

    let layout_bits = decoder.channel_layout().bits();
    let ch_arg = if layout_bits != 0 {
        format!("channel_layout=0x{layout_bits:x}")
    } else {
        format!("channels={}", decoder.channel_layout().channels().max(1))
    };
    let rate = if decoder.rate() > 0 { decoder.rate() } else { TARGET_RATE };
    let args = format!("time_base=1/{rate}:sample_rate={rate}:sample_fmt={}:{ch_arg}", decoder.format().name());

    let abuffer = filter::find("abuffer").ok_or_else(|| WhisperError::AudioDecode("ffmpeg: abuffer filter missing".into()))?;
    let abuffersink =
        filter::find("abuffersink").ok_or_else(|| WhisperError::AudioDecode("ffmpeg: abuffersink filter missing".into()))?;
    graph.add(&abuffer, "in", &args).map_err(ff_err)?;
    graph.add(&abuffersink, "out", "").map_err(ff_err)?;
    {
        let mut out = graph.get("out").ok_or_else(missing_out)?;
        out.set_sample_format(Sample::F32(SampleType::Packed));
        out.set_channel_layout(ChannelLayout::MONO);
        out.set_sample_rate(TARGET_RATE);
    }
    // Explicit resample to the target rate — the sink's format/layout constraints auto-negotiate the
    // downmix + f32 packing, but the sample-rate conversion must be requested in the chain itself.
    let spec = format!("aresample={TARGET_RATE}");
    graph.output("in", 0).map_err(ff_err)?.input("out", 0).map_err(ff_err)?.parse(&spec).map_err(ff_err)?;
    graph.validate().map_err(ff_err)?;
    Ok(graph)
}

/// Pull all decodable frames, push each into the filter graph, and collect converted samples.
fn pull_decoded(decoder: &mut ffmpeg::decoder::Audio, graph: &mut filter::Graph, out: &mut Vec<f32>) -> Result<()> {
    let mut decoded = frame::Audio::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        graph.get("in").ok_or_else(missing_in)?.source().add(&decoded).map_err(ff_err)?;
        pull_filtered(graph, out)?;
    }
    Ok(())
}

/// Drain converted (packed-f32 mono 16 kHz) frames out of the sink.
fn pull_filtered(graph: &mut filter::Graph, out: &mut Vec<f32>) -> Result<()> {
    let mut filtered = frame::Audio::empty();
    while graph.get("out").ok_or_else(missing_out)?.sink().frame(&mut filtered).is_ok() {
        let n = filtered.samples();
        if n > 0 {
            let data = filtered.plane::<f32>(0);
            out.extend_from_slice(&data[..n.min(data.len())]);
        }
    }
    Ok(())
}

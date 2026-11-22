use anyhow::{Context, Result};
use log::warn;
use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};

/// Decoded PCM of an audio, an interleaved f32 samples (L R L R)
pub struct Pcm {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub channels: usize,
    pub bits: Option<u32>,
}

/// Decode an audio file to PCM in memory
pub fn decode(path: &Path) -> Result<Pcm> {
    // Open the file and create a media source stream
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    // Create a hint to help the format registry guess what format reader is appropriate using the file extension.
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("unsupported or corrupt audio file")?;

    // Get the first track and its codec
    let track = format.default_track(TrackType::Audio).context("no audio track found")?;
    let track_id = track.id;
    let params = track.codec_params.as_ref().and_then(|p| p.audio()).context("no codec is available for this track")?;

    // Get more details of the track from its codec parameters
    let rate = params.sample_rate.context("unknown sample rate")?;
    let channels = params.channels.as_ref().map(|c| c.count()).context("unknown channel count")?;
    let bits = params.bits_per_sample;

    let mut decoder = get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .context("unsupported codec")?;

    // Decode every packet and collect the samples
    let mut samples: Vec<f32> = Vec::new();
    let mut block: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet().context("failed to read packet")? {
            Some(packet) => packet,
            None => break,
        };
        if packet.track_id != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio) => {
                block.clear();
                audio.copy_to_vec_interleaved(&mut block);
                samples.extend_from_slice(&block);
            }
            Err(symphonia::core::errors::Error::DecodeError(e)) => {
                warn!("{}: skipping damaged packet: {e}", path.display())
            }
            Err(e) => return Err(e).context("failed to decode"),
        }
    }

    Ok(Pcm {
        samples,
        rate,
        channels,
        bits,
    })
}

use std::path::Path;

use anyhow::{Context, Result, bail};
use log::warn;
use opusenc::{Comments, Encoder, MappingFamily};

use crate::{Transcoder, decoder};

#[derive(Default)]
pub struct Opus {
    bitrate_kbps: Option<u32>,
}

impl Opus {
    pub fn new(bitrate_kbps: Option<u32>) -> Self {
        Self { bitrate_kbps }
    }
}

fn default_bitrate_kbps(channels: usize) -> u32 {
    match channels {
        1 => 64,
        2 => 128,
        6 => 256,
        8 => 450,
        n => {
            warn!("non-regular channel count {n}, using (64 * n) kbps");
            64 * n as u32
        }
    }
}

impl Transcoder for Opus {
    fn extension(&self) -> &str {
        "opus"
    }
    fn transcode(&self, input: &Path, output: &Path) -> Result<()> {
        // Step 1: Get the interleaved f32 PCM.
        let pcm = decoder::decode(input)?;
        if pcm.channels == 0 || pcm.channels > 2 {
            bail!(
                "only mono/stereo is supported for now (got {} channels)",
                pcm.channels
            )
        }

        // Step 2: Tags (TODO!).
        let comments = Comments::create();

        // Step 3: Create the encoder.
        // Luckily, opusenc resamples to 48kHz automatically.
        let mut encoder = Encoder::create_file(
            output,
            comments,
            pcm.rate as i32,
            pcm.channels,
            MappingFamily::MonoStereo,
        )
        .context("failed to create OPUS encoder")?;

        // Step 4: Settings
        let kbps = self
            .bitrate_kbps
            .unwrap_or_else(|| default_bitrate_kbps(pcm.channels));
        encoder
            .set_bitrate(kbps as i32 * 1000)
            .context("failed to set bitrate")?;
        encoder
            .set_vbr_enabled(true)
            .context("failed to enable VBR")?;
        encoder
            .set_vbr_constraint(false)
            .context("failed to set unconstrained VBR")?;
        encoder
            .set_complexity(10)
            .context("failed to set complexity")?;
        match pcm.bits {
            Some(bits) => encoder
                .set_lsb_depth(bits.clamp(8, 24) as i32)
                .context("failed to set LSB depth")?,
            None => warn!("unknown source bit depth, leaving LSB depth at its default"),
        }

        encoder
            .write_float(&pcm.samples)
            .context("failed to encode")?;
        encoder.drain().context("failed to finish the OPUS file")?;

        Ok(())
    }
}

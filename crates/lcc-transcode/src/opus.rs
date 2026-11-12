use std::path::Path;

use anyhow::Result;

use crate::Transcoder;

pub struct Opus {
    bitrate_kbps: u32,
}

impl Default for Opus {
    fn default() -> Self {
        Self { bitrate_kbps: 128 }
    }
}

impl Transcoder for Opus {
    fn extension(&self) -> &str {
        "opus"
    }
    fn transcode(&self, input: &Path, output: &Path) -> Result<()> {
        let _ = (self.bitrate_kbps, input, output);
        anyhow::bail!("OPUS encoding is not implemented yet")
    }
}

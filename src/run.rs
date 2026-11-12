use anyhow::{Result, bail};

use crate::cli::*;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        #[cfg(feature = "opus")]
        Some(Commands::Transcode { transcode }) => run_transcode(transcode),
        Some(Commands::Metadata { metadata: _ }) => bail!("Metadata is not implemented yet"),
        None => Ok(()),
    }
}

#[cfg(feature = "opus")]
fn run_transcode(cmd: TranscodeCommands) -> Result<()> {
    use lcc_transcode::{Transcoder, batch, opus::Opus};

    // 1. Pick the codec
    let (t, mode): (Box<dyn Transcoder>, TranscodeMode) = match cmd {
        TranscodeCommands::Opus { mode } => (Box::new(Opus::default()), mode),
    };

    // 2. Run it
    match mode {
        TranscodeMode::Single { input, output } => t.transcode(&input, &output),
        TranscodeMode::Batch {
            input_dir,
            output_dir,
        } => batch(&*t, &input_dir, &output_dir),
    }
}

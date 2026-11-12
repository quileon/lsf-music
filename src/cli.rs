use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[arg(short, long, action = ArgAction::Count, global = true)]
    /// Show more information of each action.
    pub verbose: u8,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Transcode lossless files to another format.
    #[cfg(feature = "opus")]
    Transcode {
        #[command(subcommand)]
        transcode: TranscodeCommands,
    },
    /// Get and search music metadata.
    Metadata {
        #[command(subcommand)]
        metadata: MetadataCommands,
    },
}

#[cfg(feature = "opus")]
#[derive(Subcommand)]
pub enum TranscodeCommands {
    /// Transcode to OPUS.
    #[cfg(feature = "opus")]
    Opus {
        #[command(subcommand)]
        mode: TranscodeMode,
    },
}

#[cfg(feature = "opus")]
#[derive(Subcommand)]
pub enum TranscodeMode {
    /// Transcode one file.
    Single { input: PathBuf, output: PathBuf },
    /// Transcode every file in a directory, keeping its structure.
    Batch {
        input_dir: PathBuf,
        output_dir: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum MetadataCommands {
    /// Get lyrics metadata.
    Lyrics {
        #[command(subcommand)]
        lyrics: LyricsCommands,
    },
}

#[derive(Subcommand)]
pub enum LyricsCommands {
    /// Get lyrics based on music metadata.
    Get {
        #[arg(short, long)]
        music: PathBuf,
    },
    /// Get lyrics based on the inputted text.
    Search {
        #[arg(short, long)]
        text: String,
    },
}

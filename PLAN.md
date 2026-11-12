# Plan: Opus transcoding

Reference for building the transcode feature step by step. The goal is a workspace
where library crates (`lcc-transcode`, later `lcc-lyrics`) have no clap dependency
and the binary owns all CLI types, with Cargo features gating each format.

## Progress

- [x] 1. Create `crates/lcc-transcode`
- [x] 2. `crates/lcc-transcode/Cargo.toml` with the `opus` feature
- [x] 3. `lib.rs`: `Transcoder` trait + `batch()` (recursive walkdir, `enum Job` queue)
- [x] 4. `opus.rs`: `Opus` stub (returns `bail!` error)
- [x] 5. Root `Cargo.toml`: features + dependency
- [x] 6. `src/cli.rs`: `TranscodeCommands` + `TranscodeMode`
- [x] 7. `src/run.rs`: dispatch
- [x] 8. Build and check (both feature sets build clean; batch/single/metadata return clean errors)
- [ ] 9. Choose an encoder and fill in `Opus::transcode` (deferred, possibly the last thing done; stub returns an error until then)

## Next phase

- [x] Logging: `log` in libraries, `env_logger` in the binary, `-v`/`-vv` (`ArgAction::Count`, global)
- [x] Batch continues on per-file errors (`run_job`, `error!` + count, fail at the end, partial output removed)
- [ ] Opus encoder (step 9), moved before tests
- [ ] Tests, written once the encoder's output is known: `batch` (temp dir + fake `Transcoder`)
      and encoder properties via `lofty` as a dev-dependency

## Encoder pipeline (chosen: libopusenc via the `opusenc` crate)

Decoding and tag reading are shared by every codec, so they live outside the `opus` feature:

- `decode.rs` (always compiled): `symphonia` 0.6 (features: flac, alac, isomp4). It opens any
  supported input and yields the spec (rate, channels, bit depth) plus interleaved `f32` blocks.
  Opus *input* isn't supported by symphonia; that needs a libopus decoder later (low priority).
- `tags.rs` (LATER, not now): `lofty` reads tags and pictures from FLAC, MP4/ALAC and Opus into
  generic items, mapped to Vorbis comment keys for Opus/FLAC output. Until then, `Comments::create()` stays empty.
- `opus.rs` (`opus` feature): `opus = ["dep:opusenc"]`, with `opusenc` built with
  `features = ["encoder-options"]`. System library `libopusenc` (installed, 0.3); it resamples internally.

`Opus::transcode`:
1. Tags and cover from `tags.rs` go into `opusenc::Comments` (`add_string`, `add_picture_from_memory`).
2. Open the input with `decode.rs` to get the rate, channels and bits.
3. `Encoder::create_file(output, &comments, rate, channels, MappingFamily::MonoStereo)`, then
   `set_bitrate(self.bitrate_kbps * 1000)`, `set_serialno(fixed)`, `set_lsb_depth(bits)`.
4. For each decoded block, call `write_float`.
5. `drain()`.

`batch` input filter: extend from `flac` to the supported input extensions (flac, m4a, ...).

## Encoder requirements

- Opus at 128 kbps
- Same Vorbis comments as the source FLAC
- Cover art carried over (in Opus it's a base64 `METADATA_BLOCK_PICTURE` comment)
- Fixed Ogg serial (or configurable) so outputs are reproducible
- Output must be valid Opus with the same channel count and about the same duration

## Layout

```
lsf-music/
├── Cargo.toml              # binary + [workspace]
├── src/                    # main.rs, cli.rs, run.rs
└── crates/
    ├── lcc-lyrics/         # later: standalone, reusable from a GTK app
    └── lcc-transcode/
        ├── Cargo.toml      # features: opus (later flac)
        └── src/
            ├── lib.rs      # Transcoder trait + batch()
            ├── opus.rs     # #[cfg(feature = "opus")]
            └── flac.rs     # later, #[cfg(feature = "flac")]
```

Rule: library crates never depend on clap. CLI types stay in `src/cli.rs`.

## 1. Create the crate

```sh
cargo new --lib crates/lcc-transcode
```

## 2. `crates/lcc-transcode/Cargo.toml`

```toml
[package]
name = "lcc-transcode"
version = "0.1.0"
edition = "2024"

[features]
opus = []          # later: ["dep:<encoder crate>"]

[dependencies]
anyhow = "1"
```

## 3. `crates/lcc-transcode/src/lib.rs`

Flat version (one directory, no recursion):

```rust
use std::path::Path;

use anyhow::Result;

#[cfg(feature = "opus")]
pub mod opus;

/// A format that can transcode a single lossless file.
pub trait Transcoder {
    /// Extension of the output files, without the dot.
    fn extension(&self) -> &str;
    fn transcode(&self, input: &Path, output: &Path) -> Result<()>;
}

/// Transcode every file in `input_dir` into `output_dir`.
pub fn batch(t: &dyn Transcoder, input_dir: &Path, output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    for entry in std::fs::read_dir(input_dir)? {
        let input = entry?.path();
        if !input.is_file() {
            continue;
        }
        let Some(stem) = input.file_stem() else { continue };
        let output = output_dir.join(stem).with_extension(t.extension());
        t.transcode(&input, &output)?;
    }
    Ok(())
}
```

## 4. `crates/lcc-transcode/src/opus.rs`

```rust
use std::path::Path;

use anyhow::Result;

use crate::Transcoder;

/// Opus encoder. Its settings stay private; users only get `Default`.
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
        let _ = self.bitrate_kbps;
        todo!("encode {} -> {}", input.display(), output.display())
    }
}
```

## 5. Root `Cargo.toml`

```toml
[features]
default = ["opus"]
opus = ["lcc-transcode/opus"]

[dependencies]
anyhow = "1.0.104"
clap = { version = "4.6.7", features = ["derive"] }
lcc-transcode = { path = "crates/lcc-transcode", default-features = false }
```

## 6. `src/cli.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Transcode lossless files to another format.
    #[cfg(feature = "opus")]          // later: any(feature = "opus", feature = "flac")
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

#[derive(Subcommand)]
pub enum TranscodeCommands {
    /// Transcode to Opus.
    #[cfg(feature = "opus")]
    Opus {
        #[command(subcommand)]
        mode: TranscodeMode,
    },
}

#[derive(Subcommand)]
pub enum TranscodeMode {
    /// Transcode one file.
    Single { input: PathBuf, output: PathBuf },
    /// Transcode every file in a directory.
    Batch { input_dir: PathBuf, output_dir: PathBuf },
}
```

Delete `OpusCommands`; `TranscodeMode` replaces it.

## 7. `src/run.rs`

```rust
use anyhow::Result;

use crate::cli::*;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        #[cfg(feature = "opus")]
        Some(Commands::Transcode { transcode }) => transcode_cmd(transcode),
        Some(Commands::Metadata { .. }) => todo!("lyrics"),
        None => Ok(()),
    }
}

#[cfg(feature = "opus")]
fn transcode_cmd(cmd: TranscodeCommands) -> Result<()> {
    use lcc_transcode::{Transcoder, batch};

    let (t, mode): (Box<dyn Transcoder>, TranscodeMode) = match cmd {
        TranscodeCommands::Opus { mode } => (Box::new(lcc_transcode::opus::Opus::default()), mode),
    };

    match mode {
        TranscodeMode::Single { input, output } => t.transcode(&input, &output),
        TranscodeMode::Batch { input_dir, output_dir } => batch(t.as_ref(), &input_dir, &output_dir),
    }
}
```

`Box<dyn Transcoder>` is there so adding FLAC is one extra match arm.

## 8. Check

```sh
cargo build
cargo build --no-default-features      # transcode should vanish
cargo run -- transcode opus --help
```

## 9. Encoder options

- **External program** (`opusenc` / `ffmpeg`) via `std::process::Command`: no Rust deps,
  takes FLAC directly, copies tags. Quickest; users must have the program installed.
- **Pure Rust**: decode with `symphonia` or `claxon`, encode with `audiopus` (libopus),
  mux with `ogg`. Self-contained, but more work (resample to 48 kHz, copy metadata).

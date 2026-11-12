use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use log::{debug, error, info};
use walkdir::WalkDir;

#[cfg(feature = "opus")]
pub mod opus;

/// The requirement for codec to transcode a single file.
///
/// - `extension` method returns the extension of current codec.
/// - `transcode` method transcodes a single file from `input` to `output`.
pub trait Transcoder {
    fn extension(&self) -> &str;
    fn transcode(&self, input: &Path, output: &Path) -> Result<()>;
}

/// The individual job queue.
#[allow(dead_code)]
enum Job {
    Transcode { input: PathBuf, output: PathBuf },
    Copy { input: PathBuf, output: PathBuf },
}

/// Transcode every file in `input_dir` into `output_dir`
pub fn batch(t: &dyn Transcoder, input_dir: &Path, output_dir: &Path) -> Result<()> {
    let mut jobs: Vec<Job> = Vec::new();

    // Step 1: Get all files in input_dir
    for entry in WalkDir::new(input_dir) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        let input = entry.path();
        if !input.extension().is_some_and(|ext| ext == "flac") {
            continue;
        }

        let output = output_dir
            .join(input.strip_prefix(input_dir)?)
            .with_extension(t.extension());
        if output.exists() {
            debug!("skip {}: already exists", output.display());
            continue;
        }

        jobs.push(Job::Transcode {
            input: input.to_path_buf(),
            output,
        });
    }
    info!("{} job(s) to run", jobs.len());

    // Step 2: Transcode collected files to output_dir
    let mut failed = 0;
    for job in &jobs {
        if let Err(e) = run_job(t, job) {
            error!("{e:#}");
            failed += 1;
        }
    }

    if failed > 0 {
        bail!("{failed} or {} job(s) failed", jobs.len());
    }

    Ok(())
}

fn run_job(t: &dyn Transcoder, job: &Job) -> Result<()> {
    match job {
        Job::Copy { input, output } => {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            info!("copy {} -> {}", input.display(), output.display());
            fs::copy(input, output)
                .with_context(|| format!("failed to copy {}", input.display()))?;
        }
        Job::Transcode { input, output } => {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            info!("transcode {} -> {}", input.display(), output.display());
            if let Err(e) = t.transcode(input, output) {
                let _ = fs::remove_file(output);
                return Err(e.context(format!("failed to transcode {}", input.display())));
            }
        }
    }

    Ok(())
}

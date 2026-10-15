use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

#[derive(Debug, Parser)]
#[command(version, about = "Utilities for installing Minecraft modpacks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Extract an .mrpack archive into a directory for inspection
    Extract {
        /// Path to the .mrpack file
        pack: PathBuf,
        /// Directory where the archive contents will be extracted
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn extract_mrpack(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("opening {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)?;
    fs::create_dir_all(output_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        let Some(relative_path) = entry.enclosed_name() else {
            bail!("unsafe path in archive: {:#?}", entry.name());
        };

        let destination = output_dir.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&destination)?;
            io::copy(&mut entry, &mut output)?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { pack, output } => {
            extract_mrpack(&pack, &output)?;
            println!("Extracted {} to {}", pack.display(), output.display());
        }
    }

    Ok(())
}
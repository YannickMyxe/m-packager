use crate::archive::{extract_mrpack, read_index};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use crate::index::ModrinthIndex;

#[derive(Debug, Parser)]
#[command(version, about = "Utilities for installing Minecraft modpacks")]
struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
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

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { pack, output } => {
            _extract(&pack, &output)?;
        }
    }

    Ok(())
}

fn _extract(pack: &Path, output_dir: &Path) -> anyhow::Result<()> {
    // First validate the index before extracting
    let index = read_index(pack)?;
    index.validate()?;

    // With the pack validated, extract it in the out-dir
    extract_mrpack(&pack, &output_dir)?;
    println!("Extracted {} to {}", pack.display(), output_dir.display());
    println!("Pack: {}", index);

    Ok(())
}
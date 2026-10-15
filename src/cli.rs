use crate::archive::extract_mrpack;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
            extract_mrpack(&pack, &output)?;
            println!("Extracted {} to {}", pack.display(), output.display());
        }
    }
    Ok(())
}

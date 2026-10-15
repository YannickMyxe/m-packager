use crate::archive::{extract_mrpack, read_index};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

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
    /// Validate an .mrpack archive and check if it supports the correct version and game
    Validate {
        /// Path to the .mrpack file
        pack: PathBuf,
        /// Minecraft version to compare with the pack
        #[arg(long, visible_alias = "mc-version")]
        minecraft_version: Option<String>,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { pack, output } => {
            _extract(&pack, &output)?;
        }
        Commands::Validate {
            pack,
            minecraft_version,
        } => match minecraft_version {
            Some(minecraft_version) => {
                _validate_with_minecraft_version(&pack, minecraft_version)?;
            }
            None => {
                _validate(&pack)?;
            }
        },
    }

    Ok(())
}

fn _validate(pack: &PathBuf) -> anyhow::Result<()> {
    let index = read_index(pack)?;
    index.validate()?;
    println!("Modpack validated successfully");
    Ok(())
}

fn _validate_with_minecraft_version(pack: &PathBuf, mc_version: String) -> anyhow::Result<()> {
    let index = read_index(pack)?;
    index.validate()?;
    match index.dependencies().get("minecraft") {
        Some(version) => {
            if version != &mc_version {
                anyhow::bail!(
                    "Minecraft version do not match, expected: {mc_version}, but the pack supports {version}"
                )
            }
        }
        None => anyhow::bail!("Minecraft version not found"),
    }
    println!("Modpack validated successfully, confirmed Minecraft version {mc_version}");
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

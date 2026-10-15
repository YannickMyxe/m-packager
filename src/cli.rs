use crate::archive::{extract_mrpack, read_index};
use crate::index::{EnvironmentSupport, ModInfo};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(version, about = "Utilities for installing Minecraft modpacks")]
struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ModSort {
    Name,
    ServerSupport,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
enum SortOrder {
    #[default]
    Asc,
    Desc,
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
    /// Shows the mrpack's name, version and summary with optional flags for showing mods
    Info {
        /// Path to the .mrpack file
        pack: PathBuf,
        /// If mods should be printed or not
        #[arg(long, short = 'm')]
        mods: bool,
        /// If it should only print the Minecraft and mod-loader versions
        #[arg(long, short = 's')]
        short: bool,
        ///
        #[arg(long, value_enum, requires = "mods")]
        sort: Option<ModSort>,
        ///
        #[arg(long, value_enum, requires = "sort", default_value_t = SortOrder::Asc)]
        order: SortOrder,
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
        Commands::Info {
            pack,
            mods,
            short,
            sort,
            order,
        } => {
            _info(&pack, mods, short, sort, order)?;
        }
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

    Ok(())
}

fn _info(
    pack: &Path,
    mods: bool,
    short: bool,
    sort: Option<ModSort>,
    order: SortOrder,
) -> anyhow::Result<()> {
    let index = read_index(pack)?;
    index.validate()?;

    let minecraft = index
        .minecraft_version()
        .ok_or_else(|| anyhow::anyhow!("Minecraft version not found"))?;
    let neoforge = index
        .neoforge_version()
        .ok_or_else(|| anyhow::anyhow!("NeoForge version not found"))?;

    if !short {
        println!("{} {}", index.name(), index.version());
        if let Some(summary) = index.summary() {
            println!("Summary: {summary}");
        }
    }
    println!("Minecraft: {minecraft}, NeoForge: {neoforge}");
    if mods {
        let mut mods: Vec<ModInfo> = index
            .mods()
            .filter(|m| !matches!(m.server, EnvironmentSupport::Unsupported))
            .collect();
        if let Some(sort_by) = sort {
            mods.sort_by(|a, b| {
                let ordering = match sort_by {
                    ModSort::Name => a
                        .name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase()),
                    ModSort::ServerSupport => _support_rank(a.server)
                        .cmp(&_support_rank(b.server))
                        .then_with(|| a.name.cmp(b.name)),
                };

                match order {
                    SortOrder::Asc => ordering,
                    SortOrder::Desc => ordering.reverse(),
                }
            });
        }
        println!("Mods:");
        for m in mods {
            println!("{} (server: {})", m.name, m.server);
        }
    }

    Ok(())
}

fn _support_rank(support: &EnvironmentSupport) -> u8 {
    match support {
        EnvironmentSupport::Required => 0,
        EnvironmentSupport::Optional => 1,
        EnvironmentSupport::Unsupported => 2,
    }
}

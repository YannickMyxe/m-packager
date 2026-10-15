use anyhow::{Context, bail};
use std::fs::File;
use std::path::Path;
use std::{fs, io};
use zip::ZipArchive;

pub fn extract_mrpack(archive_path: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let file =
        File::open(archive_path).with_context(|| format!("opening {}", archive_path.display()))?;
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

use anyhow::{Context, bail};
use std::fs::File;
use std::path::Path;
use std::{fs, io};
use zip::ZipArchive;
use crate::index::ModrinthIndex;
use crate::paths::validate_file_path;

pub fn extract_mrpack(archive_path: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let file =
        File::open(archive_path).with_context(|| format!("opening {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)?;

    // Preflight: reject unsafe entries before creating or writing output files.
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;

        let Some(_) = entry.enclosed_name() else {
            bail!("unsafe path in archive: {:?}", entry.name()?);
        };

        let entry_name = entry.name()?;
        let entry_name = entry_name.as_ref();

        let path_to_validate = if entry.is_dir() {
            entry_name.strip_suffix('/').unwrap_or(entry_name)
        } else {
            entry_name
        };

        validate_file_path(path_to_validate)
            .with_context(|| format!("validating archive entry {entry_name:?}"))?;
    }

    // Extraction: preflight succeeded, so now it's safe to create output.
    fs::create_dir_all(output_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        let Some(relative_path) = entry.enclosed_name() else {
            bail!("unsafe path in archive: {:?}", entry.name()?);
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

pub fn read_index(archive_path: &Path) -> anyhow::Result<ModrinthIndex> {
    let file =
        File::open(archive_path).with_context(|| format!("opening {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)?;
    let index_file = archive
        .by_name("modrinth.index.json")
        .context("finding modrinth.index.json in archive")?;

    let index: ModrinthIndex = serde_json::from_reader(index_file)?;
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn write_archive(path: &Path, include_unsafe_entry: bool) {
        let file = File::create(path).expect("create test archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .add_directory("config/", options)
            .expect("add directory entry");
        writer
            .start_file("config/server.properties", options)
            .expect("start file entry");
        writer
            .write_all(b"motd=test")
            .expect("write file contents");

        if include_unsafe_entry {
            writer
                .start_file("../outside.txt", options)
                .expect("start unsafe file entry");
            writer
                .write_all(b"should not be extracted")
                .expect("write unsafe file contents");
        }

        writer.finish().expect("finish test archive");
    }

    #[test]
    fn extracts_directory_entries() {
        let temp = tempdir().expect("create temporary directory");
        let archive_path = temp.path().join("valid.mrpack");
        let output_dir = temp.path().join("output");
        write_archive(&archive_path, false);

        extract_mrpack(&archive_path, &output_dir).expect("extract valid archive");

        let extracted_file = fs::read_to_string(output_dir.join("config/server.properties"))
            .expect("read extracted file");
        assert_eq!(extracted_file, "motd=test");
    }

    #[test]
    fn rejects_unsafe_entry_before_creating_output() {
        let temp = tempdir().expect("create temporary directory");
        let archive_path = temp.path().join("unsafe.mrpack");
        let output_dir = temp.path().join("output");
        write_archive(&archive_path, true);

        assert!(extract_mrpack(&archive_path, &output_dir).is_err());
        assert!(!output_dir.exists());
        assert!(!temp.path().join("outside.txt").exists());
    }
}

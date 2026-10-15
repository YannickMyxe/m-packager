use crate::index::ModrinthIndex;
use crate::paths::{normalize_file_path, validate_file_path};
use anyhow::{Context, bail};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::{fs, io};
use zip::ZipArchive;

const TOTAL_SIZE_LIMIT: u64 = 25_000_000;
const TOTAL_FILE_SIZE_LIMIT: u64 = 10_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryKind {
    File,
    Directory,
}

type SeenPaths = HashMap<String, (EntryKind, String)>;

pub fn extract_mrpack(archive_path: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let file =
        File::open(archive_path).with_context(|| format!("opening {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)?;

    let mut seen_paths: SeenPaths = HashMap::new();
    let mut total_uncompressed_bytes = 0u64;

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

        register_path(&mut seen_paths, path_to_validate, entry.is_dir())
            .with_context(|| format!("checking archive entry {entry_name:?}"))?;

        let entry_size = entry.size();
        if entry_size > TOTAL_FILE_SIZE_LIMIT {
            bail!(
                "archive entry {entry_name:?} is {entry_size} bytes, exceeding the per-file limit of {TOTAL_FILE_SIZE_LIMIT} bytes by {} bytes",
                entry_size - TOTAL_FILE_SIZE_LIMIT
            );
        }

        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(entry_size)
            .context("total uncompressed archive size overflow")?;
    }

    if total_uncompressed_bytes > TOTAL_SIZE_LIMIT {
        bail!(
            "total uncompressed archive size is {total_uncompressed_bytes} bytes, exceeding the limit of {TOTAL_SIZE_LIMIT} bytes by {} bytes",
            total_uncompressed_bytes - TOTAL_SIZE_LIMIT
        );
    }

    // Extraction: preflight succeeded, so now it's safe to create output.
    fs::create_dir_all(output_dir)?;

    let mut actual_total_bytes = 0u64;
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
            let declared_size = entry.size();
            let remaining_total_limit = TOTAL_SIZE_LIMIT - actual_total_bytes;
            let copy_limit = TOTAL_FILE_SIZE_LIMIT.min(remaining_total_limit);

            let copied_bytes = {
                let mut limited_entry = (&mut entry).take(copy_limit);
                io::copy(&mut limited_entry, &mut output)?
            };

            if copied_bytes == copy_limit {
                let mut extra_byte = [0u8; 1];
                if entry.read(&mut extra_byte)? != 0 {
                    bail!(
                        "archive entry {:?} exceeds the extraction size limit",
                        entry.name()?
                    );
                }
            }

            if copied_bytes != declared_size {
                bail!(
                    "archive entry {:?} declared {declared_size} bytes but extracted {copied_bytes} bytes",
                    entry.name()?
                );
            }

            actual_total_bytes = actual_total_bytes
                .checked_add(copied_bytes)
                .context("actual extracted archive size overflow")?;
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

fn register_path(paths: &mut SeenPaths, path: &str, is_dir: bool) -> anyhow::Result<()> {
    let key = normalize_file_path(path);
    let kind = if is_dir {
        EntryKind::Directory
    } else {
        EntryKind::File
    };

    // Any second entry for the same case-insensitive path is a conflict.
    if let Some((_, previous_name)) = paths.get(&key) {
        anyhow::bail!("duplicate archive path: {path:?} conflicts with {previous_name:?}");
    }

    // A file cannot be an ancestor of another entry:
    // `config` as a file conflicts with `config/settings.txt`.
    let mut parent = String::new();
    let mut components = key.split('/').peekable();
    while let Some(component) = components.next() {
        if components.peek().is_some() {
            if !parent.is_empty() {
                parent.push('/');
            }
            parent.push_str(component);

            if let Some((EntryKind::File, previous_name)) = paths.get(&parent) {
                anyhow::bail!("archive path {path:?} requires {previous_name:?} to be a directory");
            }
        }
    }

    // The reverse order also matters: a file cannot replace a directory
    // path already needed by one of its children.
    if kind == EntryKind::File {
        let child_prefix = format!("{key}/");
        if let Some((_, previous_name)) = paths
            .iter()
            .find(|(existing, _)| existing.starts_with(&child_prefix))
            .map(|(_, value)| value)
        {
            anyhow::bail!("archive file {path:?} conflicts with child entry {previous_name:?}");
        }
    }

    paths.insert(key, (kind, path.to_owned()));
    Ok(())
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
        writer.write_all(b"motd=test").expect("write file contents");

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

    fn write_path_entries(path: &Path, entries: &[(&str, bool)]) {
        let file = File::create(path).expect("create test archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (name, is_dir) in entries {
            if *is_dir {
                writer.add_directory(*name, options).expect("add directory");
            } else {
                writer.start_file(*name, options).expect("add file");
                writer.write_all(b"test").expect("write file contents");
            }
        }

        writer.finish().expect("finish test archive");
    }

    fn write_sized_archive(path: &Path, entries: &[(&str, u64)]) {
        let file = File::create(path).expect("create test archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let block = [0u8; 64 * 1024];

        for (name, size) in entries {
            writer.start_file(*name, options).expect("add file");
            let mut remaining = *size;
            while remaining > 0 {
                let chunk_size = remaining.min(block.len() as u64) as usize;
                writer
                    .write_all(&block[..chunk_size])
                    .expect("write file contents");
                remaining -= chunk_size as u64;
            }
        }

        writer.finish().expect("finish test archive");
    }

    fn assert_conflicting_archive_is_rejected(entries: &[(&str, bool)]) {
        let temp = tempdir().expect("create temporary directory");
        let archive_path = temp.path().join("conflict.mrpack");
        let output_dir = temp.path().join("output");
        write_path_entries(&archive_path, entries);

        assert!(extract_mrpack(&archive_path, &output_dir).is_err());
        assert!(!output_dir.exists());
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

    #[test]
    fn rejects_entry_over_per_file_size_limit_before_creating_output() {
        let temp = tempdir().expect("create temporary directory");
        let archive_path = temp.path().join("oversized-file.mrpack");
        let output_dir = temp.path().join("output");
        write_sized_archive(
            &archive_path,
            &[("large.bin", TOTAL_FILE_SIZE_LIMIT + 1)],
        );

        let error = extract_mrpack(&archive_path, &output_dir).unwrap_err();

        assert!(error.to_string().contains("exceeding the per-file limit"));
        assert!(error.to_string().contains("1 bytes"));
        assert!(!output_dir.exists());
    }

    #[test]
    fn rejects_archive_over_total_size_limit_before_creating_output() {
        let temp = tempdir().expect("create temporary directory");
        let archive_path = temp.path().join("oversized-total.mrpack");
        let output_dir = temp.path().join("output");
        let third_file_size = TOTAL_SIZE_LIMIT - 2 * TOTAL_FILE_SIZE_LIMIT + 1;
        write_sized_archive(
            &archive_path,
            &[
                ("first.bin", TOTAL_FILE_SIZE_LIMIT),
                ("second.bin", TOTAL_FILE_SIZE_LIMIT),
                ("third.bin", third_file_size),
            ],
        );

        let error = extract_mrpack(&archive_path, &output_dir).unwrap_err();

        assert!(error.to_string().contains("exceeding the limit"));
        assert!(
            error
                .to_string()
                .contains(&(TOTAL_SIZE_LIMIT + 1).to_string())
        );
        assert!(error.to_string().contains("by 1 bytes"));
        assert!(!output_dir.exists());
    }

    #[test]
    fn rejects_case_insensitive_duplicate_paths() {
        assert_conflicting_archive_is_rejected(&[
            ("Config/settings.txt", false),
            ("config/SETTINGS.txt", false),
        ]);
    }

    #[test]
    fn rejects_file_directory_collision_when_file_comes_first() {
        assert_conflicting_archive_is_rejected(&[
            ("config", false),
            ("config/settings.txt", false),
        ]);
    }

    #[test]
    fn rejects_file_directory_collision_when_child_comes_first() {
        assert_conflicting_archive_is_rejected(&[
            ("config/settings.txt", false),
            ("config", false),
        ]);
    }
}

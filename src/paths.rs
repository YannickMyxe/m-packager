
pub fn validate_file_path(path: &str) -> anyhow::Result<()> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        anyhow::bail!("unsafe file path in modrinth.index.json: {path:?}");
    }

    Ok(())
}

pub fn normalize_file_path(path: &str) -> String {
    path.to_lowercase()
}
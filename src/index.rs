use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndex {
    format_version: u32,
    game: String,
    name: String,
    version_id: String,
    summary: Option<String>,
    dependencies: HashMap<String, String>,
    files: Vec<ModrinthIndexFile>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndexFile {
    downloads: Vec<String>,
    env: ModrinthIndexEnv,
    hashes: ModrinthHashes,
    path: String,
    file_size: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndexEnv {
    server: EnvironmentSupport,
    client: EnvironmentSupport,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentSupport {
    Required,
    Optional,
    Unsupported,
}

#[derive(Debug, Deserialize, Clone)]
struct ModrinthHashes {
    sha1: String,
    sha512: String,
}

impl ModrinthIndex {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.format_version != 1 {
            anyhow::bail!("Unsupported modrinth format version, we only supports version 1, {}", self.format_version);
        }

        if self.game != "minecraft" {
            anyhow::bail!("Unsupported game, we only support minecraft, {}", self.game);
        }

        for dependency in ["minecraft", "neoforge"] {
            match self.dependencies.get(dependency) {
                Some(version) if !version.trim().is_empty() => {}
                _ => anyhow::bail!("Missing or empty required dependency `{dependency}`"),
            }
        }

        Ok(())
    }

    pub fn dependencies(&self) -> &HashMap<String, String> {
        &self.dependencies
    }

    pub fn mods(&self) -> impl Iterator<Item = &str> + '_ {
        self.files.iter().filter_map(|file| {
            let filename = file.path.rsplit('/').next()?;
            filename.strip_suffix(".jar")
        })
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn version(&self) -> &str {
        self.version_id.as_str()
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }
}

impl fmt::Display for ModrinthIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}, version: {}", self.name, self.version_id)?;
        if let Some(summary) = self.summary.as_ref() {
            writeln!(f, "{}", summary)?;
        }
        writeln!(f, "Dependencies:")?;
        for (key, value) in self.dependencies.iter() {
            write!(f, "{{ {}: {} }}, ", key, value)?;
        }
        writeln!(f, "")?;
        writeln!(f, "Files:")?;
        if !self.files.is_empty() {
            for (index, file) in self.files.iter().enumerate() {
                writeln!(f, "[{}]: {}", index, file)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for ModrinthIndexFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Path: {}", self.path)?;
        writeln!(f, "Size: {} bytes", self.file_size)?;
        for download in &self.downloads {
            writeln!(f, "{}", download)?;
        }
        writeln!(f, "Env: {}", self.env)?;
        writeln!(f, "Hashes: {}", self.hashes)?;
        Ok(())
    }
}

impl fmt::Display for ModrinthIndexEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{ server: {}, client: {} }}", self.server, self.client)?;
        Ok(())
    }
}

impl fmt::Display for EnvironmentSupport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            EnvironmentSupport::Required => "Required",
            EnvironmentSupport::Optional => "Optional",
            EnvironmentSupport::Unsupported => "Unsupported",
        };
        write!(f, "{}", label)
    }
}

impl fmt::Display for ModrinthHashes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "sha1: {}", self.sha1)?;
        write!(f, "sha512: {}", self.sha512)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_with(format_version: u32, game: &str) -> ModrinthIndex {
        serde_json::from_value(serde_json::json!({
            "formatVersion": format_version,
            "game": game,
            "name": "Test pack",
            "versionId": "1",
            "dependencies": {
                "minecraft": "unknown version",
                "neoforge": "unknown version",
            },
            "files": []
        }))
            .expect("test JSON should deserialize")
    }

    #[test]
    fn accepts_supported_index() {
        let index = index_with(1, "minecraft");

        assert!(index.validate().is_ok());
    }

    #[test]
    fn rejects_unsupported_format_version() {
        let index = index_with(2, "minecraft");

        assert!(index.validate().is_err());
    }

    #[test]
    fn rejects_unsupported_game() {
        let index = index_with(1, "some-other-game");

        assert!(index.validate().is_err());
    }

    #[test]
    fn rejects_missing_minecraft_dependency() {
        let mut index = index_with(1, "minecraft");
        index.dependencies.remove("minecraft");

        assert!(index.validate().is_err());
    }

    #[test]
    fn rejects_empty_neoforge_version() {
        let mut index = index_with(1, "minecraft");
        index.dependencies.insert("neoforge".to_string(), "".to_string());

        assert!(index.validate().is_err());
    }
}
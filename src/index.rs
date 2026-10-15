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
    downloads: Vec<serde_json::Value>,
    env: ModrinthIndexEnv,
    hashes: HashMap<String, String>,
    path: String,
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

impl fmt::Display for ModrinthIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}, version: {}", self.name, self.version_id)?;
        if let Some(summary) = self.summary.as_ref() {
            writeln!(f, "{}", summary)?;
        }
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
        writeln!(f, "Download items: {}", self.downloads.len())?;
        writeln!(f, "Env items: {}", self.env)?;
        writeln!(f, "Hashes items: {}", self.hashes.len())?;
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

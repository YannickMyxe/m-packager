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
    files: Vec<serde_json::Value>,
}

impl fmt::Display for ModrinthIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}, version: {}", self.name, self.version_id)?;
        if let Some(summary) = self.summary.as_ref() {
            writeln!(f, "{}", summary)?;
        };

        write!(f, "")
    }
}

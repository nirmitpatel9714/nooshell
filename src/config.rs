use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageConfig {
    pub cmd: String,
    pub args: Vec<String>,
    pub mode: String, // "repl" or "compile-run"
}

pub type ConfigMap = HashMap<String, LanguageConfig>;

pub fn load_config<P: AsRef<Path>>(path: P) -> std::io::Result<ConfigMap> {
    let content = fs::read_to_string(path)?;
    let config: ConfigMap = serde_json::from_str(&content)?;
    Ok(config)
}

pub fn load_from_str(content: &str) -> std::io::Result<ConfigMap> {
    let config: ConfigMap = serde_json::from_str(content)?;
    Ok(config)
}

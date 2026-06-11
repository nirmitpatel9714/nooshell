use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for a single language REPL.
///
/// Deserialized from `languages.json`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageConfig {
    /// The executable to run (e.g., `"python"`, `"node"`).
    pub cmd: String,
    /// Arguments passed to the executable (e.g., `["-i"]`).
    pub args: Vec<String>,
    /// Execution mode. Currently always `"repl"`.
    pub mode: String,
}

/// Map of language alias (e.g., `"py"`, `"js"`) to [`LanguageConfig`].
pub type ConfigMap = HashMap<String, LanguageConfig>;

/// Shell configuration entry from `sh-languages.json`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShellEntry {
    /// Executable names to search in PATH, in priority order.
    pub executables: Vec<String>,
    /// Default arguments to pass to the shell.
    #[serde(default)]
    pub args: Vec<String>,
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Map of shell alias (e.g., `"bash"`, `"zsh"`) to [`ShellEntry`].
pub type ShellConfigMap = HashMap<String, ShellEntry>;

/// Returns the platform-consistent config directory: `~/.config/noo/`.
///
/// On Windows this resolves to `%USERPROFILE%\.config\noo\`.
/// On Unix this resolves to `$HOME/.config/noo/`.
pub fn config_dir() -> PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join(".config").join("noo")
}

/// Load language config from `~/.config/noo/languages.json` with fallback.
///
/// Priority:
/// 1. `~/.config/noo/languages.json`
/// 2. `./config/languages.json` (working directory, for backward compatibility)
/// 3. Embedded default string
pub fn load_language_config(embedded: &str) -> ConfigMap {
    let dir_path = config_dir().join("languages.json");
    if let Ok(content) = fs::read_to_string(&dir_path) {
        return serde_json::from_str(&content).unwrap_or_default();
    }
    if let Ok(content) = fs::read_to_string("config/languages.json") {
        return serde_json::from_str(&content).unwrap_or_default();
    }
    serde_json::from_str(embedded).unwrap_or_default()
}

/// Load shell config from `~/.config/noo/sh-languages.json` with fallback.
///
/// Priority:
/// 1. `~/.config/noo/sh-languages.json`
/// 2. Embedded default string
pub fn load_shell_config(embedded: &str) -> ShellConfigMap {
    let dir_path = config_dir().join("sh-languages.json");
    if let Ok(content) = fs::read_to_string(&dir_path) {
        return serde_json::from_str(&content).unwrap_or_else(|_| {
            serde_json::from_str(embedded).expect("embedded sh-languages.json is valid")
        });
    }
    serde_json::from_str(embedded).expect("embedded sh-languages.json is valid")
}

/// Load language configuration from a JSON file path (legacy).
pub fn load_config<P: AsRef<Path>>(path: P) -> std::io::Result<ConfigMap> {
    let content = fs::read_to_string(path)?;
    let config: ConfigMap = serde_json::from_str(&content)?;
    Ok(config)
}

/// Load language configuration from a raw JSON string.
pub fn load_from_str(content: &str) -> std::io::Result<ConfigMap> {
    let config: ConfigMap = serde_json::from_str(content)?;
    Ok(config)
}

use crate::config::{self, ShellConfigMap};
use std::path::{Path, PathBuf};

pub struct ResolvedShell {
    pub path: PathBuf,
    pub alias: String,
    pub display_name: String,
    pub args: Vec<String>,
}

/// Load the shell config from `~/.config/noo/sh-languages.json`.
pub fn load_config() -> ShellConfigMap {
    let embedded = include_str!("../config/sh-languages.json");
    config::load_shell_config(embedded)
}

/// Resolve a shell alias to its executable path.
///
/// Searches PATH for each executable name listed in the config entry.
pub fn resolve_shell(name: &str) -> Option<ResolvedShell> {
    let cfg = load_config();
    let entry = cfg.get(name)?;

    for exe in &entry.executables {
        if let Some(path) = search_path(exe) {
            return Some(ResolvedShell {
                path,
                alias: name.to_string(),
                display_name: entry
                    .description
                    .clone()
                    .unwrap_or_else(|| name.to_string()),
                args: entry.args.clone(),
            });
        }
        #[cfg(windows)]
        {
            if let Some(path) = search_common_windows_paths(exe) {
                return Some(ResolvedShell {
                    path,
                    alias: name.to_string(),
                    display_name: entry
                        .description
                        .clone()
                        .unwrap_or_else(|| name.to_string()),
                    args: entry.args.clone(),
                });
            }
        }
    }

    None
}

pub fn list_supported_shells() -> Vec<String> {
    let cfg = load_config();
    let mut shells: Vec<String> = cfg.into_keys().collect();
    shells.sort();
    shells
}

fn search_path(exe: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let with_exe = candidate.with_extension("exe");
            if with_exe.is_file() {
                return Some(with_exe);
            }
        }
    }
    None
}

#[cfg(windows)]
fn search_common_windows_paths(exe: &str) -> Option<PathBuf> {
    let program_files = [
        r"C:\Program Files\Git\bin",
        r"C:\Program Files\Git\usr\bin",
        r"C:\Program Files (x86)\Git\bin",
        r"C:\Program Files (x86)\Git\usr\bin",
        r"C:\Windows\System32\WindowsPowerShell\v1.0",
        r"C:\Windows\System32",
        r"C:\Windows\SysWOW64\WindowsPowerShell\v1.0",
        r"C:\Windows\SysWOW64",
    ];

    let exe = if !exe.to_lowercase().ends_with(".exe") {
        format!("{}.exe", exe)
    } else {
        exe.to_string()
    };

    for dir in &program_files {
        let candidate = Path::new(dir).join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // Check WSL paths
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let wsl_paths = [
        format!(r"{}\Microsoft\WindowsApps", local_app_data),
    ];
    for dir in &wsl_paths {
        let candidate = Path::new(dir).join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct Noorc {
    pub language: Option<String>,
    pub aliases: HashMap<String, String>,
    pub startup: Vec<String>,
}

fn noorc_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("nooshell").join("noorc")
}

impl Noorc {
    pub fn load() -> Self {
        let path = noorc_path();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let mut noorc = Noorc::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("language ") {
                let lang = rest.trim().trim_matches('"').trim_matches('\'');
                noorc.language = Some(lang.to_string());
            } else if let Some(rest) = line.strip_prefix("alias ") {
                if let Some((name, cmd)) = rest.split_once('=') {
                    let name = name.trim();
                    let cmd = cmd.trim().trim_matches('"').trim_matches('\'');
                    if !name.is_empty() && !cmd.is_empty() {
                        noorc.aliases.insert(name.to_string(), cmd.to_string());
                    }
                }
            } else {
                noorc.startup.push(line.to_string());
            }
        }

        noorc
    }
}

impl Default for Noorc {
    fn default() -> Self {
        Self {
            language: None,
            aliases: HashMap::new(),
            startup: Vec::new(),
        }
    }
}

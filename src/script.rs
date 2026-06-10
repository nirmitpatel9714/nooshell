use crate::config::ConfigMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct NsScript {
    pub lines: Vec<(Option<String>, String)>, // (optional alias, code)
}

// ── Variable analysis helpers ──

const KEYWORDS: &[&str] = &[
    "print", "console", "log", "typeof", "undefined", "true", "false", "null",
    "if", "else", "for", "while", "return", "import", "from", "def", "class",
    "let", "var", "const", "function", "new", "this", "in", "of", "not", "and", "or",
    "try", "catch", "finally", "throw", "async", "await", "yield", "global",
    "require", "module", "exports", "__dirname", "__filename", "process",
    "Object", "Array", "String", "Number", "Boolean", "JSON", "Math", "Date",
    "console", "dir", "globals", "builtins",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

fn is_number(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn sanitize_var_name(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if cleaned.len() > 1
        && (cleaned.starts_with(|c: char| c.is_alphabetic() || c == '_'))
        && !is_keyword(&cleaned)
    {
        Some(cleaned)
    } else {
        None
    }
}

fn extract_assignments(code: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let bytes = code.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let ch = bytes[i] as char;
        if in_string {
            if ch == string_char && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = true;
            string_char = ch;
            i += 1;
            continue;
        }
        if ch == '=' && i > 0 && bytes[i - 1] as char != '=' && i + 1 < len && bytes[i + 1] as char != '=' {
            // Find the word before this =
            let mut j = i.saturating_sub(1);
            while j > 0 && (bytes[j - 1] as char).is_whitespace() {
                j -= 1;
            }
            let start = j;
            while j > 0
                && ((bytes[j - 1] as char).is_alphanumeric() || bytes[j - 1] as char == '_')
            {
                j -= 1;
            }
            let candidate = &code[j..start];
            if let Some(cleaned) = sanitize_var_name(candidate) {
                vars.push(cleaned);
            }
        }
        i += 1;
    }
    vars
}

fn extract_words(code: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    for ch in code.chars() {
        if in_string {
            if ch == string_char {
                in_string = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = true;
            string_char = ch;
            if !current.is_empty() {
                if !is_keyword(&current) && !is_number(&current) {
                    words.push(current.clone());
                }
                current.clear();
            }
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() && !is_keyword(&current) && !is_number(&current) {
                words.push(current.clone());
            }
            current.clear();
        }
    }
    if !current.is_empty() && !is_keyword(&current) && !is_number(&current) {
        words.push(current);
    }
    words
}

/// Compute cleanup schedule: for each variable, the last line index where it appears.
/// Returns `(line_index_after_which_to_clean, "del varname")` sorted by line index.
pub fn compute_cleanup_schedule(lines: &[(Option<String>, String)]) -> Vec<(usize, String)> {
    let mut var_first: HashMap<String, usize> = HashMap::new();
    let mut var_last: HashMap<String, usize> = HashMap::new();

    for (i, (_, code)) in lines.iter().enumerate() {
        for var in extract_assignments(code) {
            var_first.entry(var.clone()).or_insert(i);
            var_last.insert(var.clone(), i);
        }
        for var in extract_words(code) {
            var_last.insert(var, i);
        }
    }

    let mut cleanup: Vec<(usize, String)> = Vec::new();
    for (var, &last) in &var_last {
        if var_first.contains_key(var) {
            cleanup.push((last, format!("del {}", var)));
        }
    }

    cleanup.sort_by_key(|(i, _)| *i);
    // Combine dels for the same line: "del x, y, z"
    let mut deduped: Vec<(usize, Vec<String>)> = Vec::new();
    for (line, cmd) in cleanup {
        let var = cmd[4..].to_string();
        if let Some(last) = deduped.last_mut() {
            if last.0 == line {
                last.1.push(var);
                continue;
            }
        }
        deduped.push((line, vec![var]));
    }
    deduped
        .into_iter()
        .map(|(line, vars)| (line, format!("del {}", vars.join(", "))))
        .collect()
}

impl NsScript {
    pub fn load<P: AsRef<Path>>(path: P, config_map: &ConfigMap) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_string(&content, config_map)
    }

    pub fn from_string(content: &str, config_map: &ConfigMap) -> std::io::Result<Self> {
        let mut lines = Vec::new();
        let mut last_lang: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((first, rest)) = line.split_once(' ') {
                let first = first.trim();
                if config_map.contains_key(first) {
                    last_lang = Some(first.to_string());
                    lines.push((last_lang.clone(), rest.trim().to_string()));
                    continue;
                }
            }
            // Manual del directive (no language prefix)
            if line.starts_with("del ") {
                lines.push((None, line.to_string()));
                continue;
            }
            // Bare line — inherit language from previous line
            if let Some(ref lang) = last_lang {
                lines.push((Some(lang.clone()), line.to_string()));
            }
        }
        Ok(Self { lines })
    }

    pub async fn run_embedded(content: &str, languages_json: &str) {
        Self::run_embedded_with_cleanup(content, languages_json, &[]).await;
    }

    pub async fn run_embedded_with_cleanup(
        content: &str,
        languages_json: &str,
        cleanup: &[(usize, String)],
    ) {
        let config = std::sync::Arc::new(
            crate::config::load_from_str(languages_json).unwrap_or_default(),
        );
        let script = match NsScript::from_string(content, config.as_ref()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to parse script: {}", e);
                return;
            }
        };

        // Build a lookup: line_index -> list of cleanup commands
        let mut cleanup_map: HashMap<usize, Vec<String>> = HashMap::new();
        for (line_idx, cmd) in cleanup {
            cleanup_map
                .entry(*line_idx)
                .or_default()
                .push(cmd.clone());
        }

        let state = crate::state::SharedState::new();
        let mut handles = Vec::new();

        for (i, (alias, code)) in script.lines.iter().enumerate() {
            // Handle manual del directive
            if alias.is_none() && code.starts_with("del ") {
                let var_name = code[4..].trim().to_string();
                state.remove(&var_name);
                continue;
            }

            let lang = alias.as_deref().unwrap_or("py").to_string();
            let code = code.clone();
            let config = config.clone();
            let state = state.clone();

            // Collect cleanup actions that fire after this line
            let after_cleanup: Vec<String> = cleanup_map.remove(&i).unwrap_or_default();

            handles.push(tokio::spawn(async move {
                if let Some(cfg) = config.get(&lang) {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
                    if let Ok(session) = crate::execution::ProcessSession::start(cfg, tx) {
                        if let Some(inj) = crate::bridge::injection_code(&state, &lang) {
                            session.send_input(&inj).await;
                        }
                        session.send_input(&code).await;

                        // Dump state back to SharedState after user code
                        if let Some(dump) = crate::bridge::dump_code(&lang) {
                            session.send_input(&dump).await;
                        }

                        // Inject cleanup after user code
                        for cmd in &after_cleanup {
                            let lang = &*lang;
                            match lang {
                                "py" => {
                                    session.send_input(cmd).await;
                                }
                                "js" => {
                                    // Convert "del x" to "delete global.x"
                                    let vars: Vec<&str> =
                                        cmd.trim_start_matches("del ").split(", ").collect();
                                    let js_del: String = vars
                                        .iter()
                                        .map(|v| format!("delete global.{}", v.trim()))
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    session.send_input(&js_del).await;
                                }
                                _ => {
                                    session.send_input(cmd).await;
                                }
                            }
                        }

                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        while let Ok(line) = rx.try_recv() {
                            let trimmed = line.trim_start_matches('>').trim_start();
                            if let Some(rest) =
                                trimmed.strip_prefix(crate::bridge::STATE_PREFIX)
                            {
                                state.import_json(rest);
                            } else if !line.is_empty() {
                                println!("{}", line);
                            }
                        }
                    }
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }
    }
}

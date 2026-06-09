use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("nooshell");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn history_path() -> PathBuf {
    data_dir().join("history.json")
}

fn sessions_path() -> PathBuf {
    data_dir().join("sessions.json")
}

fn timestamp() -> String {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}", start)
}

// ── Command history ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandRecord {
    pub id: u64,
    pub session_key: String,
    pub language: String,
    pub command: String,
    pub timestamp: String,
    pub output_preview: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HistoryStore {
    pub commands: Vec<CommandRecord>,
}

pub fn load_history() -> HistoryStore {
    fs::read_to_string(history_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_history(store: &HistoryStore) {
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(history_path(), json);
    }
}

pub fn push_command(language: &str, command: &str, output: &[String]) {
    let mut store = load_history();
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let preview = output.first().cloned().unwrap_or_default();
    store.commands.push(CommandRecord {
        id,
        session_key: String::new(),
        language: language.to_string(),
        command: command.to_string(),
        timestamp: timestamp(),
        output_preview: preview,
    });
    save_history(&store);
}

pub fn history_path_str() -> String {
    history_path().to_string_lossy().to_string()
}

pub fn clear_history() {
    save_history(&HistoryStore::default());
}

// ── Session persistence ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedCell {
    pub active_language: String,
    pub history: Vec<String>,
    pub execution_count: usize,
    pub output_lines: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedWorkspace {
    pub name: String,
    pub active_pane: usize,
    pub cells: Vec<SavedCell>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub workspaces: Vec<SavedWorkspace>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SessionStore {
    pub sessions: Vec<SessionRecord>,
}

pub fn load_sessions() -> SessionStore {
    fs::read_to_string(sessions_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_sessions(store: &SessionStore) {
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(sessions_path(), json);
    }
}

pub fn push_session(record: SessionRecord) {
    let mut store = load_sessions();
    store.sessions.push(record);
    save_sessions(&store);
}

pub fn update_session(id: &str, record: SessionRecord) {
    let mut store = load_sessions();
    if let Some(pos) = store.sessions.iter().position(|s| s.id == id) {
        store.sessions[pos] = record;
    } else {
        store.sessions.push(record);
    }
    save_sessions(&store);
}

pub fn delete_session(id: &str) -> bool {
    let mut store = load_sessions();
    let len_before = store.sessions.len();
    store.sessions.retain(|s| s.id != id);
    let removed = store.sessions.len() < len_before;
    save_sessions(&store);
    removed
}

pub fn list_sessions() -> Vec<SessionRecord> {
    load_sessions().sessions
}

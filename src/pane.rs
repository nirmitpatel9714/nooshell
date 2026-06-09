use crate::bridge;
use crate::config::ConfigMap;
use crate::execution::ProcessSession;
use crate::state::SharedState;
use std::collections::HashMap;
use std::env;
use tokio::sync::mpsc;

pub struct Pane {
    pub id: usize,
    pub active_language: String,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub output_lines: Vec<String>,
    pub session: Option<ProcessSession>,
    pub output_receiver: mpsc::Receiver<String>,
    output_sender: mpsc::Sender<String>,
    pub history: Vec<String>,
    pub history_index: usize,
    pub execution_count: usize,
    pub aliases: HashMap<String, String>,
    pub state: SharedState,
}

impl Pane {
    pub fn new(id: usize, default_language: String, state: SharedState) -> Self {
        let (output_sender, output_receiver) = mpsc::channel(100);
        Self {
            id,
            active_language: default_language,
            input_buffer: String::new(),
            cursor_pos: 0,
            output_lines: Vec::new(),
            session: None,
            output_receiver,
            output_sender,
            history: Vec::new(),
            history_index: 0,
            execution_count: 0,
            aliases: HashMap::new(),
            state,
        }
    }

    pub fn start_session(&mut self, config_map: &ConfigMap) -> Result<(), String> {
        if let Some(config) = config_map.get(&self.active_language) {
            match ProcessSession::start(config, self.output_sender.clone()) {
                Ok(session) => {
                    self.session = Some(session);

                    Ok(())
                }
                Err(e) => {
                    let err_msg = format!("Failed to start process: {}", e);
                    self.output_lines.push(err_msg.clone());
                    Err(err_msg)
                }
            }
        } else {
            let err_msg = format!("Language {} not found in config.", self.active_language);
            self.output_lines.push(err_msg.clone());
            Err(err_msg)
        }
    }

    pub async fn handle_input(&mut self) -> Option<String> {
        if self.input_buffer.trim().is_empty() {
            return None;
        }
        let input = self.input_buffer.clone();
        
        if self.history.is_empty() || self.history.last().unwrap() != &input {
            self.history.push(input.clone());
        }
        self.history_index = self.history.len();

        self.execution_count += 1;
        self.input_buffer.clear();
        self.cursor_pos = 0;

        let expanded = self.aliases.get(input.trim()).cloned();
        let input = expanded.as_deref().unwrap_or(&input).to_string();

        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0] {
            "clear" => {
                self.output_lines.clear();
                return None;
            }
            "cd" => {
                if parts.len() > 1 {
                    if let Err(e) = env::set_current_dir(parts[1]) {
                        self.output_lines.push(format!("cd error: {}", e));
                    }
                }
                return None;
            }
            "ls" | "la" => {
                if let Ok(entries) = std::fs::read_dir(".") {
                    let mut files = Vec::new();
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let md = entry.metadata().ok();
                        let is_dir = md.map(|m| m.is_dir()).unwrap_or(false);
                        if is_dir {
                            files.push(format!("[DIR] {}", name));
                        } else {
                            files.push(name);
                        }
                    }
                    self.output_lines.push(files.join("  "));
                }
                return None;
            }
            "noo" => {
                if parts.len() > 1 && parts[1] == "nbmode" {
                    return Some("nbmode".to_string());
                }
                return None;
            }
            "exit" => {
                return Some("exit".to_string());
            }
            _ => {}
        }

        if let Some(session) = &mut self.session {
            // Inject shared state into the REPL before user code
            if let Some(code) = bridge::injection_code(&self.state, &self.active_language) {
                session.send_input(&code).await;
            }
            // Send the user's code
            session.send_input(&input).await;
            // Dump state back from the REPL after user code
            if let Some(code) = bridge::dump_code(&self.active_language) {
                session.send_input(&code).await;
            }
        } else {
            self.output_lines.push("No active session.".to_string());
        }

        None
    }

    pub fn poll_output(&mut self) {
        while let Ok(line) = self.output_receiver.try_recv() {
            let trimmed = line.trim_start_matches('>').trim_start();
            if let Some(rest) = trimmed.strip_prefix(bridge::STATE_PREFIX) {
                self.state.import_json(rest);
            } else {
                self.output_lines.push(line);
            }
        }
    }
}

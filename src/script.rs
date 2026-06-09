use crate::config::ConfigMap;
use crate::execution::ProcessSession;
use crate::state::SharedState;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

pub struct NsScript {
    pub lines: Vec<(String, String)>, // (alias, code)
}

impl NsScript {
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut lines = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((alias, code)) = line.split_once(' ') {
                lines.push((alias.trim().to_string(), code.trim().to_string()));
            }
        }
        Ok(Self { lines })
    }

    pub async fn execute(
        &self,
        config_map: &ConfigMap,
        state: &SharedState,
        output_sender: mpsc::Sender<String>,
    ) {
        for (alias, code) in &self.lines {
            if let Some(config) = config_map.get(alias) {
                let (dummy_out, _dummy_rx) = mpsc::channel(100);
                if let Ok(session) = ProcessSession::start(config, dummy_out) {
                    // Inject state
                    let state_json = state.as_json_string();
                    let state_injection = match alias.as_str() {
                        "py" => format!("import json; __state = json.loads('{}')", state_json),
                        "js" => format!("const __state = JSON.parse('{}');", state_json),
                        _ => String::new(),
                    };

                    if !state_injection.is_empty() {
                        session.send_input(&state_injection).await;
                    }

                    // Run the actual code
                    session.send_input(code).await;
                    let _ = output_sender.send(format!("Executed [{}] {}", alias, code)).await;
                    // In a full implementation, we'd wait for this to finish and capture state updates
                } else {
                    let _ = output_sender.send(format!("Failed to start process for {}", alias)).await;
                }
            } else {
                let _ = output_sender.send(format!("Unknown alias: {}", alias)).await;
            }
        }
    }
}

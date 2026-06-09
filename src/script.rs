use crate::bridge;
use crate::config::ConfigMap;
use crate::execution::ProcessSession;
use crate::state::SharedState;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

pub struct NsScript {
    pub lines: Vec<(Option<String>, String)>, // (optional alias, code)
}

impl NsScript {
    pub fn load<P: AsRef<Path>>(path: P, config_map: &ConfigMap) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
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
            // Bare line — inherit language from previous line
            if let Some(ref lang) = last_lang {
                lines.push((Some(lang.clone()), line.to_string()));
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
            let alias = match alias {
                Some(a) => a.clone(),
                None => continue,
            };
            if let Some(config) = config_map.get(&alias) {
                let (dummy_out, _dummy_rx) = mpsc::channel(100);
                if let Ok(session) = ProcessSession::start(config, dummy_out) {
                    if let Some(inj) = bridge::injection_code(state, &alias) {
                        session.send_input(&inj).await;
                    }

                    session.send_input(code).await;
                    let _ = output_sender.send(format!("Executed [{}] {}", alias, code)).await;
                } else {
                    let _ = output_sender.send(format!("Failed to start process for {}", alias)).await;
                }
            } else {
                let _ = output_sender.send(format!("Unknown alias: {}", alias)).await;
            }
        }
    }
}

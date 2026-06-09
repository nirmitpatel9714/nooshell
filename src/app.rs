use crate::config::ConfigMap;
use crate::pane::Pane;
use crate::state::SharedState;
use crate::store;
use std::collections::HashMap;

pub struct Workspace {
    pub name: String,
    pub panes: Vec<Pane>,
    pub active_pane: usize,
}

impl Workspace {
    pub fn new(name: String, config: &ConfigMap, state: SharedState) -> Self {
        Self::with_language(name, config, "py", state)
    }

    pub fn with_language(name: String, config: &ConfigMap, language: &str, state: SharedState) -> Self {
        let mut pane = Pane::new(0, language.to_string(), state);
        let _ = pane.start_session(config);
        Self {
            name,
            panes: vec![pane],
            active_pane: 0,
        }
    }

    pub fn current_pane_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.active_pane]
    }

    pub fn add_cell(&mut self, config: &ConfigMap, state: SharedState) {
        let id = self.panes.len();
        let mut pane = Pane::new(id, "py".to_string(), state);
        let _ = pane.start_session(config);
        let insert_pos = self.active_pane + 1;
        self.panes.insert(insert_pos, pane);
        self.active_pane = insert_pos;
    }

    pub fn remove_cell(&mut self) {
        if self.panes.len() <= 1 {
            return;
        }
        self.panes.remove(self.active_pane);
        if self.active_pane >= self.panes.len() {
            self.active_pane = self.panes.len() - 1;
        }
    }

    pub fn move_cell_up(&mut self) {
        if self.active_pane > 0 {
            self.panes.swap(self.active_pane, self.active_pane - 1);
            self.active_pane -= 1;
        }
    }

    pub fn move_cell_down(&mut self) {
        if self.active_pane + 1 < self.panes.len() {
            self.panes.swap(self.active_pane, self.active_pane + 1);
            self.active_pane += 1;
        }
    }

    pub fn poll(&mut self) {
        for pane in &mut self.panes {
            pane.poll_output();
        }
    }

    pub fn ensure_pane(&mut self, language: &str, config: &ConfigMap, state: SharedState) -> usize {
        if let Some(pos) = self.panes.iter().position(|p| p.active_language == language) {
            return pos;
        }
        let id = self.panes.len();
        let mut pane = Pane::new(id, language.to_string(), state);
        let _ = pane.start_session(config);
        self.panes.push(pane);
        self.panes.len() - 1
    }
}

pub struct App {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
    pub config: ConfigMap,
    pub running: bool,
    pub state: SharedState,
}

impl App {
    pub fn new(config: ConfigMap) -> Self {
        let state = SharedState::new();
        let workspace = Workspace::new("Workspace 1".to_string(), &config, state.clone());
        Self {
            workspaces: vec![workspace],
            active_workspace: 0,
            config,
            running: true,
            state,
        }
    }

    pub fn with_noorc(config: ConfigMap, language: Option<&str>, aliases: HashMap<String, String>) -> Self {
        let lang = language.unwrap_or("py");
        let state = SharedState::new();
        let mut workspace = Workspace::with_language("Workspace 1".to_string(), &config, lang, state.clone());
        workspace.panes[0].aliases = aliases;
        Self {
            workspaces: vec![workspace],
            active_workspace: 0,
            config,
            running: true,
            state,
        }
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    pub fn current_pane_mut(&mut self) -> &mut Pane {
        self.current_workspace_mut().current_pane_mut()
    }

    pub fn add_cell(&mut self) {
        let config = self.config.clone();
        let state = self.state.clone();
        self.current_workspace_mut().add_cell(&config, state);
    }

    pub fn remove_cell(&mut self) {
        self.current_workspace_mut().remove_cell();
    }

    pub fn move_cell_up(&mut self) {
        self.current_workspace_mut().move_cell_up();
    }

    pub fn move_cell_down(&mut self) {
        self.current_workspace_mut().move_cell_down();
    }

    pub fn add_workspace(&mut self) {
        let name = format!("Workspace {}", self.workspaces.len() + 1);
        let config = self.config.clone();
        let state = self.state.clone();
        let ws = Workspace::new(name, &config, state);
        self.workspaces.push(ws);
        self.active_workspace = self.workspaces.len() - 1;
    }

    pub fn remove_workspace(&mut self) {
        if self.workspaces.len() <= 1 {
            return;
        }
        self.workspaces.remove(self.active_workspace);
        if self.active_workspace >= self.workspaces.len() {
            self.active_workspace = self.workspaces.len() - 1;
        }
    }

    pub fn next_workspace(&mut self) {
        if self.active_workspace + 1 < self.workspaces.len() {
            self.active_workspace += 1;
        }
    }

    pub fn previous_workspace(&mut self) {
        if self.active_workspace > 0 {
            self.active_workspace -= 1;
        }
    }

    pub fn poll_all_panes(&mut self) {
        for ws in &mut self.workspaces {
            ws.poll();
        }
    }

    pub fn record_command(&self, language: &str, command: &str, output_lines: &[String]) {
        store::push_command(language, command, output_lines);
    }

    pub fn save_session(&self, key: &str) {
        let workspaces: Vec<store::SavedWorkspace> = self.workspaces.iter().map(|ws| {
            let cells = ws.panes.iter().map(|p| store::SavedCell {
                active_language: p.active_language.clone(),
                history: p.history.clone(),
                execution_count: p.execution_count,
                output_lines: p.output_lines.clone(),
            }).collect();
            store::SavedWorkspace {
                name: ws.name.clone(),
                active_pane: ws.active_pane,
                cells,
            }
        }).collect();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();

        let record = store::SessionRecord {
            id: key.to_string(),
            name: key.to_string(),
            created_at: now.clone(),
            updated_at: now,
            workspaces,
        };
        store::update_session(key, record);
    }

    pub fn load_workspaces_from_session(key: &str) -> Option<Vec<store::SavedWorkspace>> {
        let sessions = store::list_sessions();
        sessions.into_iter().find(|s| s.id == key).map(|s| s.workspaces)
    }
}

use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SharedState {
    pub store: Arc<Mutex<Map<String, Value>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Map::new())),
        }
    }

    pub fn set(&self, key: &str, value: Value) {
        let mut store = self.store.lock().unwrap();
        store.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        let store = self.store.lock().unwrap();
        store.get(key).cloned()
    }

    pub fn as_json_string(&self) -> String {
        let store = self.store.lock().unwrap();
        serde_json::to_string(&*store).unwrap_or_else(|_| "{}".to_string())
    }
}

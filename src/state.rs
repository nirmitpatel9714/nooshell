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

    pub fn import_json(&self, json_str: &str) {
        if let Ok(parsed) = serde_json::from_str::<Map<String, Value>>(json_str) {
            let mut store = self.store.lock().unwrap();
            for (k, v) in parsed {
                store.insert(k, v);
            }
        }
    }

    pub fn remove(&self, key: &str) {
        let mut store = self.store.lock().unwrap();
        store.remove(key);
    }

    pub fn is_empty(&self) -> bool {
        let store = self.store.lock().unwrap();
        store.is_empty()
    }
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiRequest {
    pub name: String,
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub group: Option<String>,
}

pub struct CollectionManager {
    pub requests: Vec<ApiRequest>,
    pub history: Vec<ApiRequest>,
    pub base_path: String,
}

impl CollectionManager {
    pub fn new() -> Self {
        let base_path = ".clicaude".to_string();
        let coll_path = format!("{}/collections", base_path);
        if !Path::new(&coll_path).exists() { let _ = fs::create_dir_all(&coll_path); }
        let mut manager = Self { requests: Vec::new(), history: Vec::new(), base_path };
        let _ = manager.load_all();
        let _ = manager.load_history();
        manager
    }

    pub fn load_all(&mut self) -> Result<()> {
        self.requests.clear();
        let coll_path = format!("{}/collections", self.base_path);
        if let Ok(entries) = fs::read_dir(coll_path) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = fs::read_to_string(entry.path())?;
                    if let Ok(req) = serde_json::from_str::<ApiRequest>(&content) { self.requests.push(req); }
                }
            }
        }
        Ok(())
    }

    pub fn save_request(&self, req: &ApiRequest) -> Result<()> {
        let path = format!("{}/collections/{}.json", self.base_path, req.name.replace(" ", "_"));
        let content = serde_json::to_string_pretty(req)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn delete_request(&mut self, idx: usize) -> Result<()> {
        if let Some(req) = self.requests.get(idx) {
            let path = format!("{}/collections/{}.json", self.base_path, req.name.replace(" ", "_"));
            let _ = fs::remove_file(path);
            self.requests.remove(idx);
        }
        Ok(())
    }

    pub fn add_to_history(&mut self, req: ApiRequest) {
        self.history.insert(0, req);
        if self.history.len() > 50 { self.history.pop(); }
        let _ = self.save_history();
    }

    pub fn delete_history_item(&mut self, idx: usize) {
        if idx < self.history.len() {
            self.history.remove(idx);
            let _ = self.save_history();
        }
    }

    fn save_history(&self) -> Result<()> {
        let path = format!("{}/history.json", self.base_path);
        let content = serde_json::to_string_pretty(&self.history)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn load_history(&mut self) -> Result<()> {
        let path = format!("{}/history.json", self.base_path);
        if Path::new(&path).exists() {
            let content = fs::read_to_string(path)?;
            self.history = serde_json::from_str(&content)?;
        }
        Ok(())
    }
}

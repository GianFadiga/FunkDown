use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::model::{MediaKind, Platform};
use crate::settings::write_atomic;

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: u64,
    pub title: String,
    pub uploader: Option<String>,
    pub platform: Platform,
    pub kind: MediaKind,
    pub url: String,
    pub file: String,
    pub file_count: u32,
    pub finished_at: u64,
    /// Calculado na leitura: o arquivo ainda está no disco?
    #[serde(default, skip_deserializing)]
    pub exists: bool,
}

/// Histórico local, em um único JSON. Nada sai da máquina.
pub struct History {
    path: PathBuf,
    entries: Mutex<Vec<HistoryEntry>>,
}

impl History {
    pub fn load(path: PathBuf) -> Self {
        let entries = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            entries: Mutex::new(entries),
        }
    }

    fn persist(&self, entries: &[HistoryEntry]) {
        if let Ok(bytes) = serde_json::to_vec(entries) {
            let _ = write_atomic(&self.path, &bytes);
        }
    }

    /// Mais recentes primeiro.
    pub fn list(&self) -> Vec<HistoryEntry> {
        let mut out = self.entries.lock().unwrap().clone();
        for e in &mut out {
            e.exists = std::path::Path::new(&e.file).exists();
        }
        out.reverse();
        out
    }

    pub fn get(&self, id: u64) -> Option<HistoryEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    pub fn add(&self, mut entry: HistoryEntry) {
        let mut entries = self.entries.lock().unwrap();
        entry.id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        entries.push(entry);
        if entries.len() > MAX_ENTRIES {
            let extra = entries.len() - MAX_ENTRIES;
            entries.drain(..extra);
        }
        self.persist(&entries);
    }

    pub fn remove(&self, id: u64) {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|e| e.id != id);
        self.persist(&entries);
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str) -> HistoryEntry {
        HistoryEntry {
            id: 0,
            title: title.into(),
            uploader: None,
            platform: Platform::Youtube,
            kind: MediaKind::Audio,
            url: "https://youtu.be/a".into(),
            file: "/nonexistent/a.mp3".into(),
            file_count: 1,
            finished_at: 1,
            exists: false,
        }
    }

    #[test]
    fn persists_and_clears() {
        let dir = std::env::temp_dir().join(format!("fd-hist-{}", std::process::id()));
        let path = dir.join("history.json");
        let h = History::load(path.clone());
        h.add(entry("a"));
        h.add(entry("b"));
        let again = History::load(path.clone());
        let list = again.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "b");
        assert_ne!(list[0].id, list[1].id);
        again.remove(list[0].id);
        assert_eq!(History::load(path.clone()).list().len(), 1);
        again.clear();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}

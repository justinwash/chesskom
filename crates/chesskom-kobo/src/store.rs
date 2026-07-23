//! File-backed history storage for the device.

use chesskom_ui::{History, HistoryStore};
use std::path::PathBuf;

/// Persists history to a small text file. Best-effort: read/write errors degrade
/// to an empty history rather than crashing the game.
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    /// Path comes from `CHESSKOM_HISTORY`, else a file in the current directory.
    pub fn from_env() -> FileStore {
        let path = std::env::var("CHESSKOM_HISTORY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("chesskom-history.txt"));
        FileStore { path }
    }
}

impl HistoryStore for FileStore {
    fn load(&self) -> History {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => History::parse(&text),
            Err(_) => History::new(),
        }
    }

    fn save(&self, history: &History) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.path, history.serialize());
    }
}

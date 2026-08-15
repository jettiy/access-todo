//! Local crash-safe persistence for the TODO document.
//!
//! The API server keeps its working state in memory; this module mirrors
//! every change to a local file so a crash or a bad Gist sync can never
//! lose data. Writes are atomic (write to `.tmp`, then rename over the
//! target), and `backup` keeps timestamped snapshots before pushes.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};
use crate::model::TodoDoc;

/// How many timestamped backups to keep in the backup directory.
const KEEP_BACKUPS: usize = 10;

#[derive(Debug, Clone)]
pub struct LocalStore {
    path: PathBuf,
    backup_dir: PathBuf,
}

impl LocalStore {
    /// Create a store backed by `path` (e.g. `.../bin/access-data.json`),
    /// with backups under `path.parent()/backups`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let backup_dir = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        Self { path, backup_dir }
    }

    /// Load the persisted document. `Ok(None)` when no file exists yet.
    pub fn load(&self) -> Result<Option<TodoDoc>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.path)?;
        if raw.trim().is_empty() {
            return Ok(None);
        }
        let doc: TodoDoc = serde_json::from_str(&raw)?;
        Ok(Some(doc))
    }

    /// Atomically persist `doc`. The previous file is only replaced after
    /// the new content is fully written to the temp file and flushed.
    pub fn save(&self, doc: &TodoDoc) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(serde_json::to_string_pretty(doc)?.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Write a timestamped snapshot of `doc` and prune old backups.
    /// Returns the backup file path.
    pub fn backup(&self, doc: &TodoDoc) -> Result<PathBuf> {
        fs::create_dir_all(&self.backup_dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let target = self.backup_dir.join(format!("access-{stamp}.json"));
        let tmp = target.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(serde_json::to_string_pretty(doc)?.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &target)?;
        self.prune_backups()?;
        Ok(target)
    }

    /// Keep only the newest `KEEP_BACKUPS` snapshots.
    fn prune_backups(&self) -> Result<()> {
        let mut backups: Vec<PathBuf> = fs::read_dir(&self.backup_dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("access-") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();
        backups.sort(); // timestamped names sort chronologically
        while backups.len() > KEEP_BACKUPS {
            let oldest = backups.remove(0);
            let _ = fs::remove_file(oldest);
        }
        Ok(())
    }

    /// The file this store persists to.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TodoDoc;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "access-storage-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tmpdir();
        let store = LocalStore::new(dir.join("data.json"));

        assert!(store.load().unwrap().is_none());

        let mut doc = TodoDoc::new("test");
        doc.categories.push(crate::model::Category {
            id: "c1".into(),
            agent: "zcode".into(),
            name: "버그 수정".into(),
            order: 0,
            updated_at: None,
        });
        store.save(&doc).unwrap();

        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.categories.len(), 1);
        assert_eq!(loaded.categories[0].name, "버그 수정");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_creates_and_prunes() {
        let dir = tmpdir();
        let store = LocalStore::new(dir.join("data.json"));
        let doc = TodoDoc::new("test");
        for _ in 0..(KEEP_BACKUPS + 5) {
            store.backup(&doc).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let count = std::fs::read_dir(dir.join("backups")).unwrap().count();
        assert!(count <= KEEP_BACKUPS + 1); // allow race margin
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Pilot settings persisted as `access-settings.json` next to the API
//! executable: which agents are enabled, the pilot's knowledge level
//! (summary style), and the sync mode. The settings window (P1) and the
//! MCP server (pilot-profile injection) both read this file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Valid pilot knowledge levels. `auto` lets each agent's LLM infer the
/// level from conversation history (제품기획 v1.2 · 지식상태 4단계).
pub const LEVELS: &[&str] = &["beginner", "intermediate", "advanced", "auto"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Agent ids the pilot toggled ON (registry ids + custom REST ids).
    #[serde(default)]
    pub enabled_agents: Vec<String>,
    /// beginner | intermediate | advanced | auto
    #[serde(default = "default_level")]
    pub pilot_level: String,
    /// gist | local
    #[serde(default = "default_sync")]
    pub sync_mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_agents: vec![],
            pilot_level: default_level(),
            sync_mode: default_sync(),
        }
    }
}

fn default_level() -> String {
    "auto".into()
}

fn default_sync() -> String {
    "gist".into()
}

impl Settings {
    /// Clamp unknown values back to defaults so a hand-edited file can never
    /// put the system in an undefined state.
    pub fn normalize(&mut self) {
        if !LEVELS.contains(&self.pilot_level.as_str()) {
            self.pilot_level = default_level();
        }
        if self.sync_mode != "local" {
            self.sync_mode = default_sync();
        }
        self.enabled_agents.retain(|s| !s.is_empty());
        self.enabled_agents.dedup();
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .map(|mut s: Settings| {
                s.normalize();
                s
            })
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn enable(&mut self, id: &str) {
        if !self.enabled_agents.iter().any(|a| a == id) {
            self.enabled_agents.push(id.to_string());
        }
    }

    pub fn disable(&mut self, id: &str) {
        self.enabled_agents.retain(|a| a != id);
    }
}

/// Default settings file lives next to the API executable, like the data file.
pub fn default_settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("access-settings.json")))
        .unwrap_or_else(|| PathBuf::from("access-settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_gives_defaults() {
        let s = Settings::load(Path::new("Z:/definitely/missing.json"));
        assert_eq!(s.pilot_level, "auto");
        assert_eq!(s.sync_mode, "gist");
        assert!(s.enabled_agents.is_empty());
    }

    #[test]
    fn normalize_clamps_garbage_and_dedups() {
        let mut s = Settings {
            enabled_agents: vec!["zcode".into(), "zcode".into(), "".into()],
            pilot_level: "wizard".into(),
            sync_mode: "local".into(),
        };
        s.normalize();
        assert_eq!(s.pilot_level, "auto");
        assert_eq!(s.sync_mode, "local");
        assert_eq!(s.enabled_agents, vec!["zcode".to_string()]);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let path = std::env::temp_dir().join(format!(
            "access-settings-test-{}.json",
            std::process::id()
        ));
        let mut s = Settings::default();
        s.enable("zcode");
        s.enable("hermes");
        s.pilot_level = "beginner".into();
        s.save(&path).unwrap();
        let loaded = Settings::load(&path);
        assert_eq!(loaded.pilot_level, "beginner");
        assert_eq!(loaded.enabled_agents, vec!["zcode", "hermes"]);
        let _ = std::fs::remove_file(&path);
    }
}

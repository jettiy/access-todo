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
    /// game | project — 어떤 종류의 작업 공간인지 (노트 창 제목·자동 태깅 기준)
    #[serde(default = "default_mode")]
    pub workspace_mode: String,
    /// 게임 모드에서 현재 선택한 게임 이름
    #[serde(default)]
    pub active_game: Option<String>,
    /// 프로젝트 모드에서 현재 선택한 프로젝트 이름
    #[serde(default)]
    pub active_project: Option<String>,
    /// 등록된 게임 목록 (설정창에서 관리)
    #[serde(default)]
    pub games: Vec<String>,
    /// 등록된 프로젝트 목록 (설정창에서 관리)
    #[serde(default)]
    pub projects: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_agents: vec![],
            pilot_level: default_level(),
            sync_mode: default_sync(),
            workspace_mode: default_mode(),
            active_game: None,
            active_project: None,
            games: vec![],
            projects: vec![],
        }
    }
}

fn default_level() -> String {
    "auto".into()
}

fn default_sync() -> String {
    "gist".into()
}

fn default_mode() -> String {
    "game".into()
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
        if self.workspace_mode != "project" {
            self.workspace_mode = default_mode();
        }
        self.enabled_agents.retain(|s| !s.is_empty());
        self.enabled_agents.dedup();
        for list in [&mut self.games, &mut self.projects] {
            list.retain(|s| !s.is_empty());
            list.dedup();
        }
        // 활성 선택이 목록에 없으면 해제 (삭제된 게임/프로젝트 잔존 방지)
        if let Some(g) = &self.active_game {
            if !self.games.iter().any(|x| x == g) {
                self.active_game = None;
            }
        }
        if let Some(p) = &self.active_project {
            if !self.projects.iter().any(|x| x == p) {
                self.active_project = None;
            }
        }
    }

    /// 현재 모드의 활성 작업 공간 태그 (예: "game:마을도우미" / "project:access").
    /// 노트 자동 태깅의 기본값으로 쓰인다.
    pub fn active_workspace_tag(&self) -> Option<String> {
        match self.workspace_mode.as_str() {
            "project" => self.active_project.as_ref().map(|p| format!("project:{p}")),
            _ => self.active_game.as_ref().map(|g| format!("game:{g}")),
        }
    }

    /// 분류 후보 태그 목록 (게임+프로젝트 전부).
    pub fn workspace_candidates(&self) -> Vec<String> {
        self.games.iter().map(|g| format!("game:{g}"))
            .chain(self.projects.iter().map(|p| format!("project:{p}")))
            .collect()
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
            ..Default::default()
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

#[cfg(test)]
mod workspace_tests {
    use super::*;

    #[test]
    fn workspace_defaults_and_clamps() {
        let mut s = Settings { workspace_mode: "galaxy".into(), ..Default::default() };
        s.normalize();
        assert_eq!(s.workspace_mode, "game");
        assert_eq!(s.active_workspace_tag(), None); // 선택 없음
        assert!(s.workspace_candidates().is_empty());
    }

    #[test]
    fn active_tag_follows_mode() {
        let mut s = Settings {
            workspace_mode: "game".into(),
            active_game: Some("마을도우미".into()),
            active_project: Some("access".into()),
            games: vec!["마을도우미".into()],
            projects: vec!["access".into()],
            ..Default::default()
        };
        assert_eq!(s.active_workspace_tag().unwrap(), "game:마을도우미");
        s.workspace_mode = "project".into();
        assert_eq!(s.active_workspace_tag().unwrap(), "project:access");
    }

    #[test]
    fn normalize_drops_stale_active_selection() {
        let mut s = Settings {
            active_game: Some("삭제된게임".into()),
            games: vec!["다른게임".into()],
            ..Default::default()
        };
        s.normalize();
        assert_eq!(s.active_game, None);
    }
}

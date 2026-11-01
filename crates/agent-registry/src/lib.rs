//! agent-registry: detects installed agent tools and manages the Access MCP
//! registration inside each tool's config file.
//!
//! Design rules (제품기획 v1.2 · P1):
//! - Every write is preceded by a backup (`<name>.access-bak.<ext>`).
//! - Registration is idempotent: an existing entry is left untouched.
//! - A config file that exists but fails to parse is reported as
//!   [`RegError::Schema`] and never rewritten — self-healing must degrade to
//!   a manual badge instead of corrupting a tool's config after an update
//!   changed its format.

use std::path::{Path, PathBuf};

/// Agent tools we know how to register (MCP, file-config based).
/// Custom REST agents (hermes/omp/…) need no file registration and are
/// handled by the settings layer, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    /// `~/.zcode/cli/config.json` → `mcp.servers`
    ZCode,
    /// `~/.claude.json` → `mcpServers` (user scope)
    ClaudeCode,
    /// `~/.codex/config.toml` → `[mcp_servers.*]`
    Codex,
    /// `~/Documents/Cline/MCP/mcp_settings.json` → `mcpServers`
    Cline,
    /// Kilo Code / Roo Code global MCP settings (candidates, 실험적)
    KiloCode,
    /// `~/.openclaw/openclaw.json` → `mcpServers` (실험적)
    OpenClaw,
}

/// Static description of one supported agent tool.
pub struct AgentSpec {
    pub id: &'static str,
    pub display: &'static str,
    pub kind: AgentKind,
    /// Experimental formats may move between releases; surfaced to the UI.
    pub experimental: bool,
}

pub const SPECS: &[AgentSpec] = &[
    AgentSpec { id: "zcode", display: "ZCode", kind: AgentKind::ZCode, experimental: false },
    AgentSpec { id: "claude-code", display: "Claude Code", kind: AgentKind::ClaudeCode, experimental: false },
    AgentSpec { id: "codex", display: "Codex", kind: AgentKind::Codex, experimental: false },
    AgentSpec { id: "cline", display: "Cline", kind: AgentKind::Cline, experimental: false },
    AgentSpec { id: "kilo-code", display: "Kilo Code", kind: AgentKind::KiloCode, experimental: true },
    AgentSpec { id: "openclaw", display: "OpenClaw", kind: AgentKind::OpenClaw, experimental: true },
];

pub const SERVER_KEY: &str = "access-todo";

/// Result of a register/unregister call, for UI + self-heal reporting.
#[derive(Debug)]
pub enum Outcome {
    /// Entry already present — nothing written.
    AlreadyRegistered,
    /// Written now; the pre-write backup path is included.
    Registered { backup: Option<PathBuf> },
    /// Removed the entry; backup path included.
    Unregistered { backup: Option<PathBuf> },
    /// The tool's config file does not exist (tool not installed).
    NotInstalled,
}

#[derive(Debug)]
pub enum RegError {
    /// Unknown agent id.
    UnknownAgent(String),
    /// Config file exists but its format changed — never auto-rewritten.
    Schema { path: PathBuf, reason: String },
    Io(std::io::Error),
}

impl From<std::io::Error> for RegError {
    fn from(e: std::io::Error) -> Self {
        RegError::Io(e)
    }
}

impl std::fmt::Display for RegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegError::UnknownAgent(id) => write!(f, "unknown agent: {id}"),
            RegError::Schema { path, reason } => {
                write!(f, "config schema changed at {} ({reason}) — manual fix required", path.display())
            }
            RegError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

/// One row of the agent list shown in the settings window.
#[derive(Debug, serde::Serialize)]
pub struct DetectedAgent {
    pub id: String,
    pub display: String,
    pub integration: &'static str, // "mcp"
    pub experimental: bool,
    /// Tool (or one of its candidate config locations) found on disk.
    pub installed: bool,
    /// Config file the registration would live in (may not exist yet).
    pub config_path: String,
    /// Access MCP entry currently present in that file.
    pub registered: bool,
}

/// Registry bound to a home directory so tests stay hermetic.
pub struct Registry {
    home: PathBuf,
}

impl Registry {
    pub fn with_home(home: PathBuf) -> Self {
        Self { home }
    }

    /// Uses USERPROFILE (Windows) / HOME as the home directory.
    pub fn new() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        Self::with_home(home)
    }

    /// Candidate config files per kind, in priority order.
    fn config_candidates(&self, kind: AgentKind) -> Vec<PathBuf> {
        let home = &self.home;
        match kind {
            AgentKind::ZCode => vec![home.join(".zcode").join("cli").join("config.json")],
            AgentKind::ClaudeCode => vec![home.join(".claude.json")],
            AgentKind::Codex => vec![home.join(".codex").join("config.toml")],
            AgentKind::Cline => vec![
                home.join("Documents").join("Cline").join("MCP").join("mcp_settings.json"),
                // OneDrive-redirected Documents folder
                home.join("OneDrive").join("Documents").join("Cline").join("MCP").join("mcp_settings.json"),
            ],
            AgentKind::KiloCode => vec![
                home.join(".kilo-code").join("mcp_settings.json"),
                home.join(".roo").join("mcp_settings.json"),
            ],
            AgentKind::OpenClaw => vec![home.join(".openclaw").join("openclaw.json")],
        }
    }

    fn spec(&self, id: &str) -> Result<&'static AgentSpec, RegError> {
        SPECS
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| RegError::UnknownAgent(id.into()))
    }

    /// Detect all known agent tools. Never fails: unreadable files are
    /// reported as installed-but-unregistered with a schema note folded into
    /// `registered=false` (health check surfaces the details).
    pub fn detect(&self) -> Vec<DetectedAgent> {
        SPECS.iter()
            .map(|spec| {
                let candidates = self.config_candidates(spec.kind);
                let existing = candidates.iter().find(|p| p.exists());
                let target = existing.cloned().unwrap_or_else(|| candidates[0].clone());
                let registered = existing
                    .map(|p| self.json_or_toml_registered(p, spec.kind))
                    .unwrap_or(false);
                DetectedAgent {
                    id: spec.id.to_string(),
                    display: spec.display.to_string(),
                    integration: "mcp",
                    experimental: spec.experimental,
                    installed: existing.is_some(),
                    config_path: target.display().to_string(),
                    registered,
                }
            })
            .collect()
    }

    fn json_or_toml_registered(&self, path: &Path, kind: AgentKind) -> bool {
        if kind == AgentKind::Codex {
            match std::fs::read_to_string(path) {
                Ok(s) => s.contains(&format!("[mcp_servers.{SERVER_KEY}]")),
                Err(_) => false,
            }
        } else {
            match std::fs::read_to_string(path) {
                Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                    Ok(v) => json_entry(&v, kind).is_some(),
                    Err(_) => false,
                },
                Err(_) => false,
            }
        }
    }

    /// Current registration state for one agent (false if not installed).
    pub fn is_registered_ok(&self, id: &str) -> bool {
        let Ok(spec) = self.spec(id) else { return false };
        self.config_candidates(spec.kind)
            .iter()
            .find(|p| p.exists())
            .map(|p| self.json_or_toml_registered(p, spec.kind))
            .unwrap_or(false)
    }

    /// Register the Access MCP server in the agent's config.
    pub fn register(&self, id: &str, exe: &Path) -> Result<Outcome, RegError> {
        let spec = self.spec(id)?;
        let candidates = self.config_candidates(spec.kind);
        let Some(path) = candidates.iter().find(|p| p.exists()) else {
            return Ok(Outcome::NotInstalled);
        };
        match spec.kind {
            AgentKind::Codex => self.register_toml(path, exe),
            _ => self.register_json(path, spec.kind, exe),
        }
    }

    /// Remove the Access MCP entry (settings UI toggle OFF).
    pub fn unregister(&self, id: &str) -> Result<Outcome, RegError> {
        let spec = self.spec(id)?;
        let candidates = self.config_candidates(spec.kind);
        let Some(path) = candidates.iter().find(|p| p.exists()) else {
            return Ok(Outcome::NotInstalled);
        };
        match spec.kind {
            AgentKind::Codex => self.unregister_toml(path),
            _ => self.unregister_json(path, spec.kind),
        }
    }

    // ── JSON configs ──

    fn register_json(&self, path: &Path, kind: AgentKind, exe: &Path) -> Result<Outcome, RegError> {
        let text = std::fs::read_to_string(path)?;
        let mut root: serde_json::Value = serde_json::from_str(&text).map_err(|e| RegError::Schema {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        if json_entry(&root, kind).is_some() {
            return Ok(Outcome::AlreadyRegistered);
        }
        let entry = match kind {
            AgentKind::ZCode => serde_json::json!({ "type": "stdio", "command": exe_string(exe, false) }),
            // Claude / Cline / Kilo / OpenClaw accept optional type; plain command is the common form.
            _ => serde_json::json!({ "command": exe_string(exe, false) }),
        };
        let servers = match kind {
            AgentKind::ZCode => root
                .as_object_mut().unwrap()
                .entry("mcp").or_insert_with(|| serde_json::json!({}))
                .as_object_mut().unwrap()
                .entry("servers").or_insert_with(|| serde_json::json!({}))
                .as_object_mut().unwrap(),
            _ => root
                .as_object_mut().unwrap()
                .entry("mcpServers").or_insert_with(|| serde_json::json!({}))
                .as_object_mut().unwrap(),
        };
        servers.insert(SERVER_KEY.into(), entry);
        let backup = backup_file(path)?;
        std::fs::write(path, serde_json::to_string_pretty(&root).unwrap())?;
        Ok(Outcome::Registered { backup })
    }

    fn unregister_json(&self, path: &Path, kind: AgentKind) -> Result<Outcome, RegError> {
        let text = std::fs::read_to_string(path)?;
        let mut root: serde_json::Value = serde_json::from_str(&text).map_err(|e| RegError::Schema {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        let removed = match kind {
            AgentKind::ZCode => root
                .get_mut("mcp").and_then(|m| m.get_mut("servers"))
                .and_then(|s| s.as_object_mut())
                .and_then(|s| s.remove(SERVER_KEY)),
            _ => root
                .get_mut("mcpServers")
                .and_then(|s| s.as_object_mut())
                .and_then(|s| s.remove(SERVER_KEY)),
        };
        if removed.is_none() {
            return Ok(Outcome::AlreadyRegistered); // nothing to remove — same no-op outcome
        }
        let backup = backup_file(path)?;
        std::fs::write(path, serde_json::to_string_pretty(&root).unwrap())?;
        Ok(Outcome::Unregistered { backup })
    }

    // ── TOML configs (Codex) ──

    fn register_toml(&self, path: &Path, exe: &Path) -> Result<Outcome, RegError> {
        let text = std::fs::read_to_string(path)?;
        if text.contains(&format!("[mcp_servers.{SERVER_KEY}]")) {
            return Ok(Outcome::AlreadyRegistered);
        }
        // TOML tables are order-scoped to end-of-file, so appending is safe.
        // Codex accepts Windows paths with forward slashes.
        let block = format!(
            "\n[mcp_servers.{SERVER_KEY}]\ncommand = \"{}\"\n",
            exe_string(exe, true)
        );
        let backup = backup_file(path)?;
        let mut out = text;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&block);
        std::fs::write(path, out)?;
        Ok(Outcome::Registered { backup })
    }

    fn unregister_toml(&self, path: &Path) -> Result<Outcome, RegError> {
        let text = std::fs::read_to_string(path)?;
        let header = format!("[mcp_servers.{SERVER_KEY}]");
        let Some(start) = text.lines().position(|l| l.trim() == header) else {
            return Ok(Outcome::AlreadyRegistered);
        };
        let lines: Vec<&str> = text.lines().collect();
        // The table extends until the next header line or EOF.
        let mut end = lines.len();
        for (i, l) in lines.iter().enumerate().skip(start + 1) {
            if l.trim_start().starts_with('[') {
                end = i;
                break;
            }
        }
        let backup = backup_file(path)?;
        let mut kept: Vec<&str> = lines[..start].to_vec();
        kept.extend_from_slice(&lines[end..]);
        let mut out = kept.join("\n");
        if text.ends_with('\n') {
            out.push('\n');
        }
        std::fs::write(path, out)?;
        Ok(Outcome::Unregistered { backup })
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigate a parsed JSON config to the servers map for this kind.
fn json_entry<'a>(root: &'a serde_json::Value, kind: AgentKind) -> Option<&'a serde_json::Value> {
    match kind {
        AgentKind::ZCode => root.get("mcp").and_then(|m| m.get("servers")).and_then(|s| s.get(SERVER_KEY)),
        _ => root.get("mcpServers").and_then(|s| s.get(SERVER_KEY)),
    }
}

/// exe path as a config-friendly string (forward slashes for TOML).
fn exe_string(exe: &Path, forward_slashes: bool) -> String {
    let s = exe.display().to_string();
    if forward_slashes {
        s.replace('\\', "/")
    } else {
        s
    }
}

/// Copy `path` to `<stem>.access-bak.<ext>` so the change is reversible.
fn backup_file(path: &Path) -> Result<Option<PathBuf>, RegError> {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("config");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("json");
    let bak = path.parent().unwrap_or(Path::new(".")).join(format!("{stem}.access-bak.{ext}"));
    match std::fs::copy(path, &bak) {
        Ok(_) => Ok(Some(bak)),
        Err(e) => {
            // A failed backup must not block registration — log-only.
            eprintln!("warn: backup failed for {}: {e}", path.display());
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("access-registry-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn zcode_register_unregister_roundtrip() {
        let home = temp_home("zcode");
        let cfg = home.join(".zcode").join("cli").join("config.json");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        std::fs::write(&cfg, r#"{"mcp":{"servers":{"other":{"command":"x"}}}}"#).unwrap();

        let reg = Registry::with_home(home.clone());
        let exe = Path::new(r"C:\bin\Access-MCP.exe");

        assert!(!reg.is_registered_ok("zcode"));
        match reg.register("zcode", exe).unwrap() {
            Outcome::Registered { .. } => {}
            o => panic!("expected Registered, got {o:?}"),
        }
        // idempotent
        assert!(matches!(reg.register("zcode", exe).unwrap(), Outcome::AlreadyRegistered));
        assert!(reg.is_registered_ok("zcode"));

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(v["mcp"]["servers"]["access-todo"]["command"], exe.display().to_string());
        // other entries preserved
        assert!(v["mcp"]["servers"]["other"].is_object());

        match reg.unregister("zcode").unwrap() {
            Outcome::Unregistered { .. } => {}
            o => panic!("expected Unregistered, got {o:?}"),
        }
        assert!(!reg.is_registered_ok("zcode"));
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert!(v["mcp"]["servers"]["access-todo"].is_null());
        assert!(v["mcp"]["servers"]["other"].is_object());
    }

    #[test]
    fn claude_preserves_foreign_keys() {
        let home = temp_home("claude");
        let cfg = home.join(".claude.json");
        std::fs::write(
            &cfg,
            r#"{"firstLaunch":true,"mcpServers":{"foo":{"command":"foo"}},"projects":{}}"#,
        )
        .unwrap();

        let reg = Registry::with_home(home);
        reg.register("claude-code", Path::new("/bin/Access-MCP.exe")).unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(v["firstLaunch"], true);
        assert!(v["projects"].is_object());
        assert!(v["mcpServers"]["foo"].is_object());
        assert!(v["mcpServers"]["access-todo"].is_object());
    }

    #[test]
    fn codex_toml_roundtrip_and_idempotency() {
        let home = temp_home("codex");
        let cfg = home.join(".codex").join("config.toml");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        std::fs::write(&cfg, "model = \"gpt-5\"\n").unwrap();

        let reg = Registry::with_home(home);
        let exe = Path::new(r"C:\Users\me\bin\Access-MCP.exe");
        match reg.register("codex", exe).unwrap() {
            Outcome::Registered { .. } => {}
            o => panic!("{o:?}"),
        }
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("[mcp_servers.access-todo]"));
        assert!(text.contains("C:/Users/me/bin/Access-MCP.exe"));
        assert!(text.contains("model = \"gpt-5\""));
        assert!(matches!(reg.register("codex", exe).unwrap(), Outcome::AlreadyRegistered));

        reg.unregister("codex").unwrap();
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(!text.contains("access-todo"));
        assert!(text.contains("model = \"gpt-5\""));
    }

    #[test]
    fn codex_unregister_keeps_following_tables() {
        let home = temp_home("codex2");
        let cfg = home.join(".codex").join("config.toml");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg,
            "[mcp_servers.access-todo]\ncommand = \"x\"\n\n[mcp_servers.other]\ncommand = \"y\"\n",
        )
        .unwrap();

        let reg = Registry::with_home(home);
        reg.unregister("codex").unwrap();
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(!text.contains("access-todo"));
        assert!(text.contains("[mcp_servers.other]"));
    }

    #[test]
    fn schema_change_never_rewrites() {
        let home = temp_home("schema");
        let cfg = home.join(".claude.json");
        std::fs::write(&cfg, "{not valid json").unwrap();

        let reg = Registry::with_home(home);
        match reg.register("claude-code", Path::new("/bin/exe")) {
            Err(RegError::Schema { .. }) => {}
            other => panic!("expected Schema, got {other:?}"),
        }
        // file untouched
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), "{not valid json");
    }

    #[test]
    fn not_installed_is_not_an_error() {
        let home = temp_home("none");
        let reg = Registry::with_home(home);
        assert!(matches!(
            reg.register("cline", Path::new("/bin/exe")).unwrap(),
            Outcome::NotInstalled
        ));
        let detected = reg.detect();
        let cline = detected.iter().find(|d| d.id == "cline").unwrap();
        assert!(!cline.installed && !cline.registered);
    }

    #[test]
    fn detect_finds_all_and_flags_registration() {
        let home = temp_home("detect");
        std::fs::create_dir_all(home.join(".zcode").join("cli")).unwrap();
        std::fs::write(
            home.join(".zcode").join("cli").join("config.json"),
            r#"{"mcp":{"servers":{"access-todo":{"command":"x"}}}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::write(home.join(".codex").join("config.toml"), "model=\"o\"\n").unwrap();

        let reg = Registry::with_home(home);
        let detected = reg.detect();
        assert_eq!(detected.len(), SPECS.len());
        let zcode = detected.iter().find(|d| d.id == "zcode").unwrap();
        assert!(zcode.installed && zcode.registered);
        let codex = detected.iter().find(|d| d.id == "codex").unwrap();
        assert!(codex.installed && !codex.registered);
        let claude = detected.iter().find(|d| d.id == "claude-code").unwrap();
        assert!(!claude.installed);
    }
}

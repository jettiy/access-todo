//! Worklog generation: collects completed todos, groups by game system,
//! and renders a markdown body for a DevNote.

use std::collections::BTreeMap;

use crate::model::{DevNote, GameSystem, NoteType, Todo, new_note_id};

/// Extract the game system from a todo's `"system:<name>"` tag.
pub fn parse_system_tag(tags: &[String]) -> Option<GameSystem> {
    for tag in tags {
        if let Some(rest) = tag.strip_prefix("system:") {
            return match rest {
                "combat" => Some(GameSystem::Combat),
                "inventory" => Some(GameSystem::Inventory),
                "quest" => Some(GameSystem::Quest),
                "skill" => Some(GameSystem::Skill),
                "ui" => Some(GameSystem::Ui),
                "audio" => Some(GameSystem::Audio),
                "map" => Some(GameSystem::Map),
                "ai" => Some(GameSystem::Ai),
                "save" => Some(GameSystem::Save),
                "network" => Some(GameSystem::Network),
                "etc" => Some(GameSystem::Etc),
                _ => Some(GameSystem::Etc),
            };
        }
    }
    None
}

/// Extract the version from a todo's `"version:<ver>"` tag.
pub fn parse_version_tag(tags: &[String]) -> Option<String> {
    for tag in tags {
        if let Some(rest) = tag.strip_prefix("version:") {
            return Some(rest.to_string());
        }
    }
    None
}

/// Generate a Worklog DevNote from a slice of completed todos.
///
/// Todos are grouped by game system (parsed from their `system:<name>` tag,
/// defaulting to `Etc`). The body is markdown with `## SystemName` headers
/// and `- title: summary` bullets.
pub fn generate_worklog(todos: &[Todo], date: &str, _agent: Option<&str>) -> DevNote {
    // Group todos by game system.
    let mut groups: BTreeMap<GameSystem, Vec<&Todo>> = BTreeMap::new();
    let mut versions: Vec<String> = Vec::new();

    for t in todos {
        let sys = parse_system_tag(&t.tags).unwrap_or(GameSystem::Etc);
        groups.entry(sys).or_default().push(t);
        if let Some(v) = parse_version_tag(&t.tags) {
            if !versions.contains(&v) {
                versions.push(v);
            }
        }
    }

    // Build markdown body.
    let mut body = String::new();
    let count = todos.len();
    body.push_str(&format!("*총 {count}개 작업 완료*\n\n"));

    for (sys, items) in &groups {
        let sys_name = system_display_name(sys);
        body.push_str(&format!("## {sys_name}\n"));
        for t in items {
            let summary = t.note.as_deref().unwrap_or("").trim_start_matches("✓ ");
            if summary.is_empty() {
                body.push_str(&format!("- {}\n", t.title));
            } else {
                body.push_str(&format!("- {}: {}\n", t.title, summary));
            }
        }
        body.push('\n');
    }

    if !versions.is_empty() {
        body.push_str(&format!("**버전**: {}", versions.join(", ")));
    }

    let linked: Vec<String> = todos.iter().map(|t| t.id.clone()).collect();

    DevNote {
        id: new_note_id(),
        note_type: NoteType::Worklog,
        title: format!("{date} 작업 로그"),
        body,
        game_system: None,
        version: versions.first().cloned(),
        linked_todos: linked,
        tags: vec![],
        created_at: chrono::Utc::now(),
        created_by: "system".into(),
        updated_at: None,
        updated_by: None,
    }
}

fn system_display_name(sys: &GameSystem) -> &'static str {
    match sys {
        GameSystem::Combat => "Combat (전투)",
        GameSystem::Inventory => "Inventory (인벤토리)",
        GameSystem::Quest => "Quest (퀘스트)",
        GameSystem::Skill => "Skill (스킬)",
        GameSystem::Ui => "Ui (UI)",
        GameSystem::Audio => "Audio (오디오)",
        GameSystem::Map => "Map (맵)",
        GameSystem::Ai => "Ai (AI)",
        GameSystem::Save => "Save (세이브)",
        GameSystem::Network => "Network (네트워크)",
        GameSystem::Etc => "Etc (기타)",
    }
}

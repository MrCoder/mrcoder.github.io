//! Claude Code history: one JSONL transcript per project under `~/.claude/projects`.

use anyhow::Result;
use std::path::Path;
use vcs_core::time::{in_window, iso_timestamp};
use vcs_core::{Record, Role};

use crate::context::{is_machine_context, strip_leading_context, strip_trailing_context};
use crate::{
    file_fingerprint, jsonl_files, read_json_lines, text_from_blocks, Collected, FileUnit,
    SourceKind, SourceReport, UnitFilter,
};

const VISIBLE_BLOCKS: [&str; 1] = ["text"];

/// True when this `user` entry is text the person actually typed.
///
/// Claude Code writes tool results, task notifications, SDK traffic, and injected system context
/// into the same `user` channel as human prompts. Indexing those would put content into search that
/// the person never saw as their own message.
fn is_human_prompt(value: &serde_json::Value) -> bool {
    if value.get("isMeta").and_then(|v| v.as_bool()) == Some(true) {
        return false;
    }
    if value.get("toolUseResult").map_or(false, |v| !v.is_null()) {
        return false;
    }
    if value.get("sourceToolAssistantUUID").map_or(false, |v| !v.is_null()) {
        return false;
    }
    if let Some(origin) = value.get("origin").and_then(|v| v.as_object()) {
        match origin.get("kind").and_then(|v| v.as_str()) {
            Some("task-notification") => return false,
            Some("human") => return true,
            _ => {}
        }
    }
    let prompt_source = value.get("promptSource").and_then(|v| v.as_str());
    if matches!(prompt_source, Some("system") | Some("sdk"))
        || value.get("entrypoint").and_then(|v| v.as_str()) == Some("sdk-cli")
    {
        return false;
    }
    let has_tool_result = value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_array())
        .map_or(false, |blocks| {
            blocks.iter().any(|block| {
                block.get("type").and_then(|v| v.as_str()) == Some("tool_result")
            })
        });
    if has_tool_result {
        return false;
    }
    matches!(
        prompt_source,
        None | Some("typed") | Some("queued") | Some("suggestion_accepted")
    )
}

pub fn collect(
    root: &Path,
    cutoff: Option<f64>,
    max_chars: usize,
    filter: UnitFilter<'_>,
) -> Result<Collected> {
    let mut errors: Vec<String> = Vec::new();
    let mut units: Vec<FileUnit> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();
    let files = jsonl_files(&root.join("projects"));

    for path in &files {
        let scope = format!("claude:{}", path.display());
        let fingerprint = file_fingerprint(path);
        if !filter(&scope, &fingerprint) {
            unchanged.push(scope);
            continue;
        }
        let mut records: Vec<Record> = Vec::new();
        for value in read_json_lines(path, &mut errors) {
            let entry_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !matches!(entry_type, "user" | "assistant") {
                continue;
            }
            // A sidechain is a subagent transcript. The person never saw it in their own thread.
            if value.get("isSidechain").and_then(|v| v.as_bool()) == Some(true) {
                continue;
            }
            let Some(message) = value.get("message").filter(|v| v.is_object()) else { continue };
            let Some(role) = message
                .get("role")
                .and_then(|v| v.as_str())
                .and_then(Role::parse)
            else {
                continue;
            };
            if role == Role::User && !is_human_prompt(&value) {
                continue;
            }
            if role == Role::Assistant
                && value.get("entrypoint").and_then(|v| v.as_str()) == Some("sdk-cli")
            {
                continue;
            }
            let timestamp = value.get("timestamp").cloned().unwrap_or(serde_json::Value::Null);
            if !in_window(&timestamp, cutoff) {
                continue;
            }
            let session_id = value
                .get("sessionId")
                .or_else(|| value.get("session_id"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });
            let text = text_from_blocks(message.get("content"), &VISIBLE_BLOCKS);
            // A slash command leaves two user entries: the typed `/name`, which was on screen, and
            // a `<command-name>…</command-name>` block, which was not.
            let text = if role == Role::User {
                if is_machine_context(&text) {
                    continue;
                }
                strip_trailing_context(&strip_leading_context(&text))
            } else {
                text
            };
            let cwd = value.get("cwd").and_then(|v| v.as_str()).map(str::to_string);
            if let Some(record) = Record::new(
                "claude",
                &session_id,
                iso_timestamp(&timestamp),
                role,
                cwd,
                &text,
                max_chars,
            ) {
                records.push(record);
            }
        }
        units.push(FileUnit { scope, fingerprint, records });
    }

    let status = if files.is_empty() { "missing" } else { "ok" };
    let records = units.iter().map(|unit| unit.records.len()).sum();
    Ok(Collected {
        report: SourceReport {
            source: SourceKind::Claude,
            label: SourceKind::Claude.label(),
            root: root.display().to_string(),
            status: status.into(),
            files_read: files.len(),
            records,
            errors,
        },
        units,
        unchanged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("vcs-claude-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    fn write(root: &Path, lines: &[&str]) {
        let path = root.join("projects/-tmp-project/session.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, lines.join("\n")).unwrap();
    }

    #[test]
    fn a_typed_prompt_and_the_visible_answer_are_indexed() {
        let root = temporary_root("visible");
        write(
            &root,
            &[
                r#"{"type":"user","sessionId":"s1","cwd":"/tmp/project","timestamp":"2026-09-01T10:00:00Z","message":{"role":"user","content":[{"type":"text","text":"rebuild the local index"}]}}"#,
                r#"{"type":"assistant","sessionId":"s1","timestamp":"2026-09-01T10:00:09Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"hidden"},{"type":"text","text":"index rebuilt"},{"type":"tool_use","name":"Bash","input":{}}]}}"#,
            ],
        );
        let collected = crate::collect(crate::SourceKind::Claude, &root, None, 12_000).unwrap();
        let records = collected.records();
        let texts: Vec<&str> = records.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["rebuild the local index", "index rebuilt"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn tool_results_and_injected_context_never_enter_the_index() {
        let root = temporary_root("injected");
        write(
            &root,
            &[
                r#"{"type":"user","sessionId":"s1","timestamp":"2026-09-01T10:00:00Z","toolUseResult":{"stdout":"x"},"message":{"role":"user","content":[{"type":"text","text":"tool output text"}]}}"#,
                r#"{"type":"user","sessionId":"s1","timestamp":"2026-09-01T10:00:01Z","message":{"role":"user","content":[{"type":"tool_result","content":"result body"}]}}"#,
                r#"{"type":"user","sessionId":"s1","timestamp":"2026-09-01T10:00:02Z","isMeta":true,"message":{"role":"user","content":[{"type":"text","text":"meta caveat"}]}}"#,
                r#"{"type":"user","sessionId":"s1","timestamp":"2026-09-01T10:00:03Z","promptSource":"system","message":{"role":"user","content":[{"type":"text","text":"system reminder"}]}}"#,
            ],
        );
        assert!(crate::collect(crate::SourceKind::Claude, &root, None, 12_000).unwrap().records().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_slash_command_block_is_dropped_and_the_typed_command_kept() {
        let root = temporary_root("slash");
        write(
            &root,
            &[
                r#"{"type":"user","sessionId":"s1","timestamp":"2026-09-02T00:20:01Z","message":{"role":"user","content":"/compact"}}"#,
                r#"{"type":"user","sessionId":"s1","timestamp":"2026-09-02T00:20:02Z","message":{"role":"user","content":"<command-name>/compact</command-name>\n            <command-message>compact</command-message>\n            <command-args></command-args>"}}"#,
            ],
        );
        let records = crate::collect(crate::SourceKind::Claude, &root, None, 12_000).unwrap().records();
        assert_eq!(records.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(), vec!["/compact"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_subagent_sidechain_is_not_the_users_visible_conversation() {
        let root = temporary_root("sidechain");
        write(
            &root,
            &[
                r#"{"type":"user","sessionId":"s1","isSidechain":true,"timestamp":"2026-09-01T10:00:00Z","message":{"role":"user","content":[{"type":"text","text":"subagent prompt"}]}}"#,
            ],
        );
        assert!(crate::collect(crate::SourceKind::Claude, &root, None, 12_000).unwrap().records().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_explicit_human_origin_wins_over_a_missing_prompt_source() {
        let root = temporary_root("origin");
        write(
            &root,
            &[
                r#"{"type":"user","sessionId":"s1","origin":{"kind":"human"},"promptSource":"sdk","timestamp":"2026-09-01T10:00:00Z","message":{"role":"user","content":[{"type":"text","text":"typed by a person"}]}}"#,
            ],
        );
        assert_eq!(crate::collect(crate::SourceKind::Claude, &root, None, 12_000).unwrap().records().len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}

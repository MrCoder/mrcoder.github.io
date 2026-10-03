//! Codex CLI history: rollout JSONL under `~/.codex/sessions` plus the typed-prompt history file.

use anyhow::Result;
use std::path::Path;
use vcs_core::time::{in_window, iso_timestamp};
use vcs_core::{Record, Role};

use crate::{
    file_fingerprint, jsonl_files, read_json_lines, text_from_blocks, Collected, FileUnit,
    SourceKind, SourceReport, UnitFilter,
};

const ASSISTANT_BLOCKS: [&str; 2] = ["output_text", "text"];

/// Codex's own instruction injection. It is not typed and the interface never shows it as a message.
use crate::context::{is_machine_context, strip_leading_context, strip_trailing_context};

/// A typed prompt reaches a recent rollout as `input_text`.
const USER_BLOCKS: [&str; 2] = ["input_text", "text"];

pub fn collect(
    root: &Path,
    cutoff: Option<f64>,
    max_chars: usize,
    filter: UnitFilter<'_>,
) -> Result<Collected> {
    let mut errors: Vec<String> = Vec::new();
    let mut units: Vec<FileUnit> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();
    let mut files = jsonl_files(&root.join("sessions"));
    files.extend(jsonl_files(&root.join("archived_sessions")));

    for path in &files {
        let scope = format!("codex:{}", path.display());
        let fingerprint = file_fingerprint(path);
        if !filter(&scope, &fingerprint) {
            unchanged.push(scope);
            continue;
        }
        let mut records: Vec<Record> = Vec::new();
        let values = read_json_lines(path, &mut errors);
        let metadata = values
            .iter()
            .find(|value| value.get("type").and_then(|v| v.as_str()) == Some("session_meta"))
            .and_then(|value| value.get("payload"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // Subthreads and forwarded sessions duplicate a parent transcript; the parent already carries
        // the visible exchange.
        if !metadata.get("parent_thread_id").unwrap_or(&serde_json::Value::Null).is_null()
            || metadata.get("source").map_or(false, |value| value.is_object())
        {
            // The parent transcript already holds this exchange. Record the scope so the file is not
            // re-read on every run.
            units.push(FileUnit { scope, fingerprint, records: Vec::new() });
            continue;
        }
        let session_id = metadata
            .get("id")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.rsplit('-').next())
                    .unwrap_or("unknown")
                    .to_string()
            });
        let cwd = metadata.get("cwd").and_then(|value| value.as_str()).map(str::to_string);

        for value in &values {
            let kind = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let timestamp = value.get("timestamp").cloned().unwrap_or(serde_json::Value::Null);
            if kind == "event_msg" {
                let Some(payload) = value.get("payload") else { continue };
                if payload.get("type").and_then(|v| v.as_str()) != Some("user_message") {
                    continue;
                }
                if !in_window(&timestamp, cutoff) {
                    continue;
                }
                let text = payload.get("message").and_then(|v| v.as_str()).unwrap_or("");
                // The same injections reach older rollouts through the event channel.
                if is_machine_context(text) {
                    continue;
                }
                let text = strip_trailing_context(&strip_leading_context(text));
                if let Some(record) = Record::new(
                    "codex",
                    &session_id,
                    iso_timestamp(&timestamp),
                    Role::User,
                    cwd.clone(),
                    &text,
                    max_chars,
                ) {
                    records.push(record);
                }
                continue;
            }
            if kind != "response_item" {
                continue;
            }
            let Some(payload) = value.get("payload") else { continue };
            if payload.get("type").and_then(|v| v.as_str()) != Some("message") {
                continue;
            }
            // Visible messages only. Reasoning items, function calls, and their output carry a
            // different payload type and are dropped by the check above; the `developer` role holds
            // injected instructions the person never saw as a message.
            //
            // Recent rollouts write the typed prompt here as a `user` message with `input_text`
            // blocks and emit no `event_msg` for it. Reading only the assistant side lost every
            // recent prompt: measured on this computer, one project held 1,906 assistant records and
            // zero user records before this branch existed.
            let Some(role) = payload
                .get("role")
                .and_then(|v| v.as_str())
                .and_then(Role::parse)
            else {
                continue;
            };
            if !in_window(&timestamp, cutoff) {
                continue;
            }
            let blocks = match role {
                Role::User => &USER_BLOCKS,
                Role::Assistant => &ASSISTANT_BLOCKS,
            };
            let text = text_from_blocks(payload.get("content"), blocks);
            let text = if role == Role::User {
                if is_machine_context(&text) {
                    continue;
                }
                strip_trailing_context(&strip_leading_context(&text))
            } else {
                text
            };
            if let Some(record) = Record::new(
                "codex",
                &session_id,
                iso_timestamp(&timestamp),
                role,
                cwd.clone(),
                &text,
                max_chars,
            ) {
                records.push(record);
            }
        }
        units.push(FileUnit { scope, fingerprint, records });
    }

    let history = root.join("history.jsonl");
    if history.exists() {
        let scope = format!("codex:{}", history.display());
        let fingerprint = file_fingerprint(&history);
        let mut records: Vec<Record> = Vec::new();
        if !filter(&scope, &fingerprint) {
            unchanged.push(scope.clone());
            files.push(history.clone());
            return finish(root, files, units, unchanged, errors);
        }
        for value in read_json_lines(&history, &mut errors) {
            let timestamp = value.get("ts").cloned().unwrap_or(serde_json::Value::Null);
            if !in_window(&timestamp, cutoff) {
                continue;
            }
            let session_id = value
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let text = value.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(record) = Record::new(
                "codex",
                &session_id,
                iso_timestamp(&timestamp),
                Role::User,
                None,
                text,
                max_chars,
            ) {
                records.push(record);
            }
        }
        units.push(FileUnit { scope, fingerprint, records });
        files.push(history);
    }

    finish(root, files, units, unchanged, errors)
}

fn finish(
    root: &Path,
    files: Vec<std::path::PathBuf>,
    units: Vec<FileUnit>,
    unchanged: Vec<String>,
    errors: Vec<String>,
) -> Result<Collected> {
    let status = if files.is_empty() && unchanged.is_empty() { "missing" } else { "ok" };
    let records = units.iter().map(|unit| unit.records.len()).sum();
    Ok(Collected {
        report: SourceReport {
            source: SourceKind::Codex,
            label: SourceKind::Codex.label(),
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

    fn write(path: &Path, lines: &[&str]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, lines.join("\n")).unwrap();
    }

    fn temporary_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("vcs-codex-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn visible_user_and_assistant_text_is_kept_and_everything_else_is_dropped() {
        let root = temporary_root("visible");
        write(
            &root.join("sessions/2026/09/rollout-2026-09-01-abc.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"abc","cwd":"/tmp/project"}}"#,
                r#"{"type":"event_msg","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"user_message","message":"index the trigram store"}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:05Z","payload":{"type":"reasoning","content":[{"type":"text","text":"hidden thought"}]}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:06Z","payload":{"type":"function_call","name":"shell","arguments":"{}"}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:07Z","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"index rebuilt"}]}}"#,
            ],
        );
        let collected = crate::collect(crate::SourceKind::Codex, &root, None, 12_000).unwrap();
        let records = collected.records();
        let texts: Vec<&str> = records.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["index the trigram store", "index rebuilt"]);
        assert_eq!(records[0].session_id, "abc");
        assert_eq!(records[0].cwd.as_deref(), Some("/tmp/project"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_whole_message_of_machine_context_is_not_visible_conversation() {
        for injected in [
            "<environment_context>\n  <cwd>/tmp</cwd>\n</environment_context>",
            "<recommended_plugins>\nsome list\n</recommended_plugins>",
            "# AGENTS.md instructions for /tmp/project\n\nrules here",
        ] {
            assert!(is_machine_context(injected), "{injected}");
        }
        assert!(is_machine_context(
            "<recommended_plugins>list</recommended_plugins>\n<environment_context>cwd</environment_context>"
        ));
        for typed in [
            "please fix the parser",
            "compare <a> and <b> in the diff",
            "<not closed properly",
            "<environment_context>cwd</environment_context> and then do the work",
        ] {
            assert!(!is_machine_context(typed), "{typed}");
        }
    }

    #[test]
    fn an_agents_dump_after_a_context_element_is_still_machine_context() {
        // The shape that survived a full rebuild on 2026-09-02: 101 Codex records whose first block
        // was `<recommended_plugins>` and whose second was the AGENTS.md dump.
        let message = "<recommended_plugins>\nlist\n</recommended_plugins>\n# AGENTS.md instructions for /tmp/p\n\n<INSTRUCTIONS>\n# CLAUDE.md\nrules\n</INSTRUCTIONS>";
        assert!(is_machine_context(message));
        assert_eq!(
            strip_leading_context("# AGENTS.md instructions for /tmp/p\n\n<INSTRUCTIONS>\nrules\n</INSTRUCTIONS>\nplease fix the parser"),
            "please fix the parser"
        );
    }

    #[test]
    fn a_context_element_before_the_typed_words_is_removed() {
        assert_eq!(
            strip_leading_context("<image name=\"a\" path=\"/tmp/a.png\"></image>\n[Image #1] what is this?"),
            "[Image #1] what is this?"
        );
        assert_eq!(strip_leading_context("plain words"), "plain words");
        assert_eq!(
            strip_leading_context("<only>context</only>"),
            "<only>context</only>",
            "a message that is nothing but context is left for is_machine_context to reject"
        );
    }

    #[test]
    fn a_context_block_appended_to_a_typed_prompt_is_removed() {
        let text = "pitch me with all templates in an html\n<loom_context version=\"1\">\nbody\n</loom_context>";
        assert_eq!(strip_trailing_context(text), "pitch me with all templates in an html");
        assert_eq!(strip_trailing_context("just words"), "just words");
    }

    #[test]
    fn injected_context_never_reaches_the_index() {
        let root = temporary_root("injected");
        write(
            &root.join("sessions/rollout-injected.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"inj","cwd":"/tmp/project"}}"#,
                r##"{"type":"response_item","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for /tmp/project"},{"type":"input_text","text":"<environment_context>\n  <cwd>/tmp</cwd>\n</environment_context>"}]}}"##,
                r##"{"type":"response_item","timestamp":"2026-09-01T10:01:00Z","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"index the store\n<loom_context version=\"1\">\ncontext\n</loom_context>"}]}}"##,
            ],
        );
        let collected = crate::collect(crate::SourceKind::Codex, &root, None, 12_000).unwrap();
        let records = collected.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].text, "index the store");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_recent_rollout_stores_the_typed_prompt_as_a_user_response_item() {
        let root = temporary_root("input-text");
        write(
            &root.join("sessions/rollout-recent.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"recent","cwd":"/tmp/project"}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"explain the index build"}]}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:02Z","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"injected instructions"}]}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:05Z","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"it reads each store once"}]}}"#,
            ],
        );
        let collected = crate::collect(crate::SourceKind::Codex, &root, None, 12_000).unwrap();
        let records = collected.records();
        let texts: Vec<&str> = records.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["explain the index build", "it reads each store once"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_same_prompt_in_both_shapes_is_stored_once() {
        let root = temporary_root("both-shapes");
        write(
            &root.join("sessions/rollout-both.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"both","cwd":"/tmp/project"}}"#,
                r#"{"type":"event_msg","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"user_message","message":"one prompt"}}"#,
                r#"{"type":"response_item","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"one prompt"}]}}"#,
            ],
        );
        // Both shapes carry the same prompt with slightly different timestamps in real rollouts, so
        // the record identity differs and only the source-level deduplication collapses them. That
        // step lives in the public adapter entry point.
        let collected =
            crate::collect(crate::SourceKind::Codex, &root, None, 12_000).unwrap();
        assert_eq!(collected.records().len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_subthread_rollout_is_skipped_because_the_parent_already_has_it() {
        let root = temporary_root("subthread");
        write(
            &root.join("sessions/rollout-child.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"child","parent_thread_id":"parent"}}"#,
                r#"{"type":"event_msg","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"user_message","message":"duplicate prompt"}}"#,
            ],
        );
        assert!(crate::collect(crate::SourceKind::Codex, &root, None, 12_000).unwrap().records().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_malformed_line_is_reported_and_the_rest_of_the_file_still_loads() {
        let root = temporary_root("malformed");
        write(
            &root.join("sessions/rollout-x.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"x"}}"#,
                r#"{"type":"event_msg","timestamp":"#,
                r#"{"type":"event_msg","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"user_message","message":"still readable"}}"#,
            ],
        );
        let collected = crate::collect(crate::SourceKind::Codex, &root, None, 12_000).unwrap();
        assert_eq!(collected.records().len(), 1);
        assert_eq!(collected.report.errors.len(), 1);
        assert_eq!(collected.report.status, "ok");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_unit_the_filter_rejects_is_reported_unchanged_and_never_parsed() {
        let root = temporary_root("filter");
        write(
            &root.join("sessions/rollout-skip.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"skip"}}"#,
                r#"{"type":"event_msg","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"user_message","message":"already indexed"}}"#,
            ],
        );
        let mut seen: Vec<String> = Vec::new();
        let collected = crate::collect_with(
            crate::SourceKind::Codex,
            &root,
            None,
            12_000,
            &mut |scope, fingerprint| {
                seen.push(format!("{scope}|{fingerprint}"));
                false
            },
        )
        .unwrap();
        assert!(collected.units.is_empty());
        assert_eq!(collected.unchanged.len(), 1);
        assert_eq!(collected.live_scopes().len(), 1);
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("rollout-skip.jsonl|"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_store_reports_missing_instead_of_failing() {
        let report = crate::collect(crate::SourceKind::Codex, &temporary_root("absent"), None, 12_000).unwrap().report;
        assert_eq!(report.status, "missing");
        assert_eq!(report.records, 0);
    }

    #[test]
    fn the_day_window_excludes_older_records() {
        let root = temporary_root("window");
        write(
            &root.join("sessions/rollout-y.jsonl"),
            &[
                r#"{"type":"session_meta","payload":{"id":"y"}}"#,
                r#"{"type":"event_msg","timestamp":"2020-01-01T00:00:00Z","payload":{"type":"user_message","message":"ancient prompt"}}"#,
                r#"{"type":"event_msg","timestamp":"2026-09-01T00:00:00Z","payload":{"type":"user_message","message":"recent prompt"}}"#,
            ],
        );
        let cutoff = Some(1_756_000_000.0);
        let collected = crate::collect(crate::SourceKind::Codex, &root, cutoff, 12_000).unwrap();
        let records = collected.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].text, "recent prompt");
        std::fs::remove_dir_all(&root).unwrap();
    }
}

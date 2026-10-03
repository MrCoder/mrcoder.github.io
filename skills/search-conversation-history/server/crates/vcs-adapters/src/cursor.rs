//! Cursor history: composer transcripts inside the workspace state database.
//!
//! The database is opened read-only. Cursor's own process may hold it open, so a busy or locked
//! store is reported as an error for that source and never fails the whole read.

use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use vcs_core::time::{epoch_seconds, format_epoch, in_window};
use vcs_core::{Record, Role};

use crate::{Collected, FileUnit, SourceKind, SourceReport, UnitFilter};

fn table_names(connection: &Connection) -> rusqlite::Result<std::collections::HashSet<String>> {
    let mut statement = connection.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

fn report(root: &Path, status: &str, files_read: usize, errors: Vec<String>) -> SourceReport {
    SourceReport {
        source: SourceKind::Cursor,
        label: SourceKind::Cursor.label(),
        root: root.display().to_string(),
        status: status.into(),
        files_read,
        records: 0,
        errors,
    }
}

fn empty(root: &Path, status: &str, files_read: usize, errors: Vec<String>) -> Collected {
    Collected {
        units: Vec::new(),
        unchanged: Vec::new(),
        report: report(root, status, files_read, errors),
    }
}

pub fn collect(
    root: &Path,
    agent_root: &Path,
    cutoff: Option<f64>,
    max_chars: usize,
    filter: UnitFilter<'_>,
) -> Result<Collected> {
    // Cursor keeps two stores. The workspace database holds composer transcripts; newer Agent Host
    // sessions each have their own blob store under the user's `.cursor` directory.
    let (agent_units, agent_unchanged, agent_stores, agent_errors) =
        crate::cursor_agent::collect(agent_root, cutoff, max_chars, filter);

    let database = root.join("User/globalStorage/state.vscdb");
    if !database.exists() {
        if agent_stores == 0 {
            return Ok(empty(root, "missing", 0, agent_errors));
        }
        let mut collected = empty(root, "ok", agent_stores, agent_errors);
        collected.report.records = agent_units.iter().map(|unit| unit.records.len()).sum();
        collected.units = agent_units;
        collected.unchanged = agent_unchanged;
        return Ok(collected);
    }
    let connection = match Connection::open_with_flags(
        &database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    ) {
        Ok(connection) => connection,
        Err(error) => return Ok(empty(root, "error", 1, vec![error.to_string()])),
    };
    let tables = match table_names(&connection) {
        Ok(tables) => tables,
        Err(error) => return Ok(empty(root, "error", 1, vec![error.to_string()])),
    };
    let mut errors: Vec<String> = Vec::new();
    for required in ["composerHeaders", "cursorDiskKV"] {
        if !tables.contains(required) {
            return Ok(empty(
                root,
                "unrecognized_schema",
                1,
                vec![format!("missing table: {required}")],
            ));
        }
    }

    let mut units: Vec<FileUnit> = agent_units;
    let mut unchanged: Vec<String> = agent_unchanged;
    errors.extend(agent_errors);
    let headers: Vec<(String, serde_json::Value, serde_json::Value, Option<String>)> = {
        let mut statement = connection
            .prepare("SELECT composerId, createdAt, lastUpdatedAt, value FROM composerHeaders")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<f64>>(1)?.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
                row.get::<_, Option<f64>>(2)?.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    for (composer_id, created_at, updated_at, header_value) in headers {
        if cutoff.is_some() && !in_window(&updated_at, cutoff) && !in_window(&created_at, cutoff) {
            continue;
        }
        // One conversation is one unit. Cursor keeps every transcript in a single database file, so a
        // file fingerprint would invalidate all of them on any edit; the conversation's own last-update
        // time is the honest granularity.
        let scope = format!("cursor:{composer_id}");
        let fingerprint = format!(
            "{}",
            updated_at.as_f64().or_else(|| created_at.as_f64()).unwrap_or(0.0)
        );
        if !filter(&scope, &fingerprint) {
            unchanged.push(scope);
            continue;
        }
        let mut records: Vec<Record> = Vec::new();
        let mut cwd: Option<String> = None;
        if let Some(raw) = header_value.as_deref() {
            match serde_json::from_str::<serde_json::Value>(raw) {
                Ok(header) => {
                    cwd = header
                        .get("workspaceIdentifier")
                        .and_then(|value| value.get("uri"))
                        .and_then(|value| value.get("fsPath"))
                        .and_then(|value| value.as_str())
                        .map(str::to_string);
                }
                Err(_) => errors.push(format!("composer {composer_id}: malformed header JSON")),
            }
        }

        let mut bubble_ids: Vec<String> = Vec::new();
        let composer_row: Option<String> = connection
            .query_row(
                "SELECT value FROM cursorDiskKV WHERE key = ?",
                [format!("composerData:{composer_id}")],
                |row| row.get(0),
            )
            .ok();
        if let Some(raw) = composer_row {
            match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(composer) => {
                    if let Some(list) = composer
                        .get("fullConversationHeadersOnly")
                        .and_then(|value| value.as_array())
                    {
                        bubble_ids = list
                            .iter()
                            .filter_map(|item| item.get("bubbleId").and_then(|v| v.as_str()))
                            .map(str::to_string)
                            .collect();
                    }
                }
                Err(_) => errors.push(format!("composer {composer_id}: malformed composer JSON")),
            }
        }
        if bubble_ids.is_empty() {
            let mut statement =
                connection.prepare("SELECT key FROM cursorDiskKV WHERE key LIKE ?")?;
            let rows = statement.query_map([format!("bubbleId:{composer_id}:%")], |row| {
                row.get::<_, String>(0)
            })?;
            for key in rows.collect::<rusqlite::Result<Vec<String>>>()? {
                if let Some(id) = key.rsplit(':').next() {
                    bubble_ids.push(id.to_string());
                }
            }
        }

        for bubble_id in bubble_ids {
            let raw: Option<String> = connection
                .query_row(
                    "SELECT value FROM cursorDiskKV WHERE key = ?",
                    [format!("bubbleId:{composer_id}:{bubble_id}")],
                    |row| row.get(0),
                )
                .ok();
            let Some(raw) = raw else {
                errors.push(format!("composer {composer_id}: missing bubble {bubble_id}"));
                continue;
            };
            let bubble: serde_json::Value = match serde_json::from_str(&raw) {
                Ok(value) => value,
                Err(_) => {
                    errors.push(format!("composer {composer_id}: malformed bubble JSON"));
                    continue;
                }
            };
            let role = match bubble.get("type").and_then(|v| v.as_i64()) {
                Some(1) => Role::User,
                Some(2) => Role::Assistant,
                _ => continue,
            };
            // Cursor renders neither of these to the person.
            if bubble.get("skipRendering").and_then(|v| v.as_bool()) == Some(true)
                || bubble.get("isDisplayOnly").and_then(|v| v.as_bool()) == Some(true)
            {
                continue;
            }
            let timestamp_value = bubble
                .get("createdAt")
                .cloned()
                .filter(|value| !value.is_null())
                .unwrap_or_else(|| created_at.clone());
            if cutoff.is_some()
                && !timestamp_value.is_null()
                && !in_window(&timestamp_value, cutoff)
            {
                continue;
            }
            let timestamp = epoch_seconds(&timestamp_value).map(format_epoch);
            let text = bubble.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(record) = Record::new(
                "cursor",
                &composer_id,
                timestamp,
                role,
                cwd.clone(),
                text,
                max_chars,
            ) {
                records.push(record);
            }
        }
        units.push(FileUnit { scope, fingerprint, records });
    }

    let mut source_report = report(root, "ok", 1 + agent_stores, errors);
    source_report.records = units.iter().map(|unit| unit.records.len()).sum();
    Ok(Collected { units, unchanged, report: source_report })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_store(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("vcs-cursor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let database = root.join("User/globalStorage/state.vscdb");
        std::fs::create_dir_all(database.parent().unwrap()).unwrap();
        let connection = Connection::open(&database).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE composerHeaders(composerId TEXT, createdAt REAL, lastUpdatedAt REAL, value TEXT);
                 CREATE TABLE cursorDiskKV(key TEXT, value TEXT);",
            )
            .unwrap();
        root
    }

    fn open(root: &Path) -> Connection {
        Connection::open(root.join("User/globalStorage/state.vscdb")).unwrap()
    }

    #[test]
    fn user_and_assistant_bubbles_become_records_with_the_workspace_path() {
        let root = build_store("bubbles");
        let connection = open(&root);
        connection.execute(
            "INSERT INTO composerHeaders VALUES ('c1', 1788267600000, 1788267600000, ?)",
            [r#"{"workspaceIdentifier":{"uri":{"fsPath":"/tmp/project"}}}"#],
        ).unwrap();
        connection.execute(
            "INSERT INTO cursorDiskKV VALUES ('composerData:c1', ?)",
            [r#"{"fullConversationHeadersOnly":[{"bubbleId":"b1"},{"bubbleId":"b2"}]}"#],
        ).unwrap();
        connection.execute(
            "INSERT INTO cursorDiskKV VALUES ('bubbleId:c1:b1', ?)",
            [r#"{"type":1,"text":"cursor question","createdAt":1788267600000}"#],
        ).unwrap();
        connection.execute(
            "INSERT INTO cursorDiskKV VALUES ('bubbleId:c1:b2', ?)",
            [r#"{"type":2,"text":"cursor answer","createdAt":1788267601000}"#],
        ).unwrap();
        drop(connection);

        let collected = crate::collect(crate::SourceKind::Cursor, &root, None, 12_000).unwrap();
        assert_eq!(collected.report.status, "ok");
        let records = collected.records();
        let texts: Vec<&str> = records.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["cursor question", "cursor answer"]);
        assert_eq!(records[0].cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(records[0].timestamp.as_deref(), Some("2026-09-01T13:00:00Z"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn bubbles_cursor_never_rendered_are_skipped() {
        let root = build_store("hidden");
        let connection = open(&root);
        connection
            .execute("INSERT INTO composerHeaders VALUES ('c1', 1788267600000, 1788267600000, NULL)", [])
            .unwrap();
        connection.execute(
            "INSERT INTO cursorDiskKV VALUES ('bubbleId:c1:b1', ?)",
            [r#"{"type":2,"text":"internal","skipRendering":true,"createdAt":1788267600000}"#],
        ).unwrap();
        drop(connection);
        assert!(crate::collect(crate::SourceKind::Cursor, &root, None, 12_000).unwrap().records().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_unknown_schema_is_reported_instead_of_guessed() {
        let root = std::env::temp_dir().join(format!("vcs-cursor-schema-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let database = root.join("User/globalStorage/state.vscdb");
        std::fs::create_dir_all(database.parent().unwrap()).unwrap();
        Connection::open(&database).unwrap().execute_batch("CREATE TABLE other(x)").unwrap();
        let collected = crate::collect(crate::SourceKind::Cursor, &root, None, 12_000).unwrap();
        assert_eq!(collected.report.status, "unrecognized_schema");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_store_is_missing_not_an_error() {
        let root = std::env::temp_dir().join("vcs-cursor-none-does-not-exist");
        assert_eq!(crate::collect(crate::SourceKind::Cursor, &root, None, 12_000).unwrap().report.status, "missing");
    }
}

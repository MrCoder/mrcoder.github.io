//! Source adapters. Each adapter reads one local history store and emits the shared record contract.
//!
//! Adapters are the only code that knows a source format. They open history read-only and never
//! write to a history store. Everything a source keeps outside the visible transcript — hidden
//! reasoning, tool calls, tool results, system prompts, source-specific metadata — is dropped here,
//! before the core ever sees it.

pub mod claude;
mod context;
pub mod codex;
pub mod cursor;
pub mod cursor_agent;

use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use vcs_core::Record;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Codex,
    Claude,
    Cursor,
}

impl SourceKind {
    pub const ALL: [SourceKind; 3] = [SourceKind::Codex, SourceKind::Claude, SourceKind::Cursor];

    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Codex => "codex",
            SourceKind::Claude => "claude",
            SourceKind::Cursor => "cursor",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SourceKind::Codex => "Codex",
            SourceKind::Claude => "Claude Code",
            SourceKind::Cursor => "Cursor",
        }
    }

    pub fn parse(value: &str) -> Option<SourceKind> {
        match value {
            "codex" => Some(SourceKind::Codex),
            "claude" => Some(SourceKind::Claude),
            "cursor" => Some(SourceKind::Cursor),
            _ => None,
        }
    }

    /// Default location of this source's local history store.
    pub fn default_root(self) -> PathBuf {
        let home = home_directory();
        match self {
            SourceKind::Codex => home.join(".codex"),
            SourceKind::Claude => home.join(".claude"),
            SourceKind::Cursor => home.join("Library/Application Support/Cursor"),
        }
    }

    /// The paths a native `rg` search would have to walk to answer the same question.
    pub fn raw_search_paths(self, root: &Path) -> Vec<PathBuf> {
        match self {
            SourceKind::Codex => vec![root.join("sessions"), root.join("archived_sessions")]
                .into_iter()
                .filter(|path| path.exists())
                .collect(),
            SourceKind::Claude => vec![root.join("projects")]
                .into_iter()
                .filter(|path| path.exists())
                .collect(),
            SourceKind::Cursor => vec![root.join("User/globalStorage/state.vscdb")]
                .into_iter()
                .filter(|path| path.exists())
                .collect(),
        }
    }
}

/// Where Cursor keeps its Agent Host sessions, given the workspace-state root.
///
/// The two stores live in different places: the workspace database sits under application support,
/// the agent stores under the user's `.cursor` directory. For any non-default root — a test, or a
/// copied profile — the agent stores are looked for inside that same root, so nothing reaches the
/// real home directory by accident.
pub fn cursor_agent_root(cursor_root: &Path) -> PathBuf {
    if cursor_root == SourceKind::Cursor.default_root() {
        home_directory().join(".cursor/chats")
    } else {
        cursor_root.join(".cursor/chats")
    }
}

pub fn home_directory() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// What one adapter found, including the failures it survived.
#[derive(Debug, Clone, Serialize)]
pub struct SourceReport {
    pub source: SourceKind,
    pub label: &'static str,
    pub root: String,
    /// `ok`, `missing`, `unrecognized_schema`, or `error`.
    pub status: String,
    pub files_read: usize,
    pub records: usize,
    pub errors: Vec<String>,
}

impl SourceReport {
    pub fn installed(&self) -> bool {
        self.status == "ok"
    }
}

/// One indexable unit of a source: a single history file, or one Cursor conversation.
///
/// A unit is the granularity of incremental work. Its scope is the index key, and its fingerprint
/// tells a later run whether the unit changed since the last index.
#[derive(Debug, Clone)]
pub struct FileUnit {
    pub scope: String,
    pub fingerprint: String,
    pub records: Vec<Record>,
}

pub struct Collected {
    /// Units that were read this time.
    pub units: Vec<FileUnit>,
    /// Scopes skipped because their fingerprint still matched. They stay in the index untouched.
    pub unchanged: Vec<String>,
    pub report: SourceReport,
}

impl Collected {
    /// Every record read this time, in unit order.
    pub fn records(&self) -> Vec<Record> {
        self.units.iter().flat_map(|unit| unit.records.iter().cloned()).collect()
    }

    /// Scopes this source still owns, whether read or skipped. Anything else in the index is stale.
    pub fn live_scopes(&self) -> Vec<String> {
        let mut scopes: Vec<String> =
            self.units.iter().map(|unit| unit.scope.clone()).collect();
        scopes.extend(self.unchanged.iter().cloned());
        scopes
    }
}

/// Decides whether a unit must be read. `false` means the stored copy is still current.
pub type UnitFilter<'a> = &'a mut dyn FnMut(&str, &str) -> bool;

/// Read one source. A malformed file is reported and skipped; it never aborts the whole read.
///
/// `filter` receives each unit's scope and fingerprint before the unit is parsed, so an unchanged
/// file costs one metadata call instead of a full read.
pub fn collect_with(
    source: SourceKind,
    root: &Path,
    cutoff: Option<f64>,
    max_chars: usize,
    filter: UnitFilter<'_>,
) -> Result<Collected> {
    let mut collected = match source {
        SourceKind::Codex => codex::collect(root, cutoff, max_chars, filter),
        SourceKind::Claude => claude::collect(root, cutoff, max_chars, filter),
        SourceKind::Cursor => {
            cursor::collect(root, &cursor_agent_root(root), cutoff, max_chars, filter)
        }
    }?;
    // Deduplication stays inside a unit: the same prompt written twice in one file is one record,
    // and units must remain independently syncable.
    for unit in &mut collected.units {
        unit.records = vcs_core::deduplicate(std::mem::take(&mut unit.records));
    }
    collected.report.records = collected.units.iter().map(|unit| unit.records.len()).sum();
    Ok(collected)
}

/// Read every unit, ignoring any stored state.
pub fn collect(source: SourceKind, root: &Path, cutoff: Option<f64>, max_chars: usize) -> Result<Collected> {
    collect_with(source, root, cutoff, max_chars, &mut |_, _| true)
}

/// Fingerprint of a file: modification time and size. Cheap, and enough to notice an append.
pub(crate) fn file_fingerprint(path: &Path) -> String {
    match path.metadata() {
        Ok(metadata) => {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|value| value.as_millis())
                .unwrap_or(0);
            format!("{modified}-{}", metadata.len())
        }
        Err(_) => "unknown".to_string(),
    }
}

/// Cheap presence check for the welcome screen.
///
/// Normalizing every store to count records takes tens of seconds on a large history, which is far
/// too slow for a screen that must appear immediately. A probe only asks whether the store exists and
/// how many history files it holds; the exact record count arrives during the index build.
#[derive(Debug, Clone, Serialize)]
pub struct SourceProbe {
    pub source: SourceKind,
    pub label: &'static str,
    pub root: String,
    pub installed: bool,
    pub history_files: usize,
    pub bytes: u64,
}

pub fn probe_source(source: SourceKind, root: &Path) -> SourceProbe {
    let (history_files, bytes) = match source {
        SourceKind::Codex => {
            let mut files = jsonl_files(&root.join("sessions"));
            files.extend(jsonl_files(&root.join("archived_sessions")));
            let history = root.join("history.jsonl");
            if history.exists() {
                files.push(history);
            }
            let bytes = files.iter().filter_map(|path| path.metadata().ok()).map(|meta| meta.len()).sum();
            (files.len(), bytes)
        }
        SourceKind::Claude => {
            let files = jsonl_files(&root.join("projects"));
            let bytes = files.iter().filter_map(|path| path.metadata().ok()).map(|meta| meta.len()).sum();
            (files.len(), bytes)
        }
        SourceKind::Cursor => {
            let database = root.join("User/globalStorage/state.vscdb");
            match database.metadata() {
                Ok(metadata) => (1, metadata.len()),
                Err(_) => (0, 0),
            }
        }
    };
    SourceProbe {
        source,
        label: source.label(),
        root: root.display().to_string(),
        installed: history_files > 0,
        history_files,
        bytes,
    }
}

/// Probe every source. Fast enough to run before the first screen paints.
pub fn probe() -> Vec<SourceProbe> {
    SourceKind::ALL
        .iter()
        .map(|source| probe_source(*source, &source.default_root()))
        .collect()
}

/// Report every source without indexing anything, for the welcome screen.
pub fn detect(cutoff: Option<f64>, max_chars: usize) -> Vec<SourceReport> {
    SourceKind::ALL
        .iter()
        .map(|source| {
            let root = source.default_root();
            match collect(*source, &root, cutoff, max_chars) {
                Ok(collected) => collected.report,
                Err(error) => SourceReport {
                    source: *source,
                    label: source.label(),
                    root: root.display().to_string(),
                    status: "error".into(),
                    files_read: 0,
                    records: 0,
                    errors: vec![error.to_string()],
                },
            }
        })
        .collect()
}

/// Read a JSONL file, returning one value per parsable line and one error per malformed line.
pub(crate) fn read_json_lines(path: &Path, errors: &mut Vec<String>) -> Vec<serde_json::Value> {
    let content = match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            return Vec::new();
        }
    };
    let mut values = Vec::new();
    for (number, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => {
                if value.is_object() {
                    values.push(value)
                }
            }
            Err(_) => errors.push(format!("{}:{}: malformed JSON", path.display(), number + 1)),
        }
    }
    values
}

/// Flatten a message `content` field into visible text, keeping only the allowed block types.
pub(crate) fn text_from_blocks(content: Option<&serde_json::Value>, allowed: &[&str]) -> String {
    match content {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Array(blocks)) => {
            let mut parts: Vec<&str> = Vec::new();
            for block in blocks {
                let Some(object) = block.as_object() else { continue };
                let block_type = object.get("type").and_then(|value| value.as_str()).unwrap_or("");
                if !allowed.contains(&block_type) {
                    continue;
                }
                if let Some(text) = object.get("text").and_then(|value| value.as_str()) {
                    if !text.trim().is_empty() {
                        parts.push(text);
                    }
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

/// Collect `**/*.jsonl` under `directory`, sorted, so a run is reproducible.
pub(crate) fn jsonl_files(directory: &Path) -> Vec<PathBuf> {
    if !directory.exists() {
        return Vec::new();
    }
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("jsonl"))
        .collect();
    files.sort();
    files
}

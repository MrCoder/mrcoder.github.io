//! Side-by-side measurement of the native search path and the local visible-conversation index.
//!
//! The comparison is honest about what each side reads. `rg` walks raw history files, which contain
//! tool calls, tool output, and metadata as well as the visible transcript; the index holds only
//! visible user and assistant text. Match counts therefore differ by design, and the result carries
//! that statement with it.

use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

use crate::App;

#[derive(Debug, Clone, Serialize)]
pub struct RawSearch {
    pub command: String,
    pub paths: Vec<String>,
    pub elapsed_ms: f64,
    pub matching_lines: u64,
    pub available: bool,
    pub note: Option<String>,
}

/// One raw line that contains the phrase, shown so the difference between the two match counts is
/// inspectable: most of these lines are tool output or metadata that was never a visible message.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RawSample {
    pub path: String,
    /// The JSON field the phrase sits in, when the surrounding text shows one (`output`, `content`…).
    pub field: Option<String>,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexedSearch {
    pub elapsed_ms: f64,
    pub hits: usize,
    pub conversations: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub query: String,
    pub raw: RawSearch,
    pub indexed: IndexedSearch,
    /// Raw elapsed time divided by indexed elapsed time. `None` when `rg` is unavailable.
    pub speedup: Option<f64>,
    pub scope_note: String,
}

pub const SCOPE_NOTE: &str = "The two sides do not read the same bytes. The native path scans raw \
history files, which also hold tool calls, tool output, and metadata. The index holds only the user \
and assistant messages that were visible in the original interface, so its match count is smaller by \
design. Raw evidence stays available through the native path when a question needs it.";

/// Locations a native search would have to walk to answer the same question.
pub fn raw_paths() -> Vec<PathBuf> {
    vcs_adapters::SourceKind::ALL
        .iter()
        // A Cursor transcript lives inside a SQLite file, which a line-oriented tool cannot read as
        // text, so the native side compares only the two file-based stores.
        .filter(|source| **source != vcs_adapters::SourceKind::Cursor)
        .flat_map(|source| source.raw_search_paths(&source.default_root()))
        .collect()
}

/// Run only the native side. The interface calls this and the indexed search at the same moment, so
/// each lane shows its own real elapsed time instead of a replay of a server-side sequence.
pub fn raw_only(query: &str) -> RawSearch {
    run_raw(query)
}

/// Run both searches once and report the measured times.
pub fn run(app: &App, query: &str, limit: usize) -> Result<BenchResult> {
    let indexed_response = app.search(query, limit)?;
    let indexed = IndexedSearch {
        elapsed_ms: indexed_response.elapsed_ms,
        hits: indexed_response.hits.len(),
        conversations: indexed_response.conversations,
    };
    let raw = run_raw(query);
    let speedup = if raw.available && indexed.elapsed_ms > 0.0 {
        Some(raw.elapsed_ms / indexed.elapsed_ms)
    } else {
        None
    };
    Ok(BenchResult {
        query: query.to_string(),
        raw,
        indexed,
        speedup,
        scope_note: SCOPE_NOTE.into(),
    })
}

/// Up to `limit` raw lines that contain the phrase, one per file, with a short window of the
/// surrounding bytes. This is a second, untimed `rg` run: the timed comparison stays a pure count.
pub fn raw_samples(query: &str, limit: usize) -> Vec<RawSample> {
    if query.trim().is_empty() || limit == 0 {
        return Vec::new();
    }
    let pattern = format!(".{{0,90}}{}.{{0,90}}", regex_escape(query));
    let mut command = Command::new("rg");
    command
        .arg("--ignore-case")
        .arg("--only-matching")
        .arg("--with-filename")
        .arg("--no-line-number")
        .arg("--max-count")
        .arg("1")
        .arg("--no-messages")
        .arg("--no-config")
        .arg("--hidden")
        .arg("--no-ignore")
        .arg("-e")
        .arg(&pattern);
    for path in raw_paths() {
        command.arg(path);
    }
    let Ok(output) = command.output() else { return Vec::new() };
    let mut samples: Vec<RawSample> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((path, excerpt)) = line.split_once(':') else { continue };
        let excerpt = excerpt.trim();
        if excerpt.is_empty() || samples.iter().any(|sample| sample.excerpt == excerpt) {
            continue;
        }
        samples.push(RawSample {
            path: path.to_string(),
            field: json_field_before(excerpt, query),
            excerpt: excerpt.to_string(),
        });
        if samples.len() >= limit {
            break;
        }
    }
    samples
}

/// The last `"key":` that precedes the phrase inside the excerpt, when the excerpt is JSON text.
fn json_field_before(excerpt: &str, query: &str) -> Option<String> {
    let at = find_case_insensitive(excerpt, query)?;
    let head = &excerpt[..at];
    let close = head.rfind("\":")?;
    let open = head[..close].rfind('"')?;
    let key = &head[open + 1..close];
    let clean = key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    (clean && !key.is_empty()).then(|| key.to_string())
}

/// Byte offset in `text` where `needle` starts, compared case-insensitively. Lowercasing can change
/// a character's byte length (U+0130 İ becomes two code points), so an offset found in the lowered
/// text is mapped back through a per-character table instead of being used on the original.
fn find_case_insensitive(text: &str, needle: &str) -> Option<usize> {
    let needle = needle.to_lowercase();
    let mut lowered = String::with_capacity(text.len());
    let mut origin: Vec<usize> = Vec::with_capacity(text.len() + 1);
    for (index, character) in text.char_indices() {
        for lower in character.to_lowercase() {
            let before = lowered.len();
            lowered.push(lower);
            origin.extend(std::iter::repeat(index).take(lowered.len() - before));
        }
    }
    origin.push(text.len());
    let at = lowered.find(&needle)?;
    origin.get(at).copied()
}

/// Escape a phrase for the Rust regex syntax `rg` uses. Only ASCII punctuation carries meaning.
fn regex_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if c.is_ascii() && !c.is_ascii_alphanumeric() && c != ' ' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

fn run_raw(query: &str) -> RawSearch {
    let paths = raw_paths();
    // `--hidden --no-ignore` are required for a fair baseline. Without them `rg` silently skips
    // hidden directories and anything an ignore file excludes: measured on this computer, the
    // default flags searched 1,501 files and 3.88 GB while the full set is 11,882 files and 7.39 GB.
    // A baseline that reads half the history would understate its own time and miss records the
    // index holds.
    let mut command = Command::new("rg");
    command
        .arg("--fixed-strings")
        .arg("--ignore-case")
        .arg("--count-matches")
        .arg("--no-messages")
        .arg("--no-config")
        .arg("--hidden")
        .arg("--no-ignore")
        .arg(query);
    for path in &paths {
        command.arg(path);
    }
    let printable = format!(
        "rg --fixed-strings --ignore-case --count-matches --no-messages --hidden --no-ignore {:?} {}",
        query,
        paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(" ")
    );

    let started = std::time::Instant::now();
    let output = command.output();
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    match output {
        Ok(output) => {
            let mut matching_lines = 0u64;
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Some((_, count)) = line.rsplit_once(':') {
                    matching_lines += count.trim().parse::<u64>().unwrap_or(0);
                }
            }
            RawSearch {
                command: printable,
                paths: paths.iter().map(|path| path.display().to_string()).collect(),
                elapsed_ms,
                matching_lines,
                available: true,
                note: None,
            }
        }
        Err(error) => RawSearch {
            command: printable,
            paths: paths.iter().map(|path| path.display().to_string()).collect(),
            elapsed_ms: 0.0,
            matching_lines: 0,
            available: false,
            note: Some(format!("rg is not available on this computer: {error}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scope_note_refuses_to_claim_raw_parity() {
        assert!(SCOPE_NOTE.contains("do not read the same bytes"));
        assert!(SCOPE_NOTE.contains("Raw evidence stays available"));
    }

    #[test]
    fn the_json_field_before_the_phrase_is_the_nearest_key() {
        let excerpt = r#"call_id":"c1","output":"ran the forge-feature-flag skill and"#;
        assert_eq!(json_field_before(excerpt, "forge-feature-flag skill"), Some("output".into()));
        assert_eq!(json_field_before("plain text with the phrase", "phrase"), None);
    }

    #[test]
    fn a_length_changing_lowercase_before_the_phrase_does_not_break_the_slice() {
        // U+0130 lowercases to two code points, so the lowered offset is one byte past the original.
        let excerpt = "\"output\":\"İÉlite team\"";
        assert_eq!(json_field_before(excerpt, "élite"), Some("output".into()));
        assert_eq!(find_case_insensitive("İstanbul Élite", "élite"), Some("İstanbul ".len()));
    }

    #[test]
    fn regex_escaping_keeps_letters_and_spaces_and_escapes_punctuation() {
        assert_eq!(regex_escape("a.b c"), "a\\.b c");
        assert_eq!(regex_escape("这跟Full"), "这跟Full");
    }

    #[test]
    fn the_native_side_skips_the_sqlite_backed_source() {
        let cursor_root = vcs_adapters::SourceKind::Cursor.default_root();
        assert!(!raw_paths().iter().any(|path| path.starts_with(&cursor_root)));
    }
}

//! Portable, local-only core for visible AI conversation records.
//!
//! The core accepts normalized records from any adapter. It deliberately knows nothing about Codex,
//! Claude Code, Cursor, tool output, or hidden reasoning. Only `user` and `assistant` text that was
//! visible in the original product UI enters the index.

pub mod index;
pub mod time;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Roles that were visible in the original product interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            _ => None,
        }
    }
}

/// One visible conversation record. This is the entire contract between adapters and the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub source: String,
    pub session_id: String,
    pub timestamp: Option<String>,
    pub role: Role,
    pub cwd: Option<String>,
    pub text: String,
    #[serde(default)]
    pub truncated: bool,
}

/// Default per-record text ceiling, matching the reference extractor.
pub const DEFAULT_MAX_CHARS: usize = 12_000;

/// Trim trailing whitespace per line, normalize CRLF, and strip surrounding blank space.
pub fn compact_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n");
    let joined: Vec<&str> = normalized.lines().map(|line| line.trim_end()).collect();
    joined.join("\n").trim().to_string()
}

impl Record {
    /// Build a record from raw adapter output. Returns `None` when the visible text is empty.
    pub fn new(
        source: &str,
        session_id: &str,
        timestamp: Option<String>,
        role: Role,
        cwd: Option<String>,
        text: &str,
        max_chars: usize,
    ) -> Option<Record> {
        let cleaned = compact_text(text);
        if cleaned.is_empty() {
            return None;
        }
        let char_count = cleaned.chars().count();
        let truncated = char_count > max_chars;
        let body = if truncated {
            cleaned.chars().take(max_chars).collect()
        } else {
            cleaned
        };
        Some(Record {
            source: source.to_string(),
            session_id: session_id.to_string(),
            timestamp,
            role,
            cwd,
            text: body,
            truncated,
        })
    }

    /// Stable identity of a record.
    ///
    /// The digest is byte-compatible with the reference Python core so both implementations can
    /// share one index file: `sha256` over `json.dumps(identity, sort_keys=True, ensure_ascii=False)`
    /// where `identity` holds role, session_id, source, text, and timestamp.
    pub fn record_id(&self) -> String {
        let canonical = format!(
            "{{\"role\": {}, \"session_id\": {}, \"source\": {}, \"text\": {}, \"timestamp\": {}}}",
            json_string(self.role.as_str()),
            json_string(&self.session_id),
            json_string(&self.source),
            json_string(&self.text),
            match &self.timestamp {
                Some(value) => json_string(value),
                None => "null".to_string(),
            }
        );
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Record {
    /// Identity of the exchange itself, ignoring when it was written down.
    ///
    /// A resumed session replays earlier messages into a new file with new timestamps. Those are the
    /// same visible exchange and must appear once.
    pub fn dedupe_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(
            format!(
                "{}\u{1}{}\u{1}{}\u{1}{}",
                self.source,
                self.session_id,
                self.role.as_str(),
                self.text
            )
            .as_bytes(),
        );
        format!("{:x}", hasher.finalize())
    }
}

/// Serialize one string exactly as `json.dumps(value, ensure_ascii=False)` does.
fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization cannot fail")
}

/// Drop records that repeat an earlier `(source, session_id, role, text)`, preferring the copy that
/// carries a working directory. Ordering matches the reference extractor.
pub fn deduplicate(mut records: Vec<Record>) -> Vec<Record> {
    records.sort_by(|left, right| {
        let left_key = (
            left.timestamp.clone().unwrap_or_default(),
            left.session_id.clone(),
            left.role.as_str(),
        );
        let right_key = (
            right.timestamp.clone().unwrap_or_default(),
            right.session_id.clone(),
            right.role.as_str(),
        );
        left_key.cmp(&right_key)
    });
    let mut positions: std::collections::HashMap<(String, String, &'static str, String), usize> =
        std::collections::HashMap::new();
    let mut result: Vec<Record> = Vec::with_capacity(records.len());
    for record in records {
        let key = (
            record.source.clone(),
            record.session_id.clone(),
            record.role.as_str(),
            record.text.clone(),
        );
        if let Some(position) = positions.get(&key) {
            if result[*position].cwd.is_none() && record.cwd.is_some() {
                result[*position] = record;
            }
            continue;
        }
        positions.insert(key, result.len());
        result.push(record);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Record {
        Record {
            source: "codex".into(),
            session_id: "session-1".into(),
            timestamp: Some("2026-09-01T10:00:00Z".into()),
            role: Role::User,
            cwd: Some("/tmp".into()),
            text: "hello world".into(),
            truncated: false,
        }
    }

    #[test]
    fn record_id_matches_the_reference_python_core() {
        // Vector produced by the reference implementation:
        //   python3 -c "import hashlib,json; print(hashlib.sha256(json.dumps(
        //     {'role':'user','session_id':'session-1','source':'codex',
        //      'text':'hello world','timestamp':'2026-09-01T10:00:00Z'},
        //     sort_keys=True, ensure_ascii=False).encode()).hexdigest())"
        assert_eq!(
            sample().record_id(),
            "8f9a905b66610253cd9cda6c499231935e5837c67d08ab6895f511384b40d593"
        );
    }

    #[test]
    fn record_id_ignores_cwd_and_truncation() {
        let mut other = sample();
        other.cwd = None;
        other.truncated = true;
        assert_eq!(sample().record_id(), other.record_id());
    }

    #[test]
    fn empty_visible_text_produces_no_record() {
        assert!(Record::new("codex", "s", None, Role::User, None, "   \n  ", DEFAULT_MAX_CHARS).is_none());
    }

    #[test]
    fn long_text_is_truncated_and_flagged() {
        let long = "x".repeat(20);
        let record = Record::new("codex", "s", None, Role::Assistant, None, &long, 10).unwrap();
        assert_eq!(record.text.chars().count(), 10);
        assert!(record.truncated);
    }

    #[test]
    fn deduplicate_prefers_the_copy_with_a_working_directory() {
        let mut without = sample();
        without.cwd = None;
        let records = deduplicate(vec![without, sample()]);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cwd.as_deref(), Some("/tmp"));
    }
}

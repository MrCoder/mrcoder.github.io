//! Cursor Agent Host sessions: `~/.cursor/chats/<workspace>/<agent>/store.db`.
//!
//! Newer Cursor agent sessions are not in the workspace state database. Each one is its own SQLite
//! file holding a content-addressed blob store: `meta` points at a root blob, and the blobs are
//! protobuf messages that reference each other by SHA-256 digest. Only the user prompt and the
//! assistant's visible steps are read.
//!
//! One caveat the reader cannot remove: an Agent Host user payload holds the prompt as it was sent,
//! which for an automated agent session includes context the person never typed. The store keeps no
//! separate copy of what the panel displayed, so the payload is indexed as it stands rather than
//! guessed at.

use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use vcs_core::time::{epoch_seconds, format_epoch, in_window};
use vcs_core::{Record, Role};

use crate::{file_fingerprint, FileUnit, UnitFilter};

/// Field numbers in the Agent Host protobuf messages, named for what they carry.
const ROOT_TURNS: u32 = 8;
const ROOT_TIMINGS: u32 = 14;
const TIMING_EPOCH_MS: u32 = 2;
const TURN_AGENT: u32 = 1;
const AGENT_USER: u32 = 1;
const AGENT_STEPS: u32 = 2;
const PAYLOAD_TEXT: u32 = 1;

#[derive(Debug, Clone)]
enum Field {
    Varint(u64),
    Bytes(Vec<u8>),
    Fixed(Vec<u8>),
}

fn read_varint(data: &[u8], offset: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while *offset < data.len() && shift < 70 {
        let byte = data[*offset];
        *offset += 1;
        value |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
    }
    None
}

/// Split one protobuf message into its fields. An unknown field is kept, never guessed at.
fn protobuf_fields(data: &[u8]) -> Option<HashMap<u32, Vec<Field>>> {
    let mut fields: HashMap<u32, Vec<Field>> = HashMap::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let key = read_varint(data, &mut offset)?;
        let (number, wire) = ((key >> 3) as u32, (key & 7) as u8);
        if number == 0 {
            return None;
        }
        let field = match wire {
            0 => Field::Varint(read_varint(data, &mut offset)?),
            1 => {
                if offset + 8 > data.len() {
                    return None;
                }
                let value = data[offset..offset + 8].to_vec();
                offset += 8;
                Field::Fixed(value)
            }
            2 => {
                let length = read_varint(data, &mut offset)? as usize;
                if offset + length > data.len() {
                    return None;
                }
                let value = data[offset..offset + length].to_vec();
                offset += length;
                Field::Bytes(value)
            }
            5 => {
                if offset + 4 > data.len() {
                    return None;
                }
                let value = data[offset..offset + 4].to_vec();
                offset += 4;
                Field::Fixed(value)
            }
            _ => return None,
        };
        fields.entry(number).or_default().push(field);
    }
    Some(fields)
}

fn bytes_fields(fields: &HashMap<u32, Vec<Field>>, number: u32) -> Vec<&Vec<u8>> {
    fields
        .get(&number)
        .map(|values| {
            values
                .iter()
                .filter_map(|field| match field {
                    Field::Bytes(value) => Some(value),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn varint_field(fields: &HashMap<u32, Vec<Field>>, number: u32) -> Option<u64> {
    fields.get(&number)?.iter().find_map(|field| match field {
        Field::Varint(value) => Some(*value),
        _ => None,
    })
}

/// First entry that decodes as non-empty UTF-8.
fn decoded_text(values: &[&Vec<u8>]) -> String {
    for value in values {
        if let Ok(text) = std::str::from_utf8(value) {
            if !text.trim().is_empty() {
                return text.to_string();
            }
        }
    }
    String::new()
}

struct BlobStore<'a> {
    connection: &'a Connection,
    errors: Vec<String>,
    session: String,
}

impl BlobStore<'_> {
    /// Read one blob and verify it against its own digest, which is also its key.
    fn fetch(&mut self, identifier: &str) -> Option<Vec<u8>> {
        if identifier.len() != 64 {
            self.errors.push(format!("agent {}: invalid blob id", self.session));
            return None;
        }
        let row: Option<Vec<u8>> = self
            .connection
            .query_row("SELECT data FROM blobs WHERE id=?1", [identifier], |row| row.get(0))
            .ok();
        let Some(data) = row else {
            self.errors
                .push(format!("agent {}: missing blob {identifier}", self.session));
            return None;
        };
        let mut hasher = Sha256::new();
        hasher.update(&data);
        if format!("{:x}", hasher.finalize()) != identifier {
            self.errors
                .push(format!("agent {}: blob hash mismatch {identifier}", self.session));
            return None;
        }
        Some(data)
    }

    fn fetch_fields(&mut self, identifier: &str) -> Option<HashMap<u32, Vec<Field>>> {
        let blob = self.fetch(identifier)?;
        match protobuf_fields(&blob) {
            Some(fields) => Some(fields),
            None => {
                self.errors
                    .push(format!("agent {}: malformed protobuf {identifier}", self.session));
                None
            }
        }
    }
}

fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Read every Agent Host store under `root`, one unit per session.
pub fn collect(
    root: &Path,
    cutoff: Option<f64>,
    max_chars: usize,
    filter: UnitFilter<'_>,
) -> (Vec<FileUnit>, Vec<String>, usize, Vec<String>) {
    let mut units: Vec<FileUnit> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut stores: Vec<std::path::PathBuf> = Vec::new();
    if root.exists() {
        for workspace in std::fs::read_dir(root).into_iter().flatten().flatten() {
            for agent in std::fs::read_dir(workspace.path()).into_iter().flatten().flatten() {
                let candidate = agent.path().join("store.db");
                if candidate.is_file() {
                    stores.push(candidate);
                }
            }
        }
    }
    stores.sort();

    for store in &stores {
        let scope = format!("cursor-agent:{}", store.display());
        let fingerprint = file_fingerprint(store);
        if !filter(&scope, &fingerprint) {
            unchanged.push(scope);
            continue;
        }
        let mut records: Vec<Record> = Vec::new();
        match read_store(store, cutoff, max_chars, &mut records) {
            Ok(store_errors) => errors.extend(store_errors),
            Err(error) => errors.push(format!("{}: {error}", store.display())),
        }
        units.push(FileUnit { scope, fingerprint, records });
    }
    (units, unchanged, stores.len(), errors)
}

fn read_store(
    store: &Path,
    cutoff: Option<f64>,
    max_chars: usize,
    records: &mut Vec<Record>,
) -> Result<Vec<String>> {
    let connection = Connection::open_with_flags(
        store,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;
    let tables: std::collections::HashSet<String> = {
        let mut statement = connection.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    if !tables.contains("meta") || !tables.contains("blobs") {
        return Ok(vec![format!("{}: unrecognized Agent Host schema", store.display())]);
    }

    let meta: Option<String> = connection
        .query_row("SELECT value FROM meta WHERE key='0'", [], |row| row.get(0))
        .ok();
    let Some(meta) = meta else {
        return Ok(vec![format!("{}: missing meta root", store.display())]);
    };
    let decoded = (0..meta.len() / 2)
        .map(|position| u8::from_str_radix(&meta[position * 2..position * 2 + 2], 16))
        .collect::<std::result::Result<Vec<u8>, _>>();
    let Ok(decoded) = decoded else {
        return Ok(vec![format!("{}: malformed meta root", store.display())]);
    };
    let metadata: serde_json::Value = match serde_json::from_slice(&decoded) {
        Ok(value) => value,
        Err(_) => return Ok(vec![format!("{}: malformed meta root", store.display())]),
    };

    let session = metadata
        .get("agentId")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            store
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
    let created = metadata.get("createdAt").cloned().unwrap_or(serde_json::Value::Null);
    if !in_window(&created, cutoff) {
        return Ok(Vec::new());
    }

    let mut blobs = BlobStore { connection: &connection, errors: Vec::new(), session: session.clone() };
    let Some(root_id) = metadata.get("latestRootBlobId").and_then(|value| value.as_str()) else {
        return Ok(blobs.errors);
    };
    let Some(root_fields) = blobs.fetch_fields(root_id) else {
        return Ok(blobs.errors);
    };

    let turn_refs: Vec<String> = bytes_fields(&root_fields, ROOT_TURNS).iter().map(|value| hex(value)).collect();
    let timings: Vec<Option<u64>> = bytes_fields(&root_fields, ROOT_TIMINGS)
        .iter()
        .map(|value| protobuf_fields(value).and_then(|fields| varint_field(&fields, TIMING_EPOCH_MS)))
        .collect();

    for (position, turn_ref) in turn_refs.iter().enumerate() {
        let Some(turn_fields) = blobs.fetch_fields(turn_ref) else { continue };
        let agent_payloads = bytes_fields(&turn_fields, TURN_AGENT);
        let Some(agent_payload) = agent_payloads.first() else { continue };
        let Some(agent_fields) = protobuf_fields(agent_payload) else {
            blobs.errors.push(format!("agent {session}: malformed turn payload"));
            continue;
        };

        let stamp = timings
            .get(position)
            .copied()
            .flatten()
            .map(|value| serde_json::Value::from(value as f64))
            .unwrap_or_else(|| created.clone());
        let timestamp = epoch_seconds(&stamp).map(format_epoch);

        for user_ref in bytes_fields(&agent_fields, AGENT_USER).iter().take(1) {
            let identifier = hex(user_ref);
            if let Some(user_fields) = blobs.fetch_fields(&identifier) {
                let text = decoded_text(&bytes_fields(&user_fields, PAYLOAD_TEXT));
                if let Some(record) = Record::new(
                    "cursor",
                    &session,
                    timestamp.clone(),
                    Role::User,
                    None,
                    &text,
                    max_chars,
                ) {
                    records.push(record);
                }
            }
        }

        for step_ref in bytes_fields(&agent_fields, AGENT_STEPS) {
            let identifier = hex(step_ref);
            let Some(step_fields) = blobs.fetch_fields(&identifier) else { continue };
            let payloads = bytes_fields(&step_fields, PAYLOAD_TEXT);
            let Some(payload) = payloads.first() else { continue };
            let Some(assistant_fields) = protobuf_fields(payload) else { continue };
            let text = decoded_text(&bytes_fields(&assistant_fields, PAYLOAD_TEXT));
            if let Some(record) = Record::new(
                "cursor",
                &session,
                timestamp.clone(),
                Role::Assistant,
                None,
                &text,
                max_chars,
            ) {
                records.push(record);
            }
        }
    }

    Ok(blobs.errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    fn length_delimited(number: u32, body: &[u8]) -> Vec<u8> {
        let mut out = varint(((number as u64) << 3) | 2);
        out.extend(varint(body.len() as u64));
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn a_length_delimited_field_round_trips() {
        let message = length_delimited(PAYLOAD_TEXT, b"visible answer");
        let fields = protobuf_fields(&message).unwrap();
        assert_eq!(decoded_text(&bytes_fields(&fields, PAYLOAD_TEXT)), "visible answer");
    }

    #[test]
    fn a_varint_field_is_read_back() {
        let mut message = varint(((TIMING_EPOCH_MS as u64) << 3) | 0);
        message.extend(varint(1_788_267_600_000));
        let fields = protobuf_fields(&message).unwrap();
        assert_eq!(varint_field(&fields, TIMING_EPOCH_MS), Some(1_788_267_600_000));
    }

    #[test]
    fn a_truncated_message_is_rejected_rather_than_guessed() {
        let mut message = length_delimited(PAYLOAD_TEXT, b"visible answer");
        message.truncate(message.len() - 4);
        assert!(protobuf_fields(&message).is_none());
    }

    #[test]
    fn field_number_zero_is_invalid() {
        assert!(protobuf_fields(&[0x02, 0x01, 0x41]).is_none());
    }

    #[test]
    fn a_missing_directory_yields_no_units() {
        let (units, unchanged, stores, errors) = collect(
            Path::new("/tmp/vcs-cursor-agent-absent"),
            None,
            12_000,
            &mut |_, _| true,
        );
        assert!(units.is_empty() && unchanged.is_empty() && errors.is_empty());
        assert_eq!(stores, 0);
    }
}

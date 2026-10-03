//! Durable local index over visible conversation records.
//!
//! Storage is one SQLite file with an FTS5 trigram shadow table. Trigram search narrows candidates;
//! an exact case-insensitive substring check then verifies every candidate, so a result is never a
//! fuzzy or ranked guess. The schema is identical to the reference Python core, so both can read and
//! write the same file.

use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use std::path::Path;

use crate::Record;

/// Bumped whenever the stored shape changes, and also whenever the filter that decides what counts
/// as a visible message changes: unchanged files are never read again, so a filter change reaches
/// an existing index only through a rebuild. The index is derived data, so an older file is dropped
/// and rebuilt rather than migrated in place. Version 3: the harness-injection filter (3c92fc7)
/// landed without a bump, and an index built before it kept 101 injected records. Version 4: those
/// 101 records were an AGENTS.md dump after a context element, a shape the filter missed until
/// 2026-09-02; an index rebuilt at version 3 still holds them. Version 5: Claude Code slash-command
/// blocks (`<command-name>…`) are filtered the same way.
pub const SCHEMA_VERSION: i64 = 5;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS records (
    id INTEGER PRIMARY KEY,
    record_id TEXT NOT NULL UNIQUE,
    scope TEXT NOT NULL,
    source TEXT NOT NULL,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    timestamp TEXT,
    cwd TEXT,
    text TEXT NOT NULL,
    truncated INTEGER NOT NULL DEFAULT 0,
    dedupe_key TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS records_dedupe_idx ON records(dedupe_key);
CREATE INDEX IF NOT EXISTS records_scope_idx ON records(scope);
CREATE INDEX IF NOT EXISTS records_order_idx ON records(timestamp, source, session_id, id);
CREATE VIRTUAL TABLE IF NOT EXISTS records_fts USING fts5(
    text, content='records', content_rowid='id', tokenize='trigram'
);
CREATE TRIGGER IF NOT EXISTS records_insert AFTER INSERT ON records BEGIN
    INSERT INTO records_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS records_delete AFTER DELETE ON records BEGIN
    INSERT INTO records_fts(records_fts, rowid, text) VALUES ('delete', old.id, old.text);
END;
CREATE TRIGGER IF NOT EXISTS records_update AFTER UPDATE OF text ON records BEGIN
    INSERT INTO records_fts(records_fts, rowid, text) VALUES ('delete', old.id, old.text);
    INSERT INTO records_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TABLE IF NOT EXISTS watermarks (
    scope TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL
);
"#;

/// Shortest query the trigram index can accelerate. Shorter queries fall back to a full scan.
pub const MIN_TRIGRAM_QUERY: usize = 3;

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub source: String,
    pub session_id: String,
    pub role: String,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SyncOutcome {
    pub indexed: usize,
    pub removed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScopeCount {
    pub scope: String,
    pub records: i64,
}

/// Which end of the history a result set starts from.
///
/// The stored order is oldest first, which keeps a conversation readable in sequence. A person
/// searching their own history usually wants the most recent match, so the interface asks for the
/// other direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Oldest,
    Newest,
}

impl Order {
    fn parse(value: Option<&str>) -> Order {
        match value {
            Some("newest") | Some("desc") => Order::Newest,
            _ => Order::Oldest,
        }
    }
}

impl From<Option<&str>> for Order {
    fn from(value: Option<&str>) -> Order {
        Order::parse(value)
    }
}

/// Options for paginated tool searches. Existing desktop and CLI searches keep their own contract.
pub struct SearchOptions<'a> {
    pub limit: usize,
    pub offset: usize,
    pub order: Order,
    pub source: Option<&'a str>,
    pub cwd_prefix: Option<&'a str>,
    /// Inclusive UTC date or timestamp. A date means midnight UTC.
    pub since: Option<&'a str>,
    /// Exclusive UTC date or timestamp. Undated records are excluded when either bound is set.
    pub until: Option<&'a str>,
}

impl Default for SearchOptions<'_> {
    fn default() -> Self {
        Self {
            limit: 20,
            offset: 0,
            order: Order::Oldest,
            source: None,
            cwd_prefix: None,
            since: None,
            until: None,
        }
    }
}

pub struct SearchPage {
    pub hits: Vec<Hit>,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

// Adapters store UTC timestamps with zero or six fractional digits. Padding both forms makes
// comparisons exact to the microsecond: a whole second must sort before its fractional successor.
const SEARCH_TIME: &str = "CASE WHEN instr(timestamp, '.')=0
    THEN substr(timestamp, 1, 19)||'.000000Z'
    ELSE substr(timestamp, 1, 19)||'.'||
         substr(substr(timestamp, 21, length(timestamp)-21)||'000000', 1, 6)||'Z' END";

pub struct Index {
    connection: Connection,
}

impl Index {
    /// Open or create the index at `database`, creating parent directories as needed.
    pub fn open(database: &Path) -> Result<Index> {
        if let Some(parent) = database.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create index directory {}", parent.display()))?;
        }
        let connection = Connection::open(database)
            .with_context(|| format!("cannot open index {}", database.display()))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        tune(&connection)?;
        migrate(&connection)?;
        connection.execute_batch(SCHEMA)?;
        Ok(Index { connection })
    }

    /// Open an existing index read-only, for query paths that must not create or migrate anything.
    pub fn open_read_only(database: &Path) -> Result<Index> {
        let connection = Connection::open_with_flags(
            database,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .with_context(|| format!("cannot open index {}", database.display()))?;
        Ok(Index { connection })
    }

    pub fn in_memory() -> Result<Index> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(SCHEMA)?;
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Index { connection })
    }

    /// Replace everything previously stored under `scope` with `records`.
    ///
    /// A scope is one history file, or one Cursor conversation. Records absent from the new set are
    /// deleted, so a removed conversation leaves the index on the next sync. Original histories are
    /// never touched.
    pub fn sync(&mut self, scope: &str, records: &[Record]) -> Result<SyncOutcome> {
        self.sync_batch(&[(scope.to_string(), records.to_vec(), None)])
    }

    /// Synchronize many scopes in one transaction.
    ///
    /// One transaction per file cost 53 s over 6,702 files on a real history; one transaction per
    /// source brings the same work back to about 20 s. Each entry may carry the watermark to store
    /// with it, so a crash cannot leave a watermark claiming work that was rolled back.
    pub fn sync_batch(
        &mut self,
        batch: &[(String, Vec<Record>, Option<String>)],
    ) -> Result<SyncOutcome> {
        let transaction = self.connection.transaction()?;
        let mut indexed = 0usize;
        let mut removed = 0usize;
        {
            let mut insert = transaction.prepare(
                "INSERT INTO records(record_id, scope, source, session_id, role, timestamp, cwd, text, truncated, dedupe_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT DO NOTHING",
            )?;
            let mut existing_statement =
                transaction.prepare("SELECT record_id FROM records WHERE scope=?1")?;
            let mut delete = transaction.prepare("DELETE FROM records WHERE record_id=?1")?;
            let mut watermark = transaction.prepare(
                "INSERT INTO watermarks(scope, fingerprint) VALUES (?1, ?2)
                 ON CONFLICT(scope) DO UPDATE SET fingerprint=excluded.fingerprint",
            )?;
            let mut known_scope =
                transaction.prepare("SELECT 1 FROM records WHERE scope=?1 LIMIT 1")?;
            for (scope, records, fingerprint) in batch {
                // An empty scope has nothing to remove, so the full record_id fetch is skipped. The
                // probe is one indexed lookup; the fetch it replaces reads every stored id for the
                // scope. On a first build every scope is empty.
                let seen_before = known_scope.exists(params![scope])?;
                if seen_before {
                    let incoming: std::collections::HashSet<String> =
                        records.iter().map(|record| record.record_id()).collect();
                    let stored: Vec<String> = existing_statement
                        .query_map(params![scope], |row| row.get::<_, String>(0))?
                        .collect::<rusqlite::Result<Vec<String>>>()?;
                    for record_id in stored {
                        if !incoming.contains(&record_id) {
                            delete.execute(params![record_id])?;
                            removed += 1;
                        }
                    }
                }
                // `DO NOTHING` covers both unique constraints. A record already stored under another
                // scope stays where it is: a resumed session replays earlier messages with new
                // timestamps into a second file, and the reader should see that exchange once.
                for record in records {
                    insert.execute(params![
                        record.record_id(),
                        scope,
                        record.source,
                        record.session_id,
                        record.role.as_str(),
                        record.timestamp,
                        record.cwd,
                        record.text,
                        record.truncated as i64,
                        record.dedupe_key(),
                    ])?;
                }
                indexed += records.len();
                if let Some(fingerprint) = fingerprint {
                    watermark.execute(params![scope, fingerprint])?;
                }
            }
        }
        transaction.commit()?;
        Ok(SyncOutcome { indexed, removed })
    }

    /// Exact, case-insensitive substring search, oldest match first.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>> {
        self.search_ordered(query, limit, Order::Oldest)
    }

    /// Exact, case-insensitive substring search over visible conversation text.
    pub fn search_ordered(&self, query: &str, limit: usize, order: Order) -> Result<Vec<Hit>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.clamp(1, 5_000) as i64;
        let columns = "source, session_id, role, timestamp, cwd, text, truncated";
        // The inner query picks which rows; the outer one presents them. Both use the same direction
        // so the limited set is the newest or the oldest, not a slice of one shown in the other order.
        let direction = match order {
            Order::Oldest => "ASC",
            Order::Newest => "DESC",
        };
        let hits = if query.chars().count() >= MIN_TRIGRAM_QUERY {
            // Quoting turns the FTS expression into one literal phrase, so punctuation and spaces in
            // the user's phrase never become boolean operators.
            let expression = format!("\"{}\"", query.replace('"', "\"\""));
            // Two steps, because the sorter stores whatever the query selects. Selecting the text
            // and ordering in one step makes SQLite sort every matching row's full message through a
            // temporary b-tree: measured on a 92,566-record index, "forge deploy" cost about 400 ms
            // that way, and stayed near 400 ms at LIMIT 20 because the sort, not the result, was the
            // work. Ordering ids first and fetching the payload for the limited set costs about 12 ms.
            let sql = format!(
                "SELECT {columns} FROM records WHERE id IN (
                     SELECT r.id FROM records r
                     WHERE r.id IN (SELECT rowid FROM records_fts WHERE records_fts MATCH ?1)
                       AND instr(lower(r.text), lower(?2)) > 0
                     ORDER BY r.timestamp {direction}, r.source {direction},
                              r.session_id {direction}, r.id {direction} LIMIT ?3
                 )
                 ORDER BY timestamp {direction}, source {direction}, session_id {direction}, id {direction}"
            );
            let mut statement = self.connection.prepare(&sql)?;
            let rows = statement.query_map(params![expression, query, limit], read_hit)?;
            rows.collect::<rusqlite::Result<Vec<Hit>>>()?
        } else {
            let sql = format!(
                "SELECT {columns} FROM records WHERE instr(lower(text), lower(?1)) > 0
                 ORDER BY timestamp {direction}, source {direction}, session_id {direction},
                          id {direction} LIMIT ?2"
            );
            let mut statement = self.connection.prepare(&sql)?;
            let rows = statement.query_map(params![query, limit], read_hit)?;
            rows.collect::<rusqlite::Result<Vec<Hit>>>()?
        };
        Ok(hits)
    }

    /// Filter before paginating, retaining one extra row to report whether another page exists.
    pub fn search_page(&self, query: &str, options: &SearchOptions<'_>) -> Result<SearchPage> {
        if let Some(source) = options.source {
            if !matches!(source, "codex" | "claude" | "cursor") {
                bail!("source must be 'codex', 'claude', or 'cursor'");
            }
        }
        let cwd_prefix = match options.cwd_prefix {
            Some("") => bail!("cwd_prefix must not be empty"),
            Some(prefix) => Some(match prefix.trim_end_matches('/') {
                "" => "/",
                path => path,
            }),
            None => None,
        };
        let since = options
            .since
            .map(|value| search_date("since", value))
            .transpose()?;
        let until = options
            .until
            .map(|value| search_date("until", value))
            .transpose()?;
        if let (Some(since), Some(until)) = (&since, &until) {
            if since >= until {
                bail!("since must be earlier than until");
            }
        }
        let offset = i64::try_from(options.offset).map_err(|_| anyhow!("offset is too large"))?;
        let limit = options.limit.clamp(1, 5_000);
        let query = query.trim();
        if query.is_empty() {
            return Ok(SearchPage {
                hits: Vec::new(),
                has_more: false,
                next_offset: None,
            });
        }
        let accelerated = query.chars().count() >= MIN_TRIGRAM_QUERY;
        let expression = accelerated.then(|| format!("\"{}\"", query.replace('"', "\"\"")));
        let candidate_clause = if accelerated {
            "r.id IN (SELECT rowid FROM records_fts WHERE records_fts MATCH ?1) AND"
        } else {
            ""
        };
        let direction = match options.order {
            Order::Oldest => "ASC",
            Order::Newest => "DESC",
        };
        // Select ordered ids first, as in search_ordered, to avoid sorting every full text payload.
        // cwd comparisons are literal and respect directory boundaries; % and _ are not wildcards.
        let sql = format!(
            "SELECT source, session_id, role, timestamp, cwd, text, truncated FROM records
             WHERE id IN (
                 SELECT r.id FROM records r WHERE {candidate_clause}
                   instr(lower(r.text), lower(?2)) > 0
                   AND (?5 IS NULL OR r.source=?5)
                   AND (?6 IS NULL OR r.cwd=?6 OR instr(r.cwd, ?6||'/')=1
                        OR (?6='/' AND instr(r.cwd, '/')=1))
                   AND (?7 IS NULL OR ({SEARCH_TIME}) >= ?7)
                   AND (?8 IS NULL OR ({SEARCH_TIME}) < ?8)
                 ORDER BY ({SEARCH_TIME}) {direction}, r.source {direction},
                          r.session_id {direction}, r.id {direction} LIMIT ?3 OFFSET ?4
             )
             ORDER BY ({SEARCH_TIME}) {direction}, source {direction},
                      session_id {direction}, id {direction}"
        );
        let mut statement = self.connection.prepare(&sql)?;
        // The short-query path leaves ?1 unused; later numbered parameters still reserve its slot.
        let rows = statement.query_map(
            params![
                expression,
                query,
                (limit + 1) as i64,
                offset,
                options.source,
                cwd_prefix,
                since,
                until,
            ],
            read_hit,
        )?;
        let mut hits = rows.collect::<rusqlite::Result<Vec<Hit>>>()?;
        let has_more = hits.len() > limit;
        hits.truncate(limit);
        let next_offset = if has_more {
            Some(
                options
                    .offset
                    .checked_add(limit)
                    .ok_or_else(|| anyhow!("offset is too large"))?,
            )
        } else {
            None
        };
        Ok(SearchPage {
            hits,
            has_more,
            next_offset,
        })
    }

    /// Retrieve one stored visible record by its stable identity, without reading its neighbours.
    pub fn record(&self, record_id: &str) -> Result<Option<Hit>> {
        Ok(self
            .connection
            .query_row(
                "SELECT source, session_id, role, timestamp, cwd, text, truncated
             FROM records WHERE record_id=?1",
                params![record_id],
                read_hit,
            )
            .optional()?)
    }

    /// Records immediately around one hit, so the user can read the surrounding visible exchange.
    pub fn context(&self, session_id: &str, timestamp: Option<&str>, span: usize) -> Result<Vec<Hit>> {
        let columns = "source, session_id, role, timestamp, cwd, text, truncated";
        let sql = format!(
            "SELECT {columns} FROM records WHERE session_id=?1
             ORDER BY timestamp, source, session_id, id"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params![session_id], read_hit)?;
        let all = rows.collect::<rusqlite::Result<Vec<Hit>>>()?;
        let anchor = match timestamp {
            Some(value) => all
                .iter()
                .position(|hit| hit.timestamp.as_deref() == Some(value))
                .unwrap_or(0),
            None => 0,
        };
        let start = anchor.saturating_sub(span);
        let end = (anchor + span + 1).min(all.len());
        Ok(all[start..end].to_vec())
    }

    /// Every record at or after `since`, oldest first. Used by the seven-day map.
    pub fn since(&self, since: &str) -> Result<Vec<Hit>> {
        let mut statement = self.connection.prepare(
            "SELECT source, session_id, role, timestamp, cwd, text, truncated FROM records
             WHERE timestamp >= ?1 ORDER BY timestamp, source, session_id, id",
        )?;
        let rows = statement.query_map(params![since], read_hit)?;
        Ok(rows.collect::<rusqlite::Result<Vec<Hit>>>()?)
    }

    /// Fingerprint stored for `scope` at its last successful sync.
    pub fn watermark(&self, scope: &str) -> Result<Option<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT fingerprint FROM watermarks WHERE scope=?1")?;
        let mut rows = statement.query_map(params![scope], |row| row.get::<_, String>(0))?;
        Ok(match rows.next() {
            Some(value) => Some(value?),
            None => None,
        })
    }

    pub fn set_watermark(&self, scope: &str, fingerprint: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO watermarks(scope, fingerprint) VALUES (?1, ?2)
             ON CONFLICT(scope) DO UPDATE SET fingerprint=excluded.fingerprint",
            params![scope, fingerprint],
        )?;
        Ok(())
    }

    /// Every scope already stored for one source, so scopes whose file is gone can be removed.
    pub fn scopes_for_source(&self, source: &str) -> Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT DISTINCT scope FROM records WHERE source=?1")?;
        let rows = statement.query_map(params![source], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<String>>>()?)
    }

    /// Forget one scope entirely: its records and its watermark.
    pub fn drop_scope(&mut self, scope: &str) -> Result<usize> {
        let transaction = self.connection.transaction()?;
        let removed = transaction.execute("DELETE FROM records WHERE scope=?1", params![scope])?;
        transaction.execute("DELETE FROM watermarks WHERE scope=?1", params![scope])?;
        transaction.commit()?;
        Ok(removed)
    }

    /// Rows currently stored for one source. Reported instead of the number of records read, because
    /// a record already held under another scope is not stored twice.
    pub fn count_for_source(&self, source: &str) -> Result<i64> {
        Ok(self.connection.query_row(
            "SELECT count(*) FROM records WHERE source=?1",
            params![source],
            |row| row.get(0),
        )?)
    }

    pub fn total(&self) -> Result<i64> {
        Ok(self
            .connection
            .query_row("SELECT count(*) FROM records", [], |row| row.get(0))?)
    }

    /// Records per source. A scope is one file, so counting by scope would report thousands of rows.
    pub fn scope_counts(&self) -> Result<Vec<ScopeCount>> {
        let mut statement = self
            .connection
            .prepare("SELECT source, count(*) FROM records GROUP BY source ORDER BY source")?;
        let rows = statement.query_map([], |row| {
            Ok(ScopeCount {
                scope: row.get(0)?,
                records: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<ScopeCount>>>()?)
    }

    /// Distinctive phrases from the user's own recent records, used by the onboarding demonstration.
    ///
    /// Every returned phrase is a verbatim slice of a record, so searching for it always finds at
    /// least the record it came from. An earlier version joined two non-adjacent words, which
    /// produced a phrase that existed in no conversation and a demonstration that found nothing.
    pub fn suggested_phrases(&self, wanted: usize) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT text FROM records WHERE role='user' AND length(text) BETWEEN 24 AND 2000
             ORDER BY timestamp DESC LIMIT 400",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut phrases: Vec<String> = Vec::new();
        for text in rows.collect::<rusqlite::Result<Vec<String>>>()? {
            let Some(phrase) = distinctive_phrase(&text) else { continue };
            if phrases.iter().any(|existing| existing.eq_ignore_ascii_case(&phrase)) {
                continue;
            }
            // A phrase that matches a large share of the history demonstrates nothing, and one that
            // matches nothing is a defect. Keep the middle.
            let matches = self.search(&phrase, 60)?.len();
            if matches == 0 || matches >= 60 {
                continue;
            }
            phrases.push(phrase);
            if phrases.len() >= wanted {
                break;
            }
        }
        Ok(phrases)
    }
}

/// Page-cache settings.
///
/// A message can be 12,000 characters, so a result set of 200 pulls several megabytes out of
/// overflow pages. With the 2 MB default cache the same query cost 400 ms cold and 8 ms once the
/// pages were resident, which made a long-lived application pay the cold price repeatedly. A 64 MB
/// cache and a memory-mapped read path keep the second query as fast as the first.
fn tune(connection: &Connection) -> Result<()> {
    connection.pragma_update(None, "cache_size", -65_536)?;
    connection.pragma_update(None, "mmap_size", 536_870_912i64)?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(())
}

/// Drop an index written by an older build. The file only ever holds derived data.
fn migrate(connection: &Connection) -> Result<()> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < SCHEMA_VERSION {
        connection.execute_batch(
            "DROP TABLE IF EXISTS records_fts;
             DROP TABLE IF EXISTS records;
             DROP TABLE IF EXISTS watermarks;",
        )?;
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

/// Validate search bounds separately from the permissive adapter parser. Check ASCII before any
/// byte slicing so malformed user input, including Unicode, cannot panic.
fn search_date(name: &str, text: &str) -> Result<String> {
    let invalid = || {
        anyhow!("{name} must be a valid UTC date YYYY-MM-DD or timestamp YYYY-MM-DDTHH:MM:SS[.ffffff]Z (1–6 fractional digits)")
    };
    if !text.is_ascii() || text.len() < 10 {
        return Err(invalid());
    }
    let bytes = text.as_bytes();
    let number = |start: usize, end: usize| -> Result<u32> {
        let field = text.get(start..end).ok_or_else(invalid)?;
        if !field.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        field.parse().map_err(|_| invalid())
    };
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(invalid());
    }
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return Err(invalid()),
    };
    if year == 0 || day == 0 || day > days {
        return Err(invalid());
    }
    if text.len() == 10 {
        return Ok(format!("{text}T00:00:00.000000Z"));
    }
    if !(20..=27).contains(&text.len())
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || !text.ends_with('Z')
        || number(11, 13)? > 23
        || number(14, 16)? > 59
        || number(17, 19)? > 59
    {
        return Err(invalid());
    }
    let fraction = if text.len() == 20 {
        ""
    } else {
        if bytes[19] != b'.' || text.len() < 22 {
            return Err(invalid());
        }
        let fraction = &text[20..text.len() - 1];
        if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        fraction
    };
    Ok(format!(
        "{}.{}Z",
        &text[..19],
        &format!("{fraction}000000")[..6]
    ))
}

fn read_hit(row: &rusqlite::Row<'_>) -> rusqlite::Result<Hit> {
    Ok(Hit {
        source: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        timestamp: row.get(3)?,
        cwd: row.get(4)?,
        text: row.get(5)?,
        truncated: row.get::<_, i64>(6)? != 0,
    })
}

/// Pick a verbatim two-word slice of `text` that the trigram index can accelerate.
///
/// The words must be adjacent in the original message, so the returned phrase is a real substring.
fn distinctive_phrase(text: &str) -> Option<String> {
    let line = text.lines().find(|line| line.trim().chars().count() >= 12)?;
    let words: Vec<(usize, &str)> = line
        .char_indices()
        .filter(|(index, character)| {
            !character.is_whitespace() && (*index == 0 || line[..*index].ends_with(char::is_whitespace))
        })
        .map(|(index, _)| {
            let rest = &line[index..];
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (index, &rest[..end])
        })
        .collect();
    for window in words.windows(2) {
        let (start, first) = window[0];
        let (second_start, second) = window[1];
        if !usable(first) || !usable(second) {
            continue;
        }
        let phrase = line[start..second_start + second.len()].trim().to_string();
        if phrase.chars().count() >= 8 && phrase.chars().count() <= 48 {
            return Some(phrase);
        }
    }
    None
}

/// A word is usable when it is long enough for the trigram index and carries meaning.
fn usable(word: &str) -> bool {
    let trimmed = word.trim_matches(|character: char| !character.is_alphanumeric());
    let length = trimmed.chars().count();
    length >= 4
        && length <= 24
        && trimmed.chars().any(char::is_alphanumeric)
        && !COMMON_WORDS.contains(&trimmed.to_ascii_lowercase().as_str())
}

const COMMON_WORDS: [&str; 28] = [
    "that", "this", "with", "from", "have", "will", "your", "what", "when", "where", "which",
    "there", "here", "then", "than", "them", "they", "been", "were", "some", "into", "just",
    "like", "make", "more", "need", "please", "should",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;

    fn record(session: &str, role: Role, timestamp: &str, text: &str) -> Record {
        Record {
            source: "codex".into(),
            session_id: session.into(),
            timestamp: Some(timestamp.into()),
            role,
            cwd: Some("/tmp/project".into()),
            text: text.into(),
            truncated: false,
        }
    }

    #[test]
    fn search_finds_an_exact_case_insensitive_substring() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync(
                "codex",
                &[
                    record("a", Role::User, "2026-08-01T00:00:00Z", "Rebuild the Trigram index"),
                    record("a", Role::Assistant, "2026-08-01T00:01:00Z", "unrelated answer"),
                ],
            )
            .unwrap();
        let hits = index.search("trigram index", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].role, "user");
    }

    #[test]
    fn record_lookup_returns_exact_stored_text_and_metadata_or_none() {
        let mut index = Index::in_memory().unwrap();
        let mut original = record(
            "a",
            Role::Assistant,
            "2026-08-01T00:00:00Z",
            "🦀\nraw  text\t",
        );
        original.truncated = true;
        index.sync("codex", &[original.clone()]).unwrap();
        let hit = index.record(&original.record_id()).unwrap().unwrap();
        assert_eq!(hit.text, original.text);
        assert_eq!(hit.source, original.source);
        assert_eq!(hit.session_id, original.session_id);
        assert_eq!(hit.role, original.role.as_str());
        assert_eq!(hit.timestamp, original.timestamp);
        assert_eq!(hit.cwd, original.cwd);
        assert!(hit.truncated);
        assert!(index.record("missing-id").unwrap().is_none());
        // Lookup uses the persisted identity directly, rather than hashing the stored body again.
        index
            .connection
            .execute("UPDATE records SET record_id='persisted-id'", [])
            .unwrap();
        assert_eq!(
            index.record("persisted-id").unwrap().unwrap().text,
            original.text
        );
    }

    #[test]
    fn pages_in_both_orders_keep_ties_without_duplicates_and_detect_the_final_full_page() {
        let mut index = Index::in_memory().unwrap();
        let records: Vec<_> = (0..8)
            .map(|position| {
                let mut row = record(
                    if position % 3 == 0 { "b" } else { "a" },
                    Role::User,
                    "2026-09-02T00:00:00Z",
                    &format!("target ok {position}"),
                );
                row.source = if position % 2 == 0 { "codex" } else { "claude" }.into();
                row
            })
            .collect();
        index.sync("all", &records).unwrap();
        // Short queries and trigram queries use the same filtering, offsets, and stable tie order.
        for query in ["target", "ok"] {
            for order in [Order::Oldest, Order::Newest] {
                let expected: Vec<_> = index
                    .search_ordered(query, 20, order)
                    .unwrap()
                    .into_iter()
                    .map(|hit| hit.text)
                    .collect();
                let mut actual = Vec::new();
                for offset in [0, 2, 4, 6] {
                    let page = index
                        .search_page(
                            query,
                            &SearchOptions {
                                limit: 2,
                                offset,
                                order,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    assert_eq!(page.hits.len(), 2);
                    assert_eq!(page.has_more, offset < 6);
                    assert_eq!(
                        page.next_offset,
                        if offset < 6 { Some(offset + 2) } else { None }
                    );
                    actual.extend(page.hits.into_iter().map(|hit| hit.text));
                }
                assert_eq!(actual, expected);
                assert_eq!(
                    actual
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len(),
                    8
                );
                let past_end = index
                    .search_page(
                        query,
                        &SearchOptions {
                            offset: 8,
                            order,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                assert!(past_end.hits.is_empty());
                assert!(!past_end.has_more);
                assert_eq!(past_end.next_offset, None);
            }
        }
    }

    #[test]
    fn source_project_and_date_filters_run_before_pagination_with_exact_fractional_bounds() {
        let mut index = Index::in_memory().unwrap();
        let cases = [
            (
                "codex",
                Some("/tmp/project2"),
                Some("2026-09-02T00:00:00Z"),
                "sibling",
            ),
            (
                "claude",
                Some("/tmp/project"),
                Some("2026-09-02T00:00:00Z"),
                "source",
            ),
            (
                "codex",
                Some("/tmp/project"),
                Some("2026-09-01T23:59:59.999999Z"),
                "too-old",
            ),
            (
                "codex",
                Some("/tmp/project"),
                Some("2026-09-02T00:00:00Z"),
                "whole",
            ),
            (
                "codex",
                Some("/tmp/project/child"),
                Some("2026-09-02T00:00:00.000001Z"),
                "fraction",
            ),
            (
                "codex",
                Some("/tmp/project"),
                Some("2026-09-03T00:00:00Z"),
                "until",
            ),
            ("codex", Some("/tmp/project"), None, "undated"),
            ("codex", None, Some("2026-09-02T00:00:00Z"), "no-cwd"),
        ];
        let records: Vec<_> = cases
            .iter()
            .map(|(source, cwd, timestamp, label)| {
                let mut row = record(label, Role::User, "", &format!("target ok {label}"));
                row.source = (*source).into();
                row.cwd = cwd.map(str::to_string);
                row.timestamp = timestamp.map(str::to_string);
                row
            })
            .collect();
        index.sync("all", &records).unwrap();
        for query in ["target", "ok"] {
            let mut options = SearchOptions {
                limit: 1,
                source: Some("codex"),
                cwd_prefix: Some("/tmp/project/"),
                since: Some("2026-09-02"),
                until: Some("2026-09-03"),
                ..Default::default()
            };
            let first = index.search_page(query, &options).unwrap();
            assert_eq!(first.hits[0].session_id, "whole");
            assert_eq!(first.next_offset, Some(1));
            options.offset = 1;
            let last = index.search_page(query, &options).unwrap();
            assert_eq!(last.hits[0].session_id, "fraction");
            assert!(!last.has_more);
            options.offset = 0;
            options.until = Some("2026-09-02T00:00:00.000001Z");
            let whole = index.search_page(query, &options).unwrap();
            assert_eq!(whole.hits[0].session_id, "whole");
            assert!(!whole.has_more, "exclusive microsecond upper bound");
            options.since = Some("2026-09-02T00:00:00.000001Z");
            options.until = Some("2026-09-03");
            assert_eq!(
                index.search_page(query, &options).unwrap().hits[0].session_id,
                "fraction"
            );
        }
        let unbounded = index
            .search_page("target", &SearchOptions::default())
            .unwrap();
        assert_eq!(unbounded.hits[0].session_id, "undated");
        assert!(
            unbounded
                .hits
                .iter()
                .position(|hit| hit.session_id == "whole")
                .unwrap()
                < unbounded
                    .hits
                    .iter()
                    .position(|hit| hit.session_id == "fraction")
                    .unwrap()
        );
        let newest = index
            .search_page(
                "target",
                &SearchOptions {
                    order: Order::Newest,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(newest.hits.last().unwrap().session_id, "undated");
    }

    #[test]
    fn project_filter_treats_sql_wildcards_literally_and_handles_root() {
        let mut index = Index::in_memory().unwrap();
        let records: Vec<_> = [
            "/tmp/p%_",
            "/tmp/p%_/child",
            "/tmp/pXX",
            "/tmp/p%_sibling",
            "relative",
        ]
        .iter()
        .enumerate()
        .map(|(position, cwd)| {
            let mut row = record(
                &position.to_string(),
                Role::User,
                "2026-09-02T00:00:00Z",
                "target",
            );
            row.cwd = Some((*cwd).into());
            row
        })
        .collect();
        index.sync("all", &records).unwrap();
        let literal = index
            .search_page(
                "target",
                &SearchOptions {
                    cwd_prefix: Some("/tmp/p%_"),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            literal
                .hits
                .iter()
                .map(|hit| hit.session_id.as_str())
                .collect::<Vec<_>>(),
            ["0", "1"]
        );
        let root = index
            .search_page(
                "target",
                &SearchOptions {
                    cwd_prefix: Some("/"),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(root.hits.len(), 4);
    }

    #[test]
    fn short_queries_fall_back_to_a_scan_instead_of_returning_nothing() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync("codex", &[record("a", Role::User, "2026-08-01T00:00:00Z", "an ok result")])
            .unwrap();
        assert_eq!(index.search("ok", 10).unwrap().len(), 1);
    }

    #[test]
    fn trigram_candidates_are_verified_by_exact_substring() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync(
                "codex",
                &[
                    record("a", Role::User, "2026-08-01T00:00:00Z", "alpha beta"),
                    record("a", Role::User, "2026-08-01T00:02:00Z", "beta alpha"),
                ],
            )
            .unwrap();
        let hits = index.search("alpha beta", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "alpha beta");
    }

    #[test]
    fn the_accelerated_path_returns_what_a_plain_scan_would() {
        let mut index = Index::in_memory().unwrap();
        let mut records = Vec::new();
        for position in 0..200 {
            records.push(record(
                &format!("s{position}"),
                if position % 2 == 0 { Role::User } else { Role::Assistant },
                &format!("2026-08-01T00:{:02}:00Z", position % 60),
                &format!("common phrase number {position} with padding text"),
            ));
        }
        records.push(record("rare", Role::User, "2026-08-01T23:00:00Z", "a singular incantation"));
        index.sync("codex:/a.jsonl", &records).unwrap();

        // The trigram path and a bare substring scan must agree on both a common and a rare phrase.
        for phrase in ["common phrase", "singular incantation"] {
            let accelerated = index.search(phrase, 500).unwrap();
            let expected = records
                .iter()
                .filter(|record| record.text.to_lowercase().contains(&phrase.to_lowercase()))
                .count();
            assert_eq!(accelerated.len(), expected, "{phrase}");
        }
    }

    #[test]
    fn newest_first_returns_the_most_recent_matches_not_a_reversed_page() {
        let mut index = Index::in_memory().unwrap();
        let records: Vec<Record> = (0..5)
            .map(|position| {
                record(
                    "a",
                    Role::User,
                    &format!("2026-08-0{}T00:00:00Z", position + 1),
                    &format!("shared phrase {position}"),
                )
            })
            .collect();
        index.sync("codex:/a.jsonl", &records).unwrap();

        let newest = index.search_ordered("shared phrase", 2, Order::Newest).unwrap();
        assert_eq!(newest.len(), 2);
        assert_eq!(newest[0].text, "shared phrase 4");
        assert_eq!(newest[1].text, "shared phrase 3");

        let oldest = index.search_ordered("shared phrase", 2, Order::Oldest).unwrap();
        assert_eq!(oldest[0].text, "shared phrase 0");
    }

    #[test]
    fn results_are_ordered_by_time_then_source_then_session() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync(
                "codex",
                &[
                    record("b", Role::User, "2026-08-02T00:00:00Z", "shared phrase second"),
                    record("a", Role::User, "2026-08-01T00:00:00Z", "shared phrase first"),
                ],
            )
            .unwrap();
        let hits = index.search("shared phrase", 10).unwrap();
        assert_eq!(hits[0].text, "shared phrase first");
        assert_eq!(hits[1].text, "shared phrase second");
    }

    #[test]
    fn sync_removes_records_that_left_the_source() {
        let mut index = Index::in_memory().unwrap();
        let first = record("a", Role::User, "2026-08-01T00:00:00Z", "kept phrase");
        let second = record("a", Role::User, "2026-08-01T00:05:00Z", "dropped phrase");
        index.sync("codex", &[first.clone(), second]).unwrap();
        assert_eq!(index.total().unwrap(), 2);
        let outcome = index.sync("codex", &[first]).unwrap();
        assert_eq!(outcome.removed, 1);
        assert_eq!(index.total().unwrap(), 1);
        assert!(index.search("dropped phrase", 10).unwrap().is_empty());
    }

    #[test]
    fn the_same_exchange_replayed_with_a_new_timestamp_is_stored_once() {
        let mut index = Index::in_memory().unwrap();
        let first = record("s1", Role::User, "2026-08-01T00:00:00Z", "resume this session");
        let mut replayed = first.clone();
        replayed.timestamp = Some("2026-08-02T09:30:00Z".into());
        index.sync("codex:/one.jsonl", &[first]).unwrap();
        index.sync("codex:/two.jsonl", &[replayed]).unwrap();
        assert_eq!(index.total().unwrap(), 1);
        assert_eq!(index.count_for_source("codex").unwrap(), 1);
        let hits = index.search("resume this session", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].timestamp.as_deref(), Some("2026-08-01T00:00:00Z"));
    }

    #[test]
    fn a_watermark_round_trips_and_a_dropped_scope_forgets_both() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync("codex:/a.jsonl", &[record("a", Role::User, "2026-08-01T00:00:00Z", "kept phrase")])
            .unwrap();
        index.set_watermark("codex:/a.jsonl", "1234-99").unwrap();
        assert_eq!(index.watermark("codex:/a.jsonl").unwrap().as_deref(), Some("1234-99"));
        assert_eq!(index.scopes_for_source("codex").unwrap(), vec!["codex:/a.jsonl"]);
        assert_eq!(index.drop_scope("codex:/a.jsonl").unwrap(), 1);
        assert_eq!(index.watermark("codex:/a.jsonl").unwrap(), None);
        assert_eq!(index.total().unwrap(), 0);
    }

    #[test]
    fn sync_of_one_scope_leaves_another_scope_alone() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync("codex", &[record("a", Role::User, "2026-08-01T00:00:00Z", "codex phrase")])
            .unwrap();
        let mut claude_record = record("b", Role::User, "2026-08-01T00:00:00Z", "claude phrase");
        claude_record.source = "claude".into();
        index.sync("claude", &[claude_record]).unwrap();
        index.sync("claude", &[]).unwrap();
        assert_eq!(index.search("codex phrase", 10).unwrap().len(), 1);
        assert_eq!(index.total().unwrap(), 1);
    }

    #[test]
    fn a_quoted_query_cannot_become_an_fts_operator() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync(
                "codex",
                &[record("a", Role::User, "2026-08-01T00:00:00Z", "use \"quoted terms\" here")],
            )
            .unwrap();
        assert_eq!(index.search("\"quoted terms\"", 10).unwrap().len(), 1);
        assert_eq!(index.search("terms OR missing", 10).unwrap().len(), 0);
    }

    #[test]
    fn context_returns_the_surrounding_visible_exchange() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync(
                "codex",
                &[
                    record("a", Role::User, "2026-08-01T00:00:00Z", "first message"),
                    record("a", Role::Assistant, "2026-08-01T00:01:00Z", "second message"),
                    record("a", Role::User, "2026-08-01T00:02:00Z", "third message"),
                ],
            )
            .unwrap();
        let context = index.context("a", Some("2026-08-01T00:01:00Z"), 1).unwrap();
        assert_eq!(context.len(), 3);
        assert_eq!(context[0].text, "first message");
    }

    #[test]
    fn every_suggested_phrase_is_a_verbatim_slice_that_search_can_find() {
        let mut index = Index::in_memory().unwrap();
        let text = "Please rebuild the trigram candidate verification path today";
        index
            .sync("codex", &[record("a", Role::User, "2026-08-01T00:00:00Z", text)])
            .unwrap();
        let phrases = index.suggested_phrases(3).unwrap();
        assert_eq!(phrases.len(), 1);
        assert!(text.contains(&phrases[0]), "{} is not a slice of the record", phrases[0]);
        assert_eq!(index.search(&phrases[0], 10).unwrap().len(), 1);
    }

    #[test]
    fn a_phrase_that_matches_nothing_is_never_suggested() {
        let mut index = Index::in_memory().unwrap();
        index
            .sync(
                "codex",
                &[record("a", Role::User, "2026-08-01T00:00:00Z", "have you picked up the task yet")],
            )
            .unwrap();
        for phrase in index.suggested_phrases(3).unwrap() {
            assert!(
                !index.search(&phrase, 10).unwrap().is_empty(),
                "suggested {phrase} matches no record"
            );
        }
    }
}

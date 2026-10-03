//! Application service shared by the desktop shell and the command line.
//!
//! Everything the onboarding flow needs sits here so the Tauri commands and the local development
//! bridge run exactly the same code. Nothing in this crate reaches the network.

pub mod bench;
pub mod integrations;
pub mod map;
pub mod mcp;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use vcs_adapters::{SourceKind, SourceReport};
use vcs_core::index::{Hit, Index, SearchOptions};

/// Where the index lives when the caller does not choose a path.
pub fn default_database() -> PathBuf {
    vcs_adapters::home_directory()
        .join(".local/share/visible-conversation-search/index.sqlite")
}

/// Progress of an index build, polled by the welcome screen.
#[derive(Debug, Clone, Serialize)]
pub struct IndexProgress {
    /// `idle`, `reading`, `indexing`, `done`, `cancelled`, or `failed`.
    pub state: String,
    pub sources: Vec<SourceProgress>,
    pub records_total: usize,
    pub started_at_ms: u128,
    pub elapsed_ms: u128,
    /// Remaining time estimated from the sources already finished. `None` until one source is done.
    pub estimated_remaining_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceProgress {
    pub source: String,
    pub label: String,
    /// `pending`, `reading`, `indexing`, `done`, `skipped`, or `failed`.
    pub state: String,
    pub records: usize,
    pub files_read: usize,
    /// Units left untouched because their fingerprint still matched the stored one.
    pub unchanged: usize,
    pub elapsed_ms: u128,
    pub errors: Vec<String>,
}

impl IndexProgress {
    fn idle() -> IndexProgress {
        IndexProgress {
            state: "idle".into(),
            sources: Vec::new(),
            records_total: 0,
            started_at_ms: 0,
            elapsed_ms: 0,
            estimated_remaining_ms: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexRequest {
    /// Sources to index. Empty means every detected source.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Day window, or `None` for the whole history.
    #[serde(default)]
    pub days: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub elapsed_ms: f64,
    pub hits: Vec<Hit>,
    pub conversations: usize,
    pub truncated_at_limit: bool,
}

pub struct SearchPageResponse {
    pub search: SearchResponse,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

#[derive(Clone)]
pub struct App {
    database: PathBuf,
    progress: Arc<Mutex<IndexProgress>>,
    cancel: Arc<Mutex<bool>>,
    /// One long-lived read connection. Opening a 200 MB index per query added about 20 ms, which is
    /// an order of magnitude more than the query itself.
    reader: Arc<Mutex<Option<Index>>>,
}

impl App {
    pub fn new(database: PathBuf) -> App {
        App {
            database,
            progress: Arc::new(Mutex::new(IndexProgress::idle())),
            cancel: Arc::new(Mutex::new(false)),
            reader: Arc::new(Mutex::new(None)),
        }
    }

    pub fn database(&self) -> &Path {
        &self.database
    }

    /// Which histories exist on this computer. Fast: file presence only, no normalization.
    pub fn probe(&self) -> Vec<vcs_adapters::SourceProbe> {
        vcs_adapters::probe()
    }

    /// Exact visible-record counts, obtained by normalizing every store.
    ///
    /// This walks the whole history and takes tens of seconds on a large one, so it belongs behind an
    /// explicit request, never in front of the first screen.
    pub fn detect(&self, days: Option<u32>) -> Vec<SourceReport> {
        vcs_adapters::detect(cutoff(days), vcs_core::DEFAULT_MAX_CHARS)
    }

    pub fn progress(&self) -> IndexProgress {
        let mut progress = self.progress.lock().expect("progress lock").clone();
        if progress.state == "reading" || progress.state == "indexing" {
            progress.elapsed_ms = now_ms().saturating_sub(progress.started_at_ms);
        }
        progress
    }

    pub fn cancel(&self) {
        *self.cancel.lock().expect("cancel lock") = true;
    }

    /// Build the index in the calling thread, publishing progress as each source completes.
    pub fn build_index(&self, request: &IndexRequest) -> Result<IndexProgress> {
        let selected: Vec<SourceKind> = if request.sources.is_empty() {
            SourceKind::ALL.to_vec()
        } else {
            request.sources.iter().filter_map(|name| SourceKind::parse(name)).collect()
        };
        *self.cancel.lock().expect("cancel lock") = false;
        {
            let mut progress = self.progress.lock().expect("progress lock");
            *progress = IndexProgress {
                state: "reading".into(),
                sources: selected
                    .iter()
                    .map(|source| SourceProgress {
                        source: source.as_str().into(),
                        label: source.label().into(),
                        state: "pending".into(),
                        records: 0,
                        files_read: 0,
                        unchanged: 0,
                        elapsed_ms: 0,
                        errors: Vec::new(),
                    })
                    .collect(),
                records_total: 0,
                started_at_ms: now_ms(),
                elapsed_ms: 0,
                estimated_remaining_ms: None,
                error: None,
            };
        }

        let mut index = match Index::open(&self.database) {
            Ok(index) => index,
            Err(error) => {
                let mut progress = self.progress.lock().expect("progress lock");
                progress.state = "failed".into();
                progress.error = Some(error.to_string());
                return Err(error);
            }
        };

        for (position, source) in selected.iter().enumerate() {
            if *self.cancel.lock().expect("cancel lock") {
                let mut progress = self.progress.lock().expect("progress lock");
                progress.state = "cancelled".into();
                progress.elapsed_ms = now_ms().saturating_sub(progress.started_at_ms);
                return Ok(progress.clone());
            }
            self.set_source_state(position, "reading");
            let started = now_ms();
            let root = source.default_root();
            // The stored fingerprint carries the day window, so narrowing or widening the window
            // re-reads every unit instead of leaving records from the previous window behind.
            let window = request.days.map(|days| days.to_string()).unwrap_or_else(|| "all".into());
            let collected = {
                let index_ref = &index;
                let window = window.clone();
                vcs_adapters::collect_with(
                    *source,
                    &root,
                    cutoff(request.days),
                    vcs_core::DEFAULT_MAX_CHARS,
                    &mut move |scope, fingerprint| {
                        let expected = format!("{fingerprint}|{window}");
                        !matches!(index_ref.watermark(scope), Ok(Some(stored)) if stored == expected)
                    },
                )
            };
            match collected {
                Ok(collected) => {
                    self.set_source_state(position, "indexing");
                    let outcome = (|| -> anyhow::Result<usize> {
                        let batch: Vec<(String, Vec<vcs_core::Record>, Option<String>)> = collected
                            .units
                            .iter()
                            .map(|unit| {
                                (
                                    unit.scope.clone(),
                                    unit.records.clone(),
                                    Some(format!("{}|{}", unit.fingerprint, window)),
                                )
                            })
                            .collect();
                        index.sync_batch(&batch)?;
                        // A file that left the store takes its records with it.
                        let live: std::collections::HashSet<String> =
                            collected.live_scopes().into_iter().collect();
                        for scope in index.scopes_for_source(source.as_str())? {
                            if !live.contains(&scope) {
                                index.drop_scope(&scope)?;
                            }
                        }
                        // Report stored rows, not records read: a replayed exchange is stored once.
                        Ok(index.count_for_source(source.as_str())? as usize)
                    })();
                    let mut progress = self.progress.lock().expect("progress lock");
                    let entry = &mut progress.sources[position];
                    entry.files_read = collected.report.files_read;
                    entry.unchanged = collected.unchanged.len();
                    entry.errors = collected.report.errors.clone();
                    entry.elapsed_ms = now_ms().saturating_sub(started);
                    match outcome {
                        Ok(stored) => {
                            entry.records = stored;
                            entry.state = if collected.report.status == "ok" {
                                "done".into()
                            } else {
                                collected.report.status.clone()
                            };
                            progress.records_total += stored;
                        }
                        Err(error) => {
                            entry.state = "failed".into();
                            entry.errors.push(error.to_string());
                        }
                    }
                    let done = position + 1;
                    if done < progress.sources.len() {
                        let spent = now_ms().saturating_sub(progress.started_at_ms);
                        let remaining = (progress.sources.len() - done) as u128;
                        progress.estimated_remaining_ms = Some(spent / done as u128 * remaining);
                    } else {
                        progress.estimated_remaining_ms = Some(0);
                    }
                }
                Err(error) => {
                    let mut progress = self.progress.lock().expect("progress lock");
                    let entry = &mut progress.sources[position];
                    entry.state = "failed".into();
                    entry.errors.push(error.to_string());
                    entry.elapsed_ms = now_ms().saturating_sub(started);
                }
            }
        }

        self.invalidate_reader();
        let mut progress = self.progress.lock().expect("progress lock");
        progress.state = "done".into();
        progress.elapsed_ms = now_ms().saturating_sub(progress.started_at_ms);
        progress.estimated_remaining_ms = Some(0);
        Ok(progress.clone())
    }

    fn set_source_state(&self, position: usize, state: &str) {
        let mut progress = self.progress.lock().expect("progress lock");
        if let Some(entry) = progress.sources.get_mut(position) {
            entry.state = state.into();
        }
        progress.state = state.into();
    }

    /// Run `action` against the shared read connection, opening it on first use.
    fn with_reader<T>(&self, action: impl FnOnce(&Index) -> Result<T>) -> Result<T> {
        let mut guard = self.reader.lock().expect("reader lock");
        if guard.is_none() {
            *guard = Some(Index::open(&self.database)?);
        }
        action(guard.as_ref().expect("reader opened"))
    }

    /// Drop the cached connection so the next query sees a rebuilt index.
    fn invalidate_reader(&self) {
        *self.reader.lock().expect("reader lock") = None;
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<SearchResponse> {
        self.search_ordered(query, limit, vcs_core::index::Order::Oldest)
    }

    pub fn search_ordered(
        &self,
        query: &str,
        limit: usize,
        order: vcs_core::index::Order,
    ) -> Result<SearchResponse> {
        let mut guard = self.reader.lock().expect("reader lock");
        if guard.is_none() {
            *guard = Some(Index::open(&self.database)?);
        }
        let index = guard.as_ref().expect("reader opened");
        let started = std::time::Instant::now();
        let hits = index.search_ordered(query, limit, order)?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut sessions: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for hit in &hits {
            sessions.insert(hit.session_id.as_str());
        }
        Ok(SearchResponse {
            query: query.to_string(),
            elapsed_ms,
            conversations: sessions.len(),
            truncated_at_limit: hits.len() == limit,
            hits,
        })
    }

    /// Paginated search used by MCP. Counts describe this page, not the complete matching set.
    pub fn search_page(
        &self,
        query: &str,
        options: &SearchOptions<'_>,
    ) -> Result<SearchPageResponse> {
        self.with_reader(|index| {
            let started = std::time::Instant::now();
            let page = index.search_page(query, options)?;
            let conversations = page
                .hits
                .iter()
                .map(|hit| &hit.session_id)
                .collect::<std::collections::HashSet<_>>()
                .len();
            Ok(SearchPageResponse {
                search: SearchResponse {
                    query: query.to_string(),
                    elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                    conversations,
                    truncated_at_limit: page.has_more,
                    hits: page.hits,
                },
                has_more: page.has_more,
                next_offset: page.next_offset,
            })
        })
    }

    pub fn context(&self, session_id: &str, timestamp: Option<&str>, span: usize) -> Result<Vec<Hit>> {
        self.with_reader(|index| index.context(session_id, timestamp, span))
    }

    /// Full text available in the index for one visible record. `truncated` describes index limits.
    pub fn record(&self, record_id: &str) -> Result<Option<Hit>> {
        self.with_reader(|index| index.record(record_id))
    }

    /// Phrases proposed on the speed screen.
    ///
    /// They are chosen for search behaviour — long enough for the trigram index, specific enough to
    /// match a small number of conversations — not because they are important or sensitive.
    pub fn suggested_phrases(&self, wanted: usize) -> Result<Vec<String>> {
        self.with_reader(|index| index.suggested_phrases(wanted))
    }

    pub fn totals(&self) -> Result<(i64, Vec<vcs_core::index::ScopeCount>)> {
        self.with_reader(|index| Ok((index.total()?, index.scope_counts()?)))
    }
}

/// Convert a day window into an epoch cutoff.
pub fn cutoff(days: Option<u32>) -> Option<f64> {
    days.map(|days| now_ms() as f64 / 1000.0 - days as f64 * 86_400.0)
}

pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vcs_core::{Record, Role};

    fn temporary_database(name: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join(format!("vcs-service-{name}-{}", std::process::id()))
            .join("index.sqlite");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        path
    }

    fn seeded(name: &str) -> App {
        let app = App::new(temporary_database(name));
        let mut index = Index::open(app.database()).unwrap();
        index
            .sync(
                "codex",
                &[
                    Record {
                        source: "codex".into(),
                        session_id: "s1".into(),
                        timestamp: Some("2026-09-01T10:00:00Z".into()),
                        role: Role::User,
                        cwd: Some("/tmp/project".into()),
                        text: "rebuild the trigram candidate path".into(),
                        truncated: false,
                    },
                    Record {
                        source: "codex".into(),
                        session_id: "s2".into(),
                        timestamp: Some("2026-09-01T11:00:00Z".into()),
                        role: Role::Assistant,
                        cwd: Some("/tmp/project".into()),
                        text: "the trigram candidate path is rebuilt".into(),
                        truncated: false,
                    },
                ],
            )
            .unwrap();
        app
    }

    #[test]
    fn search_reports_its_own_elapsed_time_and_conversation_count() {
        let app = seeded("search");
        let response = app.search("trigram candidate", 50).unwrap();
        assert_eq!(response.hits.len(), 2);
        assert_eq!(response.conversations, 2);
        assert!(response.elapsed_ms >= 0.0);
        assert!(!response.truncated_at_limit);
    }

    #[test]
    fn a_result_set_that_fills_the_limit_is_marked_truncated() {
        let app = seeded("truncated");
        assert!(app.search("trigram candidate", 1).unwrap().truncated_at_limit);
    }

    #[test]
    fn totals_report_one_row_per_scope() {
        let app = seeded("totals");
        let (total, scopes) = app.totals().unwrap();
        assert_eq!(total, 2);
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].scope, "codex");
    }

    #[test]
    fn progress_starts_idle_and_a_build_ends_done() {
        let app = App::new(temporary_database("progress"));
        assert_eq!(app.progress().state, "idle");
        let progress = app
            .build_index(&IndexRequest { sources: vec!["cursor".into()], days: Some(1) })
            .unwrap();
        assert_eq!(progress.state, "done");
        assert_eq!(progress.sources.len(), 1);
        assert_eq!(progress.estimated_remaining_ms, Some(0));
    }

    #[test]
    fn a_second_build_skips_units_whose_fingerprint_did_not_change() {
        let home = std::env::temp_dir().join(format!("vcs-incremental-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let session = home.join(".codex/sessions/rollout-a.jsonl");
        std::fs::create_dir_all(session.parent().unwrap()).unwrap();
        std::fs::write(
            &session,
            concat!(
                r#"{"type":"session_meta","payload":{"id":"a","cwd":"/tmp/p"}}"#,
                "\n",
                r#"{"type":"event_msg","timestamp":"2026-09-01T10:00:00Z","payload":{"type":"user_message","message":"first prompt"}}"#,
            ),
        )
        .unwrap();

        // The adapters resolve their store from HOME, so this test rebinds it. No other test asserts
        // anything about the real home directory, so the process-wide change is safe here.
        let app = App::new(home.join("index.sqlite"));
        let previous = std::env::var("HOME").ok();
        std::env::set_var("HOME", &home);

        let first = app
            .build_index(&IndexRequest { sources: vec!["codex".into()], days: None })
            .unwrap();
        assert_eq!(first.sources[0].records, 1);
        assert_eq!(first.sources[0].unchanged, 0);

        let second = app
            .build_index(&IndexRequest { sources: vec!["codex".into()], days: None })
            .unwrap();
        assert_eq!(second.sources[0].unchanged, 1);
        assert_eq!(second.sources[0].records, 1, "the stored record count is unchanged");
        assert_eq!(app.totals().unwrap().0, 1, "its records stay in the index");

        // A file that leaves the store takes its records with it on the next build.
        std::fs::remove_file(&session).unwrap();
        app.build_index(&IndexRequest { sources: vec!["codex".into()], days: None })
            .unwrap();
        assert_eq!(app.totals().unwrap().0, 0);

        match previous {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_unknown_source_name_is_ignored_rather_than_indexed_blindly() {
        let app = App::new(temporary_database("unknown"));
        let progress = app
            .build_index(&IndexRequest { sources: vec!["notes".into()], days: Some(1) })
            .unwrap();
        assert!(progress.sources.is_empty());
        assert_eq!(progress.records_total, 0);
    }
}

//! Optional seven-day map: what the recent visible conversations were about, grouped by project.
//!
//! This is a secondary view. It reads the same index and never changes what search can do.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use vcs_core::index::Index;

use crate::App;

#[derive(Debug, Clone, Serialize)]
pub struct Topic {
    pub label: String,
    pub records: usize,
    pub sessions: Vec<SessionRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRef {
    pub session_id: String,
    pub source: String,
    pub first_seen: Option<String>,
    pub opening_line: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectNode {
    pub project: String,
    pub path: Option<String>,
    pub conversations: usize,
    pub records: usize,
    pub sources: Vec<String>,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentMap {
    pub days: u32,
    pub since: String,
    pub records: usize,
    pub conversations: usize,
    pub projects: Vec<ProjectNode>,
}

/// Words that say nothing about what a conversation was about.
const STOP_WORDS: [&str; 60] = [
    "about", "after", "again", "also", "another", "back", "because", "been", "before", "being",
    "both", "call", "come", "could", "does", "doing", "done", "down", "each", "even", "every",
    "first", "from", "give", "going", "have", "here", "into", "just", "know", "like", "look",
    "made", "make", "many", "more", "most", "much", "need", "only", "other", "over", "same",
    "should", "since", "some", "still", "such", "take", "than", "that", "them", "then", "there",
    "these", "they", "this", "very", "well", "what",
];

pub fn build(app: &App, days: u32) -> Result<RecentMap> {
    let since_epoch = crate::now_ms() as f64 / 1000.0 - days as f64 * 86_400.0;
    let since = vcs_core::time::format_epoch(since_epoch);
    let index = Index::open(app.database())?;
    let hits = index.since(&since)?;

    let mut grouped: HashMap<String, Vec<&vcs_core::index::Hit>> = HashMap::new();
    for hit in &hits {
        let project = hit
            .cwd
            .as_deref()
            .and_then(|path| path.rsplit('/').find(|part| !part.is_empty()))
            .unwrap_or("Unassigned")
            .to_string();
        grouped.entry(project).or_default().push(hit);
    }

    let mut projects: Vec<ProjectNode> = grouped
        .into_iter()
        .map(|(project, records)| {
            let mut sessions: Vec<&str> = records.iter().map(|hit| hit.session_id.as_str()).collect();
            sessions.sort_unstable();
            sessions.dedup();
            let mut sources: Vec<String> =
                records.iter().map(|hit| hit.source.clone()).collect();
            sources.sort();
            sources.dedup();
            ProjectNode {
                path: records.iter().find_map(|hit| hit.cwd.clone()),
                topics: topics_for(&records),
                conversations: sessions.len(),
                records: records.len(),
                sources,
                project,
            }
        })
        .collect();
    projects.sort_by(|left, right| {
        right
            .records
            .cmp(&left.records)
            .then_with(|| left.project.cmp(&right.project))
    });

    let mut all_sessions: Vec<&str> = hits.iter().map(|hit| hit.session_id.as_str()).collect();
    all_sessions.sort_unstable();
    all_sessions.dedup();

    Ok(RecentMap {
        days,
        since,
        records: hits.len(),
        conversations: all_sessions.len(),
        projects,
    })
}

/// Group a project's sessions under the terms its own messages use most.
fn topics_for(records: &[&vcs_core::index::Hit]) -> Vec<Topic> {
    let mut frequency: HashMap<String, usize> = HashMap::new();
    for hit in records {
        if hit.role != "user" {
            continue;
        }
        for term in terms(&hit.text) {
            *frequency.entry(term).or_insert(0) += 1;
        }
    }
    let mut ranked: Vec<(String, usize)> = frequency.into_iter().filter(|(_, count)| *count > 1).collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked.truncate(5);

    let mut topics: Vec<Topic> = Vec::new();
    for (label, _) in ranked {
        let mut sessions: Vec<SessionRef> = Vec::new();
        let mut count = 0usize;
        for hit in records {
            if !hit.text.to_lowercase().contains(&label) {
                continue;
            }
            count += 1;
            if sessions.iter().any(|entry| entry.session_id == hit.session_id) {
                continue;
            }
            sessions.push(SessionRef {
                session_id: hit.session_id.clone(),
                source: hit.source.clone(),
                first_seen: hit.timestamp.clone(),
                opening_line: opening_line(&hit.text),
            });
        }
        sessions.truncate(6);
        if !sessions.is_empty() {
            topics.push(Topic { label, records: count, sessions });
        }
    }
    topics
}

/// Remove angle-bracket markup before deriving topics.
///
/// A slash command reaches the transcript wrapped in tags such as `<command-message>`. The tag names
/// are structure, not subject: without this, `command-message` and `command-name` rank as topics in
/// every project that used a slash command.
fn without_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for character in text.chars() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(character),
            _ => {}
        }
    }
    out
}

fn terms(text: &str) -> Vec<String> {
    without_markup(text)
        .split(|character: char| !character.is_alphanumeric() && character != '-')
        .map(|word| word.to_lowercase())
        .filter(|word| word.chars().count() >= 4 && word.chars().count() <= 20)
        .filter(|word| !STOP_WORDS.contains(&word.as_str()))
        .filter(|word| !word.chars().all(|character| character.is_ascii_digit()))
        .collect()
}

#[cfg(test)]
mod markup_tests {
    use super::without_markup;

    #[test]
    fn tag_names_are_removed_but_their_content_stays() {
        assert_eq!(
            without_markup("<command-message>overnight</command-message> run"),
            "overnight run"
        );
    }
}

fn opening_line(text: &str) -> String {
    let line = text.lines().find(|line| !line.trim().is_empty()).unwrap_or("").trim();
    let mut short: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        short.push('…');
    }
    short
}

#[cfg(test)]
mod tests {
    use super::*;
    use vcs_core::{Record, Role};

    fn app_with(records: Vec<Record>, name: &str) -> App {
        let path = std::env::temp_dir()
            .join(format!("vcs-map-{name}-{}", std::process::id()))
            .join("index.sqlite");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        let app = App::new(path);
        Index::open(app.database()).unwrap().sync("codex", &records).unwrap();
        app
    }

    fn record(cwd: &str, session: &str, text: &str, role: Role) -> Record {
        Record {
            source: "codex".into(),
            session_id: session.into(),
            timestamp: Some(vcs_core::time::format_epoch(crate::now_ms() as f64 / 1000.0 - 3600.0)),
            role,
            cwd: Some(cwd.into()),
            text: text.into(),
            truncated: false,
        }
    }

    #[test]
    fn conversations_group_by_project_directory() {
        let app = app_with(
            vec![
                record("/tmp/alpha", "s1", "trigram index work continues", Role::User),
                record("/tmp/alpha", "s2", "trigram index rebuild again", Role::User),
                record("/tmp/beta", "s3", "unrelated billing question", Role::User),
            ],
            "group",
        );
        let map = build(&app, 7).unwrap();
        assert_eq!(map.projects.len(), 2);
        assert_eq!(map.projects[0].project, "alpha");
        assert_eq!(map.projects[0].conversations, 2);
    }

    #[test]
    fn a_repeated_term_becomes_a_topic_with_its_sessions() {
        let app = app_with(
            vec![
                record("/tmp/alpha", "s1", "trigram index work", Role::User),
                record("/tmp/alpha", "s2", "trigram index rebuild", Role::User),
            ],
            "topic",
        );
        let map = build(&app, 7).unwrap();
        let topic = &map.projects[0].topics[0];
        assert_eq!(topic.label, "index");
        assert_eq!(topic.sessions.len(), 2);
    }

    #[test]
    fn records_outside_the_window_are_absent() {
        let mut old = record("/tmp/alpha", "s1", "ancient conversation", Role::User);
        old.timestamp = Some("2020-01-01T00:00:00Z".into());
        let app = app_with(vec![old], "window");
        assert_eq!(build(&app, 7).unwrap().records, 0);
    }
}

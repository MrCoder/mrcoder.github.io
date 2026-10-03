//! Model Context Protocol server over stdin and stdout.
//!
//! This is what the integration screen connects an AI tool to. The tool starts `vcs mcp`, or the
//! packaged application binary with the same `mcp` argument, as a child process and speaks JSON-RPC
//! over the pipe, so no port is opened and no conversation leaves the
//! computer. The server exposes read-only tools; it can search the index and read visible context,
//! and it cannot index, connect, or modify anything.

use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use vcs_core::index::{Hit, Order, SearchOptions};
use crate::App;

const PROTOCOL_VERSION: &str = "2025-06-18";
const SNIPPET_CHARS: usize = 600;

#[derive(Serialize)]
struct McpRecord {
    record_id: String,
    #[serde(flatten)]
    hit: Hit,
}

impl McpRecord {
    fn new(hit: Hit) -> Result<Self> {
        // Hash the complete indexed text before clipping. These are exactly the identity fields
        // used on insert; cwd and the index truncation flag do not contribute to the stored id.
        let record_id = vcs_core::Record {
            source: hit.source.clone(),
            session_id: hit.session_id.clone(),
            role: vcs_core::Role::parse(&hit.role).ok_or_else(|| anyhow!("invalid stored role"))?,
            timestamp: hit.timestamp.clone(),
            cwd: hit.cwd.clone(),
            text: hit.text.clone(),
            truncated: hit.truncated,
        }
        .record_id();
        Ok(Self { record_id, hit })
    }
}

#[derive(Serialize)]
struct McpSearchHit {
    #[serde(flatten)]
    record: McpRecord,
    /// All positions count Unicode scalar values in the original indexed text; ends are exclusive.
    snippet_start_char: usize,
    snippet_end_char: usize,
    total_chars: usize,
    match_start_char: usize,
    match_end_char: usize,
    snippet_clipped: bool,
}

impl McpSearchHit {
    fn new(hit: Hit, query: &str, full: bool) -> Result<Self> {
        let mut record = McpRecord::new(hit)?;
        let text = &record.hit.text;
        // SQLite's built-in lower() folds ASCII only. It leaves UTF-8 byte lengths unchanged, so
        // these byte positions can be translated safely back into the exact original text.
        let query = query.trim();
        let match_byte = text
            .to_ascii_lowercase()
            .find(&query.to_ascii_lowercase())
            .ok_or_else(|| anyhow!("search hit does not contain the query"))?;
        let match_start_char = text[..match_byte].chars().count();
        let match_end_char = match_start_char + query.chars().count();
        let total_chars = text.chars().count();
        let (snippet_start_char, snippet_end_char) = if full {
            (0, total_chars)
        } else {
            // Keep the whole match even for a phrase longer than the usual snippet budget.
            let budget = SNIPPET_CHARS.max(match_end_char - match_start_char);
            let before = (budget - (match_end_char - match_start_char)) / 2;
            let start = match_start_char.saturating_sub(before);
            let end = start.saturating_add(budget).min(total_chars);
            (end.saturating_sub(budget), end)
        };
        let snippet_clipped = snippet_start_char > 0 || snippet_end_char < total_chars;
        if snippet_clipped {
            record.hit.text = text
                .chars()
                .skip(snippet_start_char)
                .take(snippet_end_char - snippet_start_char)
                .collect();
        }
        Ok(Self {
            record,
            snippet_start_char,
            snippet_end_char,
            total_chars,
            match_start_char,
            match_end_char,
            snippet_clipped,
        })
    }
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "search_visible_conversations",
            "description": "Search visible user and assistant messages from Codex, Claude Code, and Cursor by literal substring (ASCII case-insensitive). Filters apply before pagination. Follow next_offset while has_more=true, keeping query, filters, order, and limit unchanged; pages are stable while the index is unchanged. conversations counts only this page. Default text is an exact raw snippet of about 600 characters around the first match; use read_conversation_record with record_id for full indexed text, or format=full. Character offsets count Unicode scalar values in the original indexed text, with exclusive ends. snippet_clipped reports snippet clipping; truncated separately reports an index-truncated tail, which full reads cannot recover. Hidden reasoning, tool calls, and tool output are not indexed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Literal phrase to find." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 20 },
                    "format": { "type": "string", "enum": ["snippet", "full"], "default": "snippet" },
                    "offset": { "type": "integer", "minimum": 0, "default": 0 },
                    "order": { "type": "string", "enum": ["oldest", "newest"], "default": "oldest" },
                    "source": { "type": "string", "enum": ["codex", "claude", "cursor"] },
                    "cwd_prefix": { "type": "string", "description": "Literal directory path: matches that directory and its descendants, not similarly named siblings. Trailing slashes are ignored." },
                    "since": { "type": "string", "description": "Inclusive UTC date YYYY-MM-DD (midnight UTC) or timestamp YYYY-MM-DDTHH:MM:SS[.ffffff]Z, with 1–6 fractional digits. Excludes undated records." },
                    "until": { "type": "string", "description": "Exclusive UTC date YYYY-MM-DD (midnight UTC) or timestamp YYYY-MM-DDTHH:MM:SS[.ffffff]Z, with 1–6 fractional digits. Must be after since. Excludes undated records." }
                },
                "required": ["query"]
            }
        },
        {
            "name": "read_conversation_record",
            "description": "Read one full visible record from the local index using a search result's record_id. Returns exact indexed text and metadata. truncated=true means the index omitted a tail; this tool cannot recover that tail. An unknown record_id returns an error.",
            "inputSchema": {
                "type": "object",
                "properties": { "record_id": { "type": "string" } },
                "required": ["record_id"]
            }
        },
        {
            "name": "read_conversation_context",
            "description": "Read the visible messages around one match, by session id and timestamp.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": { "type": "string" },
                    "timestamp": { "type": "string" },
                    "span": { "type": "integer", "minimum": 1, "maximum": 20, "default": 3 }
                },
                "required": ["session_id"]
            }
        }
    ])
}

fn text_content(body: String) -> Value {
    json!({ "content": [{ "type": "text", "text": body }] })
}

fn search_options(arguments: &Value) -> Result<SearchOptions<'_>> {
    let string = |name: &str| -> Result<Option<&str>> {
        arguments
            .get(name)
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| anyhow!("{name} must be a string"))
            })
            .transpose()
    };
    let integer = |name: &str, default: u64| -> Result<u64> {
        arguments
            .get(name)
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| anyhow!("{name} must be a non-negative integer"))
            })
            .transpose()
            .map(|value| value.unwrap_or(default))
    };
    let limit = integer("limit", 20)?;
    if !(1..=200).contains(&limit) {
        return Err(anyhow!("limit must be between 1 and 200"));
    }
    let offset =
        usize::try_from(integer("offset", 0)?).map_err(|_| anyhow!("offset is too large"))?;
    let order = match string("order")? {
        None | Some("oldest") => Order::Oldest,
        Some("newest") => Order::Newest,
        _ => return Err(anyhow!("order must be 'oldest' or 'newest'")),
    };
    Ok(SearchOptions {
        limit: limit as usize,
        offset,
        order,
        source: string("source")?,
        cwd_prefix: string("cwd_prefix")?,
        since: string("since")?,
        until: string("until")?,
    })
}

fn call_tool(app: &App, name: &str, arguments: &Value) -> Result<Value> {
    match name {
        "search_visible_conversations" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("query is required and must be a string"))?;
            let full = match arguments.get("format") {
                None => false,
                Some(Value::String(value)) if value == "snippet" => false,
                Some(Value::String(value)) if value == "full" => true,
                _ => return Err(anyhow!("format must be 'snippet' or 'full'")),
            };
            let options = search_options(arguments)?;
            let page = app.search_page(query, &options)?;
            let response = page.search;
            // An empty index answers every query with nothing; say so, so the caller does not read
            // "no match" where the truth is "not built yet" (an index is dropped on a schema change).
            if response.hits.is_empty() && app.totals()?.0 == 0 {
                return Ok(text_content(
                    "The local index is empty. Open Visible Conversation Search and choose Build Index; \
                     until then no conversation can match."
                        .into(),
                ));
            }
            let hits = response
                .hits
                .into_iter()
                .map(|hit| McpSearchHit::new(hit, query, full))
                .collect::<Result<Vec<_>>>()?;
            Ok(text_content(serde_json::to_string(&json!({
                "query": response.query,
                "elapsed_ms": response.elapsed_ms,
                "hits": hits,
                "conversations": response.conversations,
                "truncated_at_limit": response.truncated_at_limit,
                "offset": options.offset,
                "has_more": page.has_more,
                "next_offset": page.next_offset,
            }))?))
        }
        "read_conversation_record" => {
            let record_id = arguments
                .get("record_id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("record_id is required"))?;
            let hit = app
                .record(record_id)?
                .ok_or_else(|| anyhow!("record not found: {record_id}"))?;
            Ok(text_content(serde_json::to_string(&McpRecord {
                record_id: record_id.to_string(),
                hit,
            })?))
        }
        "read_conversation_context" => {
            let session = arguments.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
            let timestamp = arguments.get("timestamp").and_then(|v| v.as_str());
            let span = arguments.get("span").and_then(|v| v.as_u64()).unwrap_or(3).clamp(1, 20) as usize;
            let hits = app.context(session, timestamp, span)?;
            Ok(text_content(serde_json::to_string(&hits)?))
        }
        other => Err(anyhow::anyhow!("unknown tool: {other}")),
    }
}

/// Return the same validated search/record payload through the native CLI.
/// MCP's human-readable empty-index notice becomes explicit JSON for command-line callers.
pub fn call_json(app: &App, name: &str, arguments: &Value) -> Result<Value> {
    let result = call_tool(app, name, arguments)?;
    let text = result["content"][0]["text"].as_str().ok_or_else(|| anyhow!("missing tool payload"))?;
    match serde_json::from_str(text) {
        Ok(value) => Ok(value),
        Err(_) => Ok(json!({"index_empty": true, "message": "The local index is empty. Run vcs index with the same --database before searching.", "hits": [], "has_more": false, "next_offset": null})),
    }
}

/// Handle one request. Returns `None` for a notification, which carries no reply.
pub fn handle(app: &App, request: &Value) -> Option<Value> {
    let method = request.get("method").and_then(|value| value.as_str()).unwrap_or("");
    let id = request.get("id").cloned();
    if id.is_none() {
        return None;
    }
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "fast-conversation-search", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let parameters = request.get("params").cloned().unwrap_or(json!({}));
            let name = parameters.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = parameters.get("arguments").cloned().unwrap_or(json!({}));
            call_tool(app, name, &arguments)
        }
        "ping" => Ok(json!({})),
        other => Err(anyhow::anyhow!("unknown method: {other}")),
    };
    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": error.to_string() }
        }),
    })
}

pub fn serve(app: App) -> Result<()> {
    let input = std::io::stdin();
    let mut output = std::io::stdout();
    for line in input.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                let body = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": error.to_string() }
                });
                writeln!(output, "{body}")?;
                output.flush()?;
                continue;
            }
        };
        if let Some(response) = handle(&app, &request) {
            writeln!(output, "{response}")?;
            output.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str) -> App {
        with_records(name, &[sample()])
    }

    fn sample() -> vcs_core::Record {
        vcs_core::Record {
            source: "codex".into(),
            session_id: "s1".into(),
            timestamp: Some("2026-09-01T10:00:00Z".into()),
            role: vcs_core::Role::User,
            cwd: Some("/tmp/project".into()),
            text: "rebuild the trigram path".into(),
            truncated: false,
        }
    }

    fn with_records(name: &str, records: &[vcs_core::Record]) -> App {
        let path = std::env::temp_dir()
            .join(format!("vcs-mcp-{name}-{}", std::process::id()))
            .join("index.sqlite");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        let app = App::new(path);
        let mut index = vcs_core::index::Index::open(app.database()).unwrap();
        index.sync("codex", records).unwrap();
        app
    }

    fn tool(app: &App, name: &str, arguments: Value) -> Value {
        handle(
            app,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}}),
        )
        .unwrap()
    }

    fn body(response: &Value) -> Value {
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    #[test]
    fn native_json_payload_matches_mcp_search_record_and_context() {
        let mut record = sample(); record.text = format!("{}rebuild{}", "prefix ".repeat(150), " suffix".repeat(150));
        let app = with_records("cli-parity", &[record.clone()]);
        let arguments = json!({"query":"rebuild", "format":"snippet", "limit":1, "offset":0,
            "source":"codex", "cwd_prefix":"/tmp/project", "since":"2026-09-01", "until":"2026-09-02", "order":"newest"});
        let mut cli = call_json(&app, "search_visible_conversations", &arguments).unwrap();
        let mut mcp = body(&tool(&app, "search_visible_conversations", arguments));
        cli.as_object_mut().unwrap().remove("elapsed_ms"); mcp.as_object_mut().unwrap().remove("elapsed_ms");
        assert_eq!(cli,mcp); assert_eq!(cli["hits"][0]["snippet_clipped"],true);
        let id = cli["hits"][0]["record_id"].clone();
        let args = json!({"record_id":id});
        assert_eq!(call_json(&app,"read_conversation_record",&args).unwrap(),body(&tool(&app,"read_conversation_record",args)));
        let args = json!({"session_id":"s1", "span":3});
        assert_eq!(call_json(&app,"read_conversation_context",&args).unwrap(),body(&tool(&app,"read_conversation_context",args)));
        let full = call_json(&app,"search_visible_conversations",&json!({"query":"rebuild","format":"full"})).unwrap();
        assert_eq!(full["hits"][0]["text"],record.text);
        let page = call_json(&app,"search_visible_conversations",&json!({"query":"rebuild","offset":1})).unwrap();
        assert_eq!(page["hits"],json!([])); assert_eq!(page["has_more"],false);
    }

    #[test]
    fn native_empty_index_is_explicit_json() {
        let result = call_json(&with_records("cli-empty", &[]), "search_visible_conversations", &json!({"query":"rebuild"})).unwrap();
        assert_eq!(result["index_empty"],true); assert_eq!(result["hits"],json!([]));
    }

    #[test]
    fn initialize_answers_with_the_protocol_version_and_server_name() {
        let response = handle(&app("init"), &json!({"jsonrpc":"2.0","id":1,"method":"initialize"})).unwrap();
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "fast-conversation-search");
    }

    #[test]
    fn only_read_only_tools_are_offered() {
        let response = handle(&app("tools"), &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "search_visible_conversations",
                "read_conversation_record",
                "read_conversation_context"
            ]
        );
    }

    #[test]
    fn a_search_call_returns_the_matching_visible_record() {
        let response = handle(
            &app("search"),
            &json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
                    "params":{"name":"search_visible_conversations","arguments":{"query":"trigram path"}}}),
        )
        .unwrap();
        let body = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(body.contains("rebuild the trigram path"));
    }

    #[test]
    fn default_snippet_finds_a_late_match_and_full_read_recovers_exact_record() {
        let mut original = sample();
        original.text = format!("{}\nKeep This Phrase\t  raw suffix", "prefix ".repeat(1200));
        let app = with_records("late-match", &[original.clone()]);
        let response = tool(
            &app,
            "search_visible_conversations",
            json!({"query":"  keep this phrase  ","limit":1}),
        );
        let result = body(&response);
        let hit = &result["hits"][0];
        let snippet = hit["text"].as_str().unwrap();
        assert_eq!(snippet.chars().count(), SNIPPET_CHARS);
        assert!(snippet.contains("Keep This Phrase"));
        assert!(snippet.ends_with("\t  raw suffix"));
        assert_eq!(hit["record_id"], original.record_id());
        assert_eq!(hit["snippet_clipped"], true);
        assert_eq!(hit["truncated"], false);
        assert_eq!(hit["source"], original.source);
        assert_eq!(hit["session_id"], original.session_id);
        assert_eq!(hit["role"], original.role.as_str());
        assert_eq!(hit["timestamp"], original.timestamp.unwrap());
        assert_eq!(hit["cwd"], original.cwd.unwrap());
        assert_eq!(result["conversations"], 1);
        assert_eq!(result["truncated_at_limit"], false);
        assert!(
            !response["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains('\n'),
            "JSON is compact"
        );

        let recovered = body(&tool(
            &app,
            "read_conversation_record",
            json!({"record_id":hit["record_id"]}),
        ));
        assert_eq!(recovered["record_id"], hit["record_id"]);
        assert_eq!(recovered["text"], original.text);
        assert_eq!(recovered["truncated"], false);
        assert_eq!(
            app.search("keep this phrase", 1).unwrap().hits[0].text,
            original.text,
            "shared search remains full text"
        );
    }

    #[test]
    fn unicode_offsets_slice_original_text_and_use_sqlite_ascii_case_semantics() {
        let mut original = sample();
        original.text = format!(
            "{}É İ incidental\nMiXeD 🦀 query\n{}",
            "前🦀e\u{301}".repeat(300),
            "終🦀".repeat(500)
        );
        let app = with_records("unicode", &[original.clone()]);
        let result = body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":"mixed 🦀 QUERY"}),
        ));
        let hit = &result["hits"][0];
        let start = hit["snippet_start_char"].as_u64().unwrap() as usize;
        let end = hit["snippet_end_char"].as_u64().unwrap() as usize;
        let chars: Vec<char> = original.text.chars().collect();
        assert_eq!(hit["text"], chars[start..end].iter().collect::<String>());
        assert_eq!(hit["total_chars"], chars.len());
        assert_eq!(end - start, SNIPPET_CHARS);
        let match_start = hit["match_start_char"].as_u64().unwrap() as usize;
        let match_end = hit["match_end_char"].as_u64().unwrap() as usize;
        assert_eq!(
            chars[match_start..match_end].iter().collect::<String>(),
            "MiXeD 🦀 query"
        );
        assert!(
            start < match_start && match_end < end,
            "context exists on both sides"
        );
        // SQLite lower() does not fold accented letters or expand İ into multiple code points.
        assert!(body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":"é"})
        ))["hits"]
            .as_array()
            .unwrap()
            .is_empty());
        let exact = body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":"É"}),
        ));
        assert_eq!(exact["hits"].as_array().unwrap().len(), 1);
        let match_start = exact["hits"][0]["match_start_char"].as_u64().unwrap() as usize;
        assert_eq!(chars[match_start], 'É');
    }

    #[test]
    fn full_format_and_index_truncation_are_independent_of_snippet_clipping() {
        let mut original = sample();
        original.text = format!("target\n{}", "rest ".repeat(300));
        original.truncated = true;
        let app = with_records("full-format", &[original.clone()]);
        let snippet = body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":"target"}),
        ));
        assert_eq!(snippet["hits"][0]["truncated"], true);
        assert_eq!(snippet["hits"][0]["snippet_clipped"], true);
        let full = body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":"target","format":"full"}),
        ));
        assert_eq!(full["hits"][0]["text"], original.text);
        assert_eq!(full["hits"][0]["truncated"], true);
        assert_eq!(full["hits"][0]["snippet_clipped"], false);
        assert_eq!(full["hits"][0]["snippet_start_char"], 0);
        assert_eq!(
            full["hits"][0]["snippet_end_char"],
            original.text.chars().count()
        );
        let recovered = body(&tool(
            &app,
            "read_conversation_record",
            json!({"record_id":full["hits"][0]["record_id"]}),
        ));
        assert_eq!(recovered["text"], original.text);
        assert_eq!(recovered["truncated"], true);
    }

    #[test]
    fn a_query_longer_than_the_snippet_budget_keeps_the_whole_match() {
        let query = "query ".repeat(130).trim().to_string();
        let mut original = sample();
        original.text = format!("before {query} after");
        let app = with_records("long-query", &[original]);
        let result = body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":query}),
        ));
        assert_eq!(result["hits"][0]["text"], query);
        assert_eq!(result["hits"][0]["match_start_char"], 7);
    }

    #[test]
    fn replayed_record_ids_retrieve_the_original_stored_timestamp() {
        let original = sample();
        let app = with_records("replayed", &[original.clone()]);
        let mut replay = original.clone();
        replay.timestamp = Some("2026-09-02T10:00:00Z".into());
        vcs_core::index::Index::open(app.database())
            .unwrap()
            .sync("replay", &[replay])
            .unwrap();
        let result = body(&tool(
            &app,
            "search_visible_conversations",
            json!({"query":"trigram"}),
        ));
        assert_eq!(result["hits"].as_array().unwrap().len(), 1);
        assert_eq!(result["hits"][0]["record_id"], original.record_id());
        let recovered = body(&tool(
            &app,
            "read_conversation_record",
            json!({"record_id":result["hits"][0]["record_id"]}),
        ));
        assert_eq!(recovered["timestamp"], original.timestamp.unwrap());
    }

    #[test]
    fn missing_record_and_invalid_format_return_explicit_errors() {
        let app = app("invalid");
        let missing = tool(
            &app,
            "read_conversation_record",
            json!({"record_id":"missing-id"}),
        );
        assert_eq!(missing["error"]["message"], "record not found: missing-id");
        let absent = tool(&app, "read_conversation_record", json!({}));
        assert_eq!(absent["error"]["message"], "record_id is required");
        let invalid = tool(
            &app,
            "search_visible_conversations",
            json!({"query":"trigram","format":"summary"}),
        );
        assert_eq!(
            invalid["error"]["message"],
            "format must be 'snippet' or 'full'"
        );
    }

    #[test]
    fn context_keeps_the_default_span_and_full_visible_text() {
        let records: Vec<vcs_core::Record> = (0..9)
            .map(|position| {
                let mut record = sample();
                record.timestamp = Some(format!("2026-09-01T10:0{position}:00Z"));
                record.text = format!("message {position} {}", "full text ".repeat(100));
                record
            })
            .collect();
        let app = with_records("context-contract", &records);
        let result = body(&tool(
            &app,
            "read_conversation_context",
            json!({"session_id":"s1","timestamp":records[4].timestamp}),
        ));
        assert_eq!(result.as_array().unwrap().len(), 7);
        assert_eq!(result[0]["text"], records[1].text);
        assert_eq!(result[6]["text"], records[7].text);
    }

    #[test]
    fn search_exposes_exact_pagination_and_accepts_all_filters() {
        let records: Vec<_> = (0..4)
            .map(|position| {
                let mut row = sample();
                row.source = "cursor".into();
                row.text = format!("target {position}");
                row.timestamp = Some(format!("2026-09-0{}T10:00:00Z", position + 1));
                row
            })
            .collect();
        let app = with_records("pagination", &records);
        let mut args = json!({"query":"target","source":"cursor","cwd_prefix":"/tmp/project",
            "since":"2026-09-02","until":"2026-09-04","order":"newest","limit":1});
        let first = body(&tool(&app, "search_visible_conversations", args.clone()));
        assert_eq!(first["hits"][0]["text"], "target 2");
        assert_eq!(first["offset"], 0);
        assert_eq!(first["has_more"], true);
        assert_eq!(first["truncated_at_limit"], true);
        assert_eq!(first["next_offset"], 1);
        assert_eq!(first["conversations"], 1);
        args["offset"] = first["next_offset"].clone();
        let last = body(&tool(&app, "search_visible_conversations", args));
        assert_eq!(last["hits"][0]["text"], "target 1");
        assert_eq!(last["offset"], 1);
        assert_eq!(last["has_more"], false);
        assert_eq!(last["truncated_at_limit"], false);
        assert_eq!(last["next_offset"], Value::Null);
    }

    #[test]
    fn malformed_search_options_return_errors_without_panicking() {
        let app = app("bad-search-options");
        for (key, value) in [
            ("offset", json!(-1)),
            ("offset", json!(1.5)),
            ("offset", json!("1")),
            ("offset", json!(u64::MAX)),
            ("order", json!("desc")),
            ("source", json!("unknown")),
            ("source", json!(42)),
            ("cwd_prefix", json!("")),
            ("cwd_prefix", json!(false)),
            ("since", json!("2026-02-29")),
            ("until", json!("2026-04-31")),
            ("since", json!("2026-00-10")),
            ("since", json!("2026-09-01T24:00:00Z")),
            ("since", json!("2026-09-01T00:60:00Z")),
            ("until", json!("2026-09-01T00:00:60Z")),
            ("since", json!("2026-09-01T00:00:00+00:00")),
            ("since", json!("2026-09-01T00:00:00.Z")),
            ("until", json!("2026-09-01T00:00:00.1234567Z")),
            ("since", json!("2026-09-01T00:00:00.abcZ")),
            ("since", json!("你好你好你好你好你好你好")),
            ("since", json!(42)),
            ("limit", json!(0)),
            ("limit", json!(201)),
            ("limit", json!(false)),
            ("query", json!(null)),
        ] {
            let mut args = json!({"query":"trigram"});
            args[key] = value;
            let result = tool(&app, "search_visible_conversations", args.clone());
            assert!(
                result.get("error").is_some(),
                "expected an error for {args}"
            );
        }
        for (since, until) in [
            ("2026-09-02", "2026-09-01"),
            ("2026-09-01", "2026-09-01T00:00:00.000000Z"),
        ] {
            let result = tool(
                &app,
                "search_visible_conversations",
                json!({"query":"trigram","since":since,"until":until}),
            );
            assert_eq!(
                result["error"]["message"],
                "since must be earlier than until"
            );
        }
        assert!(tool(
            &app,
            "search_visible_conversations",
            json!({"query":"trigram","since":"2024-02-29","until":"2026-09-02T00:00:00.1Z"})
        )
        .get("error")
        .is_none());
    }

    #[test]
    fn an_unknown_tool_is_an_error_not_a_silent_empty_result() {
        let response = handle(
            &app("unknown"),
            &json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"delete_everything"}}),
        )
        .unwrap();
        assert!(response["error"]["message"].as_str().unwrap().contains("unknown tool"));
    }

    #[test]
    fn a_notification_gets_no_reply() {
        assert!(handle(&app("notify"), &json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
    }
}

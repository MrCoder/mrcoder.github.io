---
name: search-conversation-history
description: Use when reconstructing decisions or evolving approaches across Codex, Claude Code, and Cursor histories, when the original source is uncertain, or when a question needs multiple related lookups.
---

# Search Conversation History

Use this skill for non-trivial reconstruction across the three local history sources. The `fast-conversation-search` MCP tools are the default path. Indexed acceleration requires a separately installed compatible MCP server; it is not bundled. If no compatible server or configuration is available, use the bundled Python extractor; inspect original records separately when evidence requires content excluded from the visible index.

See [installation and standalone usage](references/install.md) for setup, supported defaults, and dependency limits. Replace `<skill-dir>` in commands with the actual installed directory.

## Choose the mode and scope

A known file or session plus one exact term is a simple lookup. Do **not** use this skill for it: use a targeted `rg` or a small JSON parser, bound the returned matches, and never dump giant raw lines. Use this skill when the source is uncertain, the answer crosses tools or sessions, a decision changed over time, or several related lookups must be reconciled.

Recognize `benchmark` as a mode before ordinary search when it is the first argument: `$search-conversation-history benchmark <question>` or the Claude slash equivalent `/search-conversation-history benchmark <question>`. In benchmark mode, load and follow [references/benchmark.md](references/benchmark.md) before searching. Do not paste the full reference into an ordinary answer, and do not repeat a heavy benchmark automatically for every query.

## Measure each history search

Before refresh or extraction, create a private temporary directory and start the helper with a nonexistent state path; finish after evidence collection, before drafting the answer:

```text
python3 <skill-dir>/scripts/measure_usage.py start --state <temp-dir>/state.json [--session-log <verified current log>]
python3 <skill-dir>/scripts/measure_usage.py finish --state <temp-dir>/state.json
```

Codex auto-discovery searches the default `~/.codex/sessions` and verifies `CODEX_THREAD_ID`; custom `CODEX_HOME` auto-discovery is unsupported, so pass a verified explicit log or report counters unavailable; verify `agent_path` when present. Claude needs an explicit current log. A child's log must belong to that child. Missing or reset counters mean unavailable, never estimated; use only this request's boundary deltas.

End with one concise line in the user's language: elapsed (refresh included when performed), uncached input, cached input, output; include cache writes separately when nonzero. Uncached input includes cache writes; output includes reasoning. State that counters stop at the latest available usage event before the final response and may lag. These are usage counters, not billing dollars.

Add a brief judgement of whether the query suits indexed search, and offer `$search-conversation-history benchmark <same question>` when a comparison would help. Load benchmark details only in that mode.

## Search visible conversation

Use the MCP tools: `search_visible_conversations` (literal substring search returning raw match snippets), `read_conversation_record` (one indexed message by `record_id`), and `read_conversation_context` (surrounding messages for one hit), provided by a separately installed compatible `fast-conversation-search` server configured for your runtime (for example, `~/.claude.json` or `~/.codex/config.toml`). Check that configuration and these tools exist first. If absent, continue with Extract; do not assume a server is preconfigured. They read a persistent, local-only trigram index built from normalized `role: user`/`role: assistant` records; reasoning, tool calls, tool results, system instructions, and source-specific metadata are excluded by contract. Keep this MCP path visible-only.

Before the first MCP query for each history-search request, refresh the shared index once. Read the active runtime config's `mcpServers`/`mcp_servers` entry and use its configured executable and exact `--database` value; do not assume the executable's default database:

```bash
"<configured executable>" index --database "<configured database>"
```

Do not add `--days`: the shared index retains all historical coverage, and fingerprint-based incremental indexing skips unchanged records. Do not refresh before each query or context read. Refresh again only when you need records created after the completed refresh.

Wait for completion and inspect index state and per-source errors; an exit code alone is insufficient. If a refresh fails but leaves an existing index, MCP may still return results: state the freshness limitation and do not conclude "no evidence" from stale results.

Search snippets first; expand relevant hits by `record_id` before relying on a decision, qualifier, or contradiction that the snippet may omit. Read surrounding context only when needed, normally with `span: 1` and the hit’s timestamp. `truncated` describes stored-text truncation; snippet clipping is separate. If the indexed full record is truncated and its missing tail matters, read the original record through Evidence fallback. Use `format: "full"` only when the whole search result is needed.

For bounded questions, use relevant `source`, `cwd_prefix`, `since` (inclusive), and `until` (exclusive) filters; do not invent scope restrictions. Choose `order: "newest"` for current decisions or `"oldest"` for origins. When `has_more` is true, continue with `next_offset` as `offset`, preserving query, filters, and order. An unexamined page is not evidence of absence. Do not refresh between pages; if the index changes, restart pagination.

Within one history-search request, keep a set of expanded `record_id` values. Reuse already-read records; fetch again only for fresh data, and reconsider them when new context changes their meaning. If the runtime supports local batching (for example, Codex `functions.exec`), suppress repeated `record_id`s before emitting results to the model while preserving each query's `has_more`, `next_offset`, and provenance; otherwise limit overlapping searches and avoid duplicate full reads.

Do not merge distinct records because snippets match or text is semantically similar. Identical fully-read text may be presented once with all source, timestamp, and session references retained, while distinguishing user from assistant. The index already deduplicates by source/session/role/text.

## Extract

Create a disposable output directory and run:

```bash
python3 <skill-dir>/scripts/extract_history.py \
  --days 30 --output "$OUTPUT_DIR"
```

Read `$OUTPUT_DIR/summary.json`. Continue when a source is missing or unavailable, but report that limitation. Never modify a history store.

If the 30-day evidence is absent, weak, or materially incomplete, rerun with `--all` into a new directory. Do not expand merely because one source is unavailable when the available sources answer the question conclusively.

## Reduce candidates before analysis

Derive specific literal terms and synonyms. Search all three sources locally through the index or normalized source files, then return only candidate metadata (`source`, timestamp, session ID, cwd) and match-centered snippets. Expand relevant records or neighbors as needed; do not read whole source files by default.

Do not launch one agent per source by default: all three sources are searched locally. Use the cheapest capable worker only for independent, complex candidate subsets, giving it only the question and candidate evidence, never a whole source/history or the full parent conversation. Preserve direct-match and conceptual-match labels, timestamps, session IDs, and `cwd`; report unavailable sources. Expand to all history only when the evidence warrants it.

MCP is primary when connected. If its connector is disconnected but the configured executable works, use that executable's CLI or stdio MCP against the same configured database; extraction is only for when the index path is unavailable. Verify the advertised tool schemas; if a connected server predates snippet/pagination tools, use the configured stdio MCP. If refresh fails with a stale index, report the freshness limit and do not claim absence.

## Evidence fallback

The visible index is the default product mode, not a transparent replacement for raw `rg`.

- Use visible results for past discussion, intent, decisions, and user-visible conclusions.
- Re-read source JSONL or use raw `rg` only when a request materially depends on tool output, execution evidence, file paths, raw line numbers, command behavior, or strict original-record comparison.
- State clearly when a conclusion comes from visible conversation versus raw evidence.

Raw lookups should target an identified session, time range, and source first. Parse, filter, and aggregate locally before returning model output: never dump whole JSONL files or unbounded `rg` lines. Return only relevant facts, counts/status, and minimal raw samples with file/line/time/session citations; state incomplete source or scope, and expand additional raw evidence only when the question requires it.

For token usage, treat cumulative counters such as `Codex total_token_usage` as the latest session snapshot or as boundary differences for an interval; never sum snapshots. Keep cached and uncached input separate, account for output/reasoning according to source semantics without double counting, exclude duplicate or replayed sessions, and treat counters as usage rather than provider billing dollars.

## Reconcile

Synthesize the source results in the main agent:

- distinguish the user's words from assistant statements and quoted third-party text;
- deduplicate copied prompts or sessions found in multiple tools;
- prefer direct dated evidence over recollection or inference;
- show only the minimum excerpt required to support the answer;
- state uncertainty and competing interpretations instead of forcing a conclusion;
- cite source, timestamp, and session ID for material claims;
- mention missing or unrecognized stores.

Keep extracted files outside repositories and remove the disposable directory after answering when safe to do so.

## Script options

Use `--codex-root`, `--claude-root`, `--cursor-root`, and `--cursor-agent-root` only for tests or nonstandard installations. Cursor extraction covers both IDE Composer and Agent Host histories. Preserve the default `--max-chars` indexing coverage; limit returned snippets instead, and use raw-tail fallback when a record is genuinely truncated.

Run fixture tests after changing the extractor:

```bash
python3 <skill-dir>/scripts/test_extract_history.py
```

---
name: search-conversation-history
description: Search local Codex, Claude Code, and Cursor conversation histories for prior decisions, approaches, incidents, commands, or context. Default to visible user and assistant conversation text; retrieve raw records only when strict evidence requires it.
---

# Search Conversation History

Search all three local history sources. The `fast-conversation-search` MCP tools are the default path. The extractor and per-source workers below are the fallback for when that tool is unavailable, its index is stale, or a request needs raw records the visible index excludes by contract.

## Search visible conversation

Call the MCP tools first: `search_visible_conversations` (literal substring search) and `read_conversation_context` (surrounding messages for one hit), served by the `fast-conversation-search` server registered in both `~/.claude.json` and `~/.codex/config.toml`. They read a persistent, local-only trigram index built from normalized `role: user`/`role: assistant` records; reasoning, tool calls, tool results, system instructions, and source-specific metadata are excluded by contract. No setup needed when the server answers a query.

If a call reports the index is empty, or the server is not connected in this session, rebuild it with the same binary, at the exact `--database` path that server's own config passes it (read that path from the config rather than assuming the binary's own default — they differ):

```bash
vcs index --database "<the --database value from that mcpServers/mcp_servers entry>" --days 30
```

Then retry the MCP call. Only fall back to Extract below if `vcs` itself is unavailable.

## Extract

Create a disposable output directory and run:

```bash
python3 ~/.claude/skills/search-conversation-history/scripts/extract_history.py \
  --days 30 --output "$OUTPUT_DIR"
```

Read `$OUTPUT_DIR/summary.json`. Continue when a source is missing or unavailable, but report that limitation. Never modify a history store.

If the 30-day evidence is absent, weak, or materially incomplete, rerun with `--all` into a new directory. Do not expand merely because one source is unavailable when the available sources answer the question conclusively.

## Search in parallel

Derive broad literal terms and semantic concepts from the user's question. Search `codex.jsonl`, `claude.jsonl`, and `cursor.jsonl` concurrently when workers are available.

- In Claude Code, launch one Agent worker per source with `model: haiku`.
- In Codex, spawn one `luna_worker` per source. If that role is unavailable, use the cheapest available capable model.
- Give each worker only the user's question and its one normalized source file. Do not pass conclusions from another worker.
- If delegation or parallel execution is unavailable, search the three files sequentially in the main agent.

Ask every worker to return:

1. Direct matches with timestamp, session ID, working directory, and a short excerpt.
2. Conceptual or paraphrased matches, explicitly labelled as inference.
3. A source-level conclusion.
4. Whether an all-history expansion is warranted and why.

Use the visible index first to reduce large files before asking a worker to inspect them. Search synonyms and domain terms, not only the user's exact wording.

## Evidence fallback

The visible index is the default product mode, not a transparent replacement for raw `rg`.

- Use visible results for past discussion, intent, decisions, and user-visible conclusions.
- Re-read source JSONL or use raw `rg` only when a request materially depends on tool output, execution evidence, file paths, raw line numbers, command behavior, or strict original-record comparison.
- State clearly when a conclusion comes from visible conversation versus raw evidence.

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

Use `--codex-root`, `--claude-root`, `--cursor-root`, and `--cursor-agent-root` only for tests or nonstandard installations. Cursor extraction covers both IDE Composer and Agent Host histories. Use `--max-chars` to lower the per-record text cap when histories are unusually large.

Run fixture tests after changing the extractor:

```bash
python3 ~/.claude/skills/search-conversation-history/scripts/test_extract_history.py
```

---
name: search-conversation-history
description: Search local Codex, Claude Code, and Cursor conversation histories for prior decisions, approaches, incidents, commands, or context. Use when the user asks what was discussed, decided, tried, fixed, or learned in previous AI conversations, including requests such as "check conversation history", "how did we handle this before?", or "find the earlier discussion about X".
---

# Search Conversation History

Search all three local history sources. Use scripts for extraction and AI only for semantic matching and synthesis.

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

Use `rg -i` first to reduce large files before asking a worker to inspect them. Search synonyms and domain terms, not only the user's exact wording.

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

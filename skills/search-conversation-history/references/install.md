# Install and use

The ZIP contains the skill, Python helpers, fixture tests, and references. Python 3.10 or newer is required. The helpers use Python's standard library; no runtime pip dependencies are needed. The Rust search engine and MCP server are not bundled.

Download `search-conversation-history.zip` and its `.sha256` file from this site's download links. From the directory containing both files, verify and install for Claude Code:

```bash
shasum -a 256 -c search-conversation-history.zip.sha256
mkdir -p "$HOME/.claude/skills"
unzip search-conversation-history.zip -d "$HOME/.claude/skills"
```

For Codex, use this destination instead:

```bash
mkdir -p "$HOME/.codex/skills"
unzip search-conversation-history.zip -d "$HOME/.codex/skills"
```

Back up an existing skill directory before replacing it. Restart the agent session if the new skill is not discovered. Normal invocation:

```text
/search-conversation-history <question>       # Claude Code
$search-conversation-history <question>       # Codex
```

Benchmark invocation (loads the benchmark reference):

```text
/search-conversation-history benchmark <question>
$search-conversation-history benchmark <question>
```

## Standalone Python extraction

This path works without an MCP server. Set `SKILL_DIR` to your installation and use a disposable private output directory outside any repository:

```bash
SKILL_DIR="$HOME/.codex/skills/search-conversation-history"
OUTPUT_DIR="$(mktemp -d)"
python3 "$SKILL_DIR/scripts/extract_history.py" --days 30 --output "$OUTPUT_DIR"
```

Inspect `summary.json` first, then search the normalized `codex.jsonl`, `claude.jsonl`, and `cursor.jsonl` with bounded local parsing. Use `--all` when the question needs older evidence. These output files contain private history; do not publish them. Remove the disposable directory after use when safe.

Defaults are `~/.codex`, `~/.claude`, macOS Cursor IDE `~/Library/Application Support/Cursor`, and Cursor Agent Host `~/.cursor/chats`. Linux and Windows Cursor IDE paths are not detected automatically. Pass `--cursor-root <actual Cursor application-data directory>` for those systems or any nonstandard setup. Other overrides are `--codex-root`, `--claude-root`, and `--cursor-agent-root`. Missing or unrecognized stores are reported in the summary; the remaining sources can still be used.

## Optional indexed acceleration

Indexed mode requires a separately installed compatible `fast-conversation-search` MCP server exposing `search_visible_conversations`, `read_conversation_record`, and `read_conversation_context`. It also requires a configured executable supporting `index --database <configured database>`. This distribution does not provide the Rust engine, an installer, or a download location for it. Do not assume it is available or use an invented installation command.

Check the active runtime's MCP configuration and tool schemas. Refresh once with its exact configured executable and database, following the main skill. If compatible configuration is absent, use Python extraction. If a configured connector is disconnected but its executable works, the main skill permits that executable's compatible CLI or stdio MCP path.

## Usage measurement and tests

The measurement helper discovers Codex logs only under the default `~/.codex/sessions`, verifying `CODEX_THREAD_ID`. Custom `CODEX_HOME` auto-discovery is unsupported; provide `--session-log <verified current log>` or report unavailable counters. Claude requires an explicit verified current log. Missing or reset counters are unavailable, never estimates. See the main skill for request-boundary measurement and the benchmark reference for comparison rules.

```bash
python3 "$SKILL_DIR/scripts/extract_history.py" --help
python3 "$SKILL_DIR/scripts/measure_usage.py" --help
python3 "$SKILL_DIR/scripts/test_extract_history.py"
python3 "$SKILL_DIR/scripts/test_measure_usage.py"
```

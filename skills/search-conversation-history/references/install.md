# Install search-conversation-history

The ZIP includes the skill, a local history CLI, Apple Silicon executable, Rust source, Python fallback, usage measurement, tests, and license notices. Indexing and search stay on your machine; no API key or model call is needed.

## Install

Download [the ZIP](https://mrcoder.github.io/downloads/search-conversation-history.zip) and [its SHA-256 checksum](https://mrcoder.github.io/downloads/search-conversation-history.zip.sha256). Verify the checksum before extracting:

```bash
shasum -a 256 -c search-conversation-history.zip.sha256
unzip search-conversation-history.zip -d ~/.claude/skills
python3 ~/.claude/skills/search-conversation-history/scripts/history.py index
```

For Codex, extract to `~/.codex/skills` instead. For other agents, use their skill discovery directory. Keep the extracted directory: it contains the executable and scripts. No client configuration changes or restart are needed for CLI use; the agent needs shell execution access.

Requirements:

- Python 3.11+ for the launcher and usage helper.
- Apple Silicon macOS: the bundled native executable runs directly; Rust is unnecessary. This is the verified platform.
- Intel macOS, Linux, and Windows: the launcher builds the included source on first use. Install [Rust/Cargo](https://www.rust-lang.org/tools/install) and a C compiler first. These platforms have not been verified for this release. Cargo may download build dependencies; history data is not sent to Cargo.

The launcher verifies the bundled binary checksum before executing it. Other platforms build into `~/.cache/conversation-history/native`. Build errors remain visible; the Python extractor is available if native setup is blocked.

## Use the CLI

Replace `<skill-dir>` with the installed directory:

```bash
python3 <skill-dir>/scripts/history.py index
python3 <skill-dir>/scripts/history.py search "literal phrase" --limit 20 --source claude --order newest
python3 <skill-dir>/scripts/history.py search "literal phrase" --since 2026-01-01 --until 2026-02-01 --offset 20 --limit 20
python3 <skill-dir>/scripts/history.py record "<record_id>"
python3 <skill-dir>/scripts/history.py context --session "<session_id>" --timestamp "<timestamp>" --span 1
```

Output is JSON. Search defaults to match snippets; `--format full` returns full indexed messages. Search supports `--source`, `--cwd-prefix`, `--since`, `--until`, `--order`, `--limit`, and `--offset`. Follow `next_offset` while `has_more` is true, preserving query and filters. Expand selected hits with `record`, then read context when needed. `index_empty: true` means an index has not been built, not that the topic was never discussed.

The default database is `~/.local/share/visible-conversation-search/index.sqlite`. Add `--database <absolute-path>` to each command for a separate index. The launcher creates a private index file. History stores are read-only. Refresh once per request without `--days`; fingerprints skip unchanged files. First build time depends on history size. Inspect source warnings: partial or stale coverage cannot establish absence.

Defaults cover `~/.codex/sessions`, `~/.claude/projects`, Cursor IDE's macOS Application Support store, and `~/.cursor/projects`. Cursor IDE store detection is currently macOS-specific. Nonstandard roots can be read through the Python fallback options below.

Invoke the skill normally with `$search-conversation-history <question>` (Claude also accepts `/search-conversation-history <question>`). Use a targeted `rg` or JSON parser for a known file and one exact term. For a controlled workflow comparison, explicitly invoke `$search-conversation-history benchmark <question>`; see [benchmark.md](benchmark.md).

## Optional MCP

MCP is optional. The bundled executable also supports:

```bash
<skill-dir>/server/bin/darwin-arm64/vcs mcp --database <absolute-path>
```

On other platforms, use the built `vcs` (`vcs.exe` on Windows) under `~/.cache/conversation-history/native/release`. Add that executable and the arguments `mcp`, `--database`, and the absolute index path to your client's MCP configuration only if you want MCP integration. The server exposes `search_visible_conversations`, `read_conversation_record`, and `read_conversation_context`. Index freshness still requires one `index` command per request. CLI use does not require any of this configuration.

## Python fallback and usage

```bash
python3 <skill-dir>/scripts/extract_history.py --days 30 --output <private-temp-dir>
# If necessary, widen into a new directory:
python3 <skill-dir>/scripts/extract_history.py --all --output <another-private-temp-dir>
```

Read `summary.json`, then locally filter normalized records before returning snippets. Custom roots: `--codex-root`, `--claude-root`, `--cursor-root`, `--cursor-agent-root`. Missing or unrecognized stores must be reported. Do not change history files.

The skill reports elapsed time and request-boundary usage counters. Codex auto-discovery verifies the current thread in default session stores. Claude requires an explicit current log. Other runtimes or inaccessible counters report unavailable. These are usage counts, not billing dollars; final response generation is outside the measurement boundary.

## Verify and inspect

```bash
python3 <skill-dir>/scripts/test_history.py
python3 <skill-dir>/scripts/test_extract_history.py
python3 <skill-dir>/scripts/test_measure_usage.py
```

`server/provenance.json` records source and executable checksums. `server/LICENSE` and `server/THIRD_PARTY_NOTICES.md` contain the project and dependency license notices. Source-only CLI workspace builds with `cargo build --release --locked --manifest-path <skill-dir>/server/Cargo.toml`; no desktop application is included.

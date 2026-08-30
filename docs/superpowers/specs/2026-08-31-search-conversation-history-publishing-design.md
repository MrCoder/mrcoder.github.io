# Publish `search-conversation-history`

Date: 2026-08-31

## Goal

Publish the local `search-conversation-history` skill as the sixth installable skill on `mrcoder.github.io`, without publishing any conversation content or machine-specific data.

The result should fit the existing Field Guide, remain useful to both Claude Code and Codex users, and install every runtime file the skill needs.

## Audience and promise

The page is for developers who use more than one AI coding tool and need to recover a prior decision, command, incident, or approach.

The public promise is deliberately narrow:

- search visible user and assistant conversation text from Codex, Claude Code, and Cursor;
- read the original stores without modifying them;
- start with the last 30 days and expand to all history only when recent evidence is insufficient;
- keep each source's evidence separate until the main agent reconciles it;
- report timestamps, session IDs, working directories, excerpts, missing sources, and uncertainty.

It is not a persistent index, hosted search service, raw-transcript exporter, or guarantee that every historical storage format will remain supported.

## Publication structure

Copy the local package into the site without changing its operational behavior:

```text
skills/search-conversation-history/
├── SKILL.md
├── agents/
│   └── openai.yaml
└── scripts/
    ├── extract_history.py
    └── test_extract_history.py
```

All four files are public source artifacts. The normal installation command downloads `SKILL.md`, `agents/openai.yaml`, and `scripts/extract_history.py`. The test file remains available for maintainers and users who want to validate the extractor, but is not required at runtime.

No generated JSONL, `summary.json`, temporary directory, source database, transcript excerpt, username, absolute home path, customer identifier, credential, or secret is copied into the repository.

## Field Guide integration

The existing page remains a single static page with the same typography, colours, and survey-field-guide visual language.

Update the document title, description, hero copy, and install summary from five skills to six. Keep the existing pre/during/post transect for lifecycle skills. Present `search-conversation-history` as `recall · anytime`, because it can be invoked before, during, or after implementation and does not belong to one stage of the transect.

Add its entry before the lifecycle entries. The entry contains:

- a concise description centred on recovering dated evidence from three local AI histories;
- example triggers such as “search conversation history”, “how did we handle this before?”, and “find the earlier discussion about X”;
- a copyable installation command that creates the nested directories and downloads the three runtime files;
- a short privacy statement that the search is local and disposable extraction output is not uploaded.

The global install block continues to install the five single-file skills in its current loop, then installs the three runtime files for `search-conversation-history`. Its caption and accessible labels state that all six skills are installed.

## Installation behavior

The installation command uses only `mkdir` and `curl`, matching the existing site. It writes to `~/.claude/skills/search-conversation-history` because that is the skill's canonical local location. The same `SKILL.md` contains Codex-specific delegation instructions, while `agents/openai.yaml` supplies OpenAI-facing metadata.

Each download uses `curl -fLsS` so an HTTP error fails visibly instead of creating an empty or error-page file. Commands remain readable and directly copyable; the page does not introduce a remote shell installer.

## Error handling and compatibility

The published extractor retains its current fail-closed behavior:

- missing stores are reported and do not stop searches of available stores;
- an unrecognized Cursor schema produces no Cursor records and reports the schema problem;
- malformed records are skipped and recorded as extraction errors;
- source histories are opened read-only where applicable and are never rewritten;
- default paths reflect macOS, while explicit root options support tests and nonstandard installations.

The Field Guide does not claim Windows or Linux Cursor auto-discovery because the current default Cursor path is macOS-specific.

## Verification

Before publication:

1. Run `scripts/test_extract_history.py`; all four fixture tests must pass.
2. Scan the copied package for home-directory usernames, customer identifiers, credentials, and generated history data.
3. Serve the site locally and verify the updated copy, all-six install command, per-skill command, responsive layout, and copy buttons.
4. Verify each published file can be fetched from the local server at its final URL path.

After pushing `master`:

1. Confirm the GitHub Pages homepage contains `search-conversation-history`.
2. Fetch every runtime URL with HTTP failure enabled and compare it with the committed file.
3. Verify the page renders successfully in a browser at desktop and narrow viewport widths.

Publication is complete only after the live page and runtime downloads are verified.

## Git and release

The target repository is clean and publishes from `master`. Commit the specification separately. After specification approval, implement the package and page changes in a second commit, push `master` to `origin`, and wait for GitHub Pages to serve the new commit.


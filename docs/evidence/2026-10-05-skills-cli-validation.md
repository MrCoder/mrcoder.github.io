# AI Hero installation methods

Date: 5 October 2026, Australia/Melbourne. This revision follows the user's request to adopt AI Hero's installation methods completely. The website remains a local preview; no remote release or repository setting has changed.

## Current channels

The main channel uses the official skills CLI, as on [AI Hero](https://www.aihero.dev/skills): `npx skills@latest add https://github.com/MrCoder/mrcoder.github.io/tree/master`, with `npx skills update` for updates. The branch is explicit because the repository's default gh-pages branch contains the old blog. Individual commands select the actual skill names and preserve no-bs/renhua and overnight/unattended companion selections. Scope defaults to the project, with agent/scope choices handled by the CLI. The old custom Bash installer and its unpublished `/install.sh` endpoint are removed.

The second channel is the managed Claude Code plugin `mrcoder-skills@mrcoder`, registered from the MrCoder marketplace on master. Its manifests use the canonical root skills directory. It is our own marketplace, with documented registration and update steps and namespaced skill invocations. It is not claimed to be in Anthropic's official directory or to update automatically. Details and official references are in `../plans/installation-channels.md`.

## Execution evidence

- Official npm CLI entrypoint: `npx --yes skills@latest --version` reports 1.7.0; its remote `add <master source> --list` discovers twelve directory names (eleven skills and the alias).
- Real remote CLI copy installs for Claude Code and Codex into temporary projects preserve all fifty-nine skill files, byte-for-byte, including executable flags. Individual alias/dependency selections also pass. Personal skill directories and HOME were untouched. Verification uses the documented telemetry opt-out.
- Claude Code 2.1.289 strict validation of plugin and marketplace metadata reports no errors or warnings. Isolated local marketplace registration and installation succeeds; plugin details lists all twelve skills, and its managed cache contains the same fifty-nine files and executable flags.
- The Git registration syntax explicitly selects master. Public remote registration currently reports `manifest_missing`, since these manifests have not been published. Local validation is not presented as successful remote publication.
- `npm test`: thirteen tests pass, zero skipped. The official CLI is pinned at 1.7.0 as a development dependency so local copy integration runs in the default test suite and CI. Node 22.20.0 or later is required.
- `npm run check`: eighteen files, zero errors, warnings or hints. Build generates sixteen HTML pages and verifies all sixty-six original files, both ZIP checksums, internal URLs/fragments, canonical routes and aliases. It validates seventy-three skills CLI command choices, four plugin commands and twelve plugin skill directories. The preservation baseline is unchanged.

## Browser checks

The production preview shows the two source-like installation channels. Main CLI copying succeeds. The marketplace setup link reaches `/install/#claude-plugin`; copying registration preserves the quoted Git URL. Selecting overnight and Codex emits both overnight and unattended with `--agent codex`. Homepage, install guide and long history detail page have no document overflow at 320px; commands scroll within their own rows. Recommendation remains pitch-me.

Current installation screenshot: `../previews/aihero-installation.png`. Earlier Lighthouse and performance evidence describes the earlier preview; no new audit score is claimed for this revision. The plugin manifests and website still need publication before the remote plugin channel can be used.

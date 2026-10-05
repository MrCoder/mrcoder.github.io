# Installation channels and local verification

Date: 5 October 2026. Current approved installation follows AI Hero's two channels: the official skills CLI first, and Claude Code plugin installation second. The bespoke curl/Bash installer prototype is historical, and `/install.sh` is removed.

## Skills CLI

The canonical public source is `https://github.com/MrCoder/mrcoder.github.io/tree/master`. The explicit branch is required because GitHub's default branch is `gh-pages`, while the canonical skills live on `master`.

```sh
npx skills@latest add https://github.com/MrCoder/mrcoder.github.io/tree/master
npx skills update
```

Individual selections use `--skill`; Claude Code uses `--agent claude-code` and Codex uses `--agent codex`. Select no-bs and renhua together, and overnight and unattended together. The generic command offers interactive skill, agent and scope selection. Node.js 22.20 or newer, npm, Git and network access are required. The official CLI owns installation and updates; no replacement-backup guarantee is made by this site.

The installation worker verified skills CLI 1.7.0 against the real public `master` source in temporary projects. Both agents' copy installations preserved all 59 files and executable modes. Individual alias/dependency selections preserved their complete package files. Root also verified remote discovery of all twelve directory names. These checks do not invoke a model or claim history-index setup is complete.

## Claude Code plugin

The marketplace is `mrcoder`; its one plugin is `mrcoder-skills`. The marketplace's `source: "./"` uses the repository root. The plugin uses Claude's default root `skills/` discovery, so it does not duplicate skills or add redundant custom paths. The plugin contains no hooks, agents, MCP or LSP services.

Remote commands, **after the manifests are published to master**:

```sh
claude plugin marketplace add 'https://github.com/MrCoder/mrcoder.github.io.git#master'
claude plugin install mrcoder-skills@mrcoder
```

The `#master` Git source suffix is supported by the [official marketplace source reference](https://code.claude.com/docs/en/plugins/marketplace-reference). The installed Claude CLI has no `--ref` option. An isolated Git wrapper observed the actual marketplace command request an explicit `master` branch argument for its clone; the wrapper recorded only that boolean, not arbitrary command arguments.

Use namespaced skills, for example `/mrcoder-skills:diagram`. Manual updates:

```sh
claude plugin marketplace update mrcoder
claude plugin update mrcoder-skills@mrcoder
```

The commands are from the [official plugin CLI reference](https://code.claude.com/docs/en/plugins/cli-reference). This is a third-party marketplace, not an Anthropic marketplace. Auto-updates depend on the user's configuration; the site does not promise them. Bump the plugin version for future releases. The [manifest reference](https://code.claude.com/docs/en/plugins/manifest-reference) documents default root skill discovery.

## Evidence and publication boundary

Claude Code 2.1.289, with isolated configuration directories under `/private/tmp`:

- `CLAUDE_CONFIG_DIR=/private/tmp/mrcoder-plugin-validation claude plugin validate .claude-plugin/plugin.json --strict --json`: successful, no errors or warnings.
- Same isolated configuration, `claude plugin validate . --strict --json`: successful marketplace validation, no errors or warnings.
- `CLAUDE_CONFIG_DIR=/private/tmp/mrcoder-plugin-local-test claude plugin marketplace add /Users/pengxiao/workspaces/mrcoder.github.io --scope user --json`: successful registration.
- Same isolated configuration, `claude plugin install mrcoder-skills@mrcoder --scope user --json`: successful installation.
- Same isolated configuration, `claude plugin details mrcoder-skills`: twelve skills, including renhua; no agents, hooks, MCP or LSP components.
- Installed cache at `/private/tmp/mrcoder-plugin-local-test/plugins/cache/mrcoder/mrcoder-skills/1.0.0`: all 59 canonical skill files matched exact bytes and executable flags.
- Public remote registration of the Git URL with `#master`: `manifest_missing`, expected because `.claude-plugin` remains local. A subsequent branch-argument observation encountered the sandbox's DNS restriction, without changing this publication finding.

No user Claude settings were changed, and no model or paid API was invoked. The manifests have not been pushed, deployed or submitted to any external marketplace. Remote plugin installation must be checked again after an accepted preview and an authorized publication.

The build verifies the independent command contract, explicit branch, aliases/dependencies, exact plugin strings and canonical plugin metadata. Nine publishing tests include rejection of stale Bash endpoints, unpinned source commands, missing companion skills, redirected plugin sources and duplicate skill paths. The original 66-file preservation fixture remains unchanged.

# MrCoder Skills

Practical skills for work with coding agents, by Peng Xiao. This project was previously presented as Field Guide. The site adapts the AI Hero fork’s UI in Astro static output and keeps the existing `https://mrcoder.github.io` hostname.

## Develop and review

Use Node 22.20 or later for both the website toolchain and the official skills CLI. From the repository root:

```sh
npm ci
npm run dev
```

The development server binds to localhost. To review the exact publishing output:

```sh
npm run check
npm test
npm run build
npm run preview
```

The build runs Astro, copies tracked raw packages/downloads/assets into `dist/`, generates `sitemap.xml` and `robots.txt`, then verifies compatibility. The preview uses that verified output.

## Maintain content

- `src/data/skills.ts` defines the eleven skills, three main groups, quieter optional practices, related links, examples, requirements, and human documentation.
- `src/data/install.ts` generates official `npx skills@latest` commands and the Claude Code plugin commands. The source explicitly selects `master`, because this repository defaults to `gh-pages`. Individual commands include the renhua companion for no-bs and the unattended core for overnight.
- `.claude-plugin/plugin.json` and `marketplace.json` expose the existing root `skills/` as one Claude Code plugin named `mrcoder-skills` in the `mrcoder` marketplace. Do not duplicate packages or add redundant skill paths.
- `src/pages/`, `src/layouts/`, `src/components/`, and `src/styles/` define the website. `public/` holds website assets. Record the upstream source and preserve relevant notices when adapting files.
- `skills/` remains the canonical agent package source. `downloads/` contains released ZIP packages and checksums. Human documentation must agree with those sources.

The build publishes only Git-tracked files from `skills/`, `downloads/`, and `assets/`, plus `.nojekyll`. It never recursively copies untracked scratch files or caches. Add intentionally released files to Git before building. Do not create a second package tree under `public/skills`.

The raw file URLs and homepage skill anchors remain stable. `/skills/renhua/` explains the alias and links to no-bs; renhua does not count as a separate skill. The official skills CLI manages package installation and `npx skills update`; this site does not promise replacement backups. Installing the history package and indexing local history are separate actions.

## Compatibility checks

`tests/fixtures/published-files.json` records SHA-256 hashes for all 66 package, download, and asset files that existed before the redesign. `published-paths.txt` records the matching paths. Build verification compares both source and output to this fixed baseline, so modifying both cannot silently bypass preservation.

A deliberate package release needs its own review, regenerated ZIP/checksum where applicable, and an explained baseline update. Do not refresh the fixture merely to make a failed build pass. Keep website changes separate from package releases.

Tests check the immutable publishing boundary, actual CLI command generation, complete skill package files and executable permissions, alias/dependency selections, and plugin metadata. Installer tests use temporary destinations; real remote CLI and isolated local plugin evidence is recorded separately. Installing the history package does not initialise its local index.

Build verification checks both ZIP checksums, all fifteen canonical routes, eleven old homepage anchors, the alias link, static 404, sitemap, local HTML links/fragments, exact official CLI commands, and canonical Claude plugin metadata. The earlier bespoke `/install.sh` prototype and its curl/Bash evidence are superseded; that endpoint is removed.

## Installation channels

The primary channel works against the existing public `master` skill source:

```sh
npx skills@latest add https://github.com/MrCoder/mrcoder.github.io/tree/master
```

The CLI offers agent and skill selection. Individual command rows include explicit companion skills where required. Choose the complete set or use the tested per-skill commands in the install guide. Run `npx skills update` from the installed project for CLI-managed updates.

Claude Code has a second channel. These commands were verified against the published `master` manifests on 5 October 2026:

```sh
claude plugin marketplace add 'https://github.com/MrCoder/mrcoder.github.io.git#master'
claude plugin install mrcoder-skills@mrcoder
```

The plugin's skills use namespaced invocations such as `/mrcoder-skills:diagram`. Manual plugin updates use `claude plugin marketplace update mrcoder`, then `claude plugin update mrcoder-skills@mrcoder`. Third-party marketplace auto-updates are a user setting; no automatic update promise is made here. This is our own marketplace, not an Anthropic marketplace.

Both manifests passed strict validation with Claude Code 2.1.289. An isolated local marketplace install discovered twelve directories (eleven skills plus renhua); all 59 package files and executable bits matched the canonical source. Remote registration and plugin installation from `master` subsequently succeeded in isolated configuration; all twelve entries and 59 package files matched the canonical source, including executable flags. See [installation channel evidence](docs/plans/installation-channels.md).

## Release gate and hosting

Released on 5 October 2026 from [`3d69549`](https://github.com/MrCoder/mrcoder.github.io/commit/3d6954978cb90bb62e3d778b950091ffe83216d8). The Pages workflow completed successfully, and public HTTP checks verified all 66 preserved files by SHA-256, sixteen site HTML pages, the package HTML template, sitemap, robots rules and expected 404 responses. Both remote installation channels passed. See [release evidence](docs/evidence/2026-10-05-release.md).

Pre-release research found that GitHub Pages publishes `master` using the legacy Pages source, while the repository default branch is `gh-pages`. Verify these settings through the GitHub API before deployment. The workflow in `.github/workflows/pages.yml` targets the publishing branch, `master`, and validates pull requests without deploying them. A push or manual run on `master` builds and publishes a Pages artifact; `configure-pages` has automatic enablement disabled.

**The user accepted the local preview and explicitly authorized release on 5 October 2026.** The preview and compatibility evidence support that acceptance. Release authorization permits coordinating the Pages source switch to GitHub Actions and publishing to `master`; it does not establish that deployment has completed.

Confirm the intended `master` commit and record the current Pages settings through the GitHub API. Enable the workflow-based Pages source, publish, and verify the deployment result through the API. Smoke-check the homepage, documentation pages, raw files, downloads/checksums, and analytics event fields. Retain the previous commit and recorded Pages settings for rollback. Do not select `gh-pages` simply because it is the repository default.

## Provenance and scope

The user explicitly chose to fork AI Hero and follow its design closely, superseding the earlier independent-design plan. The real fork is [MrCoder/ai-hero](https://github.com/MrCoder/ai-hero), whose parent is [badass-courses/ai-hero](https://github.com/badass-courses/ai-hero). The UI adaptation uses source pinned to [7a9fca045999c74112a13f26581872dda04e1b9e](https://github.com/MrCoder/ai-hero/tree/7a9fca045999c74112a13f26581872dda04e1b9e). See the [upstream source map](docs/plans/upstream-source-map.md) for component provenance.

Keep attribution and any upstream notices when adapting source. No repository-wide reuse license was found in the inspected upstream tree; the upstream page’s MIT label refers to its skills rather than licensing the whole website. The fork and user’s implementation choice are recorded without inventing a blanket MIT declaration. Package-specific notices remain with their packages.

The adaptation uses our owned skills and content. AI Hero’s email course, social proof, auth, commerce and workflow services are not part of this static site. Earlier preview evidence remains historical; the user accepted the revised local preview on 5 October 2026.

See the [approved research and plan](docs/plans/2026-10-05-redesign-research-and-plan.md) and [historical blog URL inventory](docs/plans/legacy-url-inventory.md). Historical posts are inventoried for future review; this release does not restore them or redirect unrelated articles to the skill catalogue.

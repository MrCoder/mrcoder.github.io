# MrCoder Skills redesign: research and approved plan

Date: 5 October 2026, Australia/Melbourne. Status: source adaptation approved on 5 October 2026; faithful AI Hero revision in progress. Release is pending preview acceptance.

## Approved implementation direction

The user's later direction, “所有东西模仿 aihero，直接 fork 没问题”, supersedes the earlier independent-design choice. A real GitHub fork has been created at [MrCoder/ai-hero](https://github.com/MrCoder/ai-hero), with [badass-courses/ai-hero](https://github.com/badass-courses/ai-hero) as its parent. Source adaptation is pinned to [`7a9fca045999c74112a13f26581872dda04e1b9e`](https://github.com/MrCoder/ai-hero/tree/7a9fca045999c74112a13f26581872dda04e1b9e).

Adapt the fork's page structure, header/sidebar, typography, theme tokens, catalogue rungs, command rows and detail-page primitives faithfully into the existing Astro site. Keep our owned skills and documentation, accurate installation controls, stable raw/download URLs, eleven-skill count and renhua alias. The catalogue has three primary groups: Understand and recover (no-bs, search-conversation-history, diagram), Work through the task (1by1, unattended, overnight), and Verify and present (spot-check, merge-quiz, pitch-me). Blind-spot-pass and deviation-log remain in quieter Optional practices.

The revised site is presented as MrCoder Skills; Field Guide remains the historical name in the initial audit. The fork supplies the source for the UI adaptation. The Next.js monorepo's CMS, authentication, commerce and workflow services are not needed to render our static eleven-skill catalogue. Do not copy AI Hero's email-course offer, company proof, remote popularity numbers or other claims as MrCoder Skills claims. Preserve upstream notices and record source attribution and component mappings; do not describe the adapted UI as independently implemented or infer a blanket MIT license from the upstream skills label.

The original research, independent-design recommendation, audit and five-group wireframes below are historical material. The later user choice and three primary groups above are authoritative. Browser evidence recorded before this change describes the earlier preview; new comparison evidence must be recorded for this revision. No remote Pages setting, publishing branch default, deployment, DNS, or domain is changed by this choice. The only remote repository action reported here is the authorized fork.

## Current implementation approach

Keep Astro static output, TypeScript content, local browser enhancements, GitHub Pages and `mrcoder.github.io`. Use the fork's design/source as the implementation reference rather than reinterpret it with an unrelated palette or layout. Headings and UI use DM Sans; command/category text uses JetBrains Mono. Follow the neutral light/dark tokens, limited gold accent, 1440px bordered shell, 264px rail and responsive catalogue structure where they fit our content. See [upstream source mapping](upstream-source-map.md).

The latest approved installation direction follows AI Hero: official `npx skills@latest` commands first, with Claude Code plugin installation as the second channel. Commands explicitly target `master`, preserve no-bs/renhua and overnight/unattended companions, and retain manual/raw downloads. The local plugin exposes the canonical root skills without duplicating them. Plugin manifests are validated locally and remain unpublished; the release evidence states that remote plugin registration requires publication. Installation and history indexing remain separate actions. The earlier bespoke curl/Bash endpoint and its evidence are historical and superseded; `/install.sh` is removed. See [installation evidence](installation-channels.md).

## Historical research findings (before the later fork decision)

### The actual website source is public

The website is in [badass-courses/ai-hero](https://github.com/badass-courses/ai-hero), whose README identifies it as the monorepo for aihero.dev. I inspected source at commit [`7a9fca045999c74112a13f26581872dda04e1b9e`](https://github.com/badass-courses/ai-hero/tree/7a9fca045999c74112a13f26581872dda04e1b9e). This is distinct from both the skills repository and the course exercise repository.

| Repository | What it contains | Reuse finding |
| --- | --- | --- |
| [badass-courses/ai-hero](https://github.com/badass-courses/ai-hero) | Actual website application | Public source; no repository-wide reuse license found in the inspected tree, manifests, or READMEs |
| [mattpocock/skills](https://github.com/mattpocock/skills) | Agent skills and human documentation | [MIT license](https://github.com/mattpocock/skills/blob/main/LICENSE), with notice requirements |
| [ai-hero-dev/ai-hero](https://github.com/ai-hero-dev/ai-hero) | Course examples and exercises | Its README describes teaching material; this is not the website application |

The website's MIT label describes the skills. It does not establish a license for the website code or artwork. [GitHub's licensing documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository) distinguishes public source from licensed reuse. The initial recommendation was independent implementation. That recommendation is superseded by the user’s later choice to fork and adapt the source faithfully. The license finding remains a provenance fact: the fork is not presented as proof of a blanket MIT license. Upstream notices and attribution remain with adapted source.

### The reference has an explicit design system

The findings below come from components and styles, supplemented by browser DOM inspection of `/skills` and `/skills-prototype`. Screenshot capture timed out, so pixel-level visual comparison has not been completed. Source revision is pinned; production deployment revision was not established.

| Aspect | Verified reference | Initial proposed adaptation (superseded where noted above) |
| --- | --- | --- |
| Navigation | Sticky 264px sidebar at widths of 1024px and above; collapsed sections | A narrower catalogue-focused rail, mobile menu, active skill and group |
| Page structure | Bordered 1440px shell; 18px mobile and 44px desktop inner gutters | Restrained shell around a readable catalogue and documentation area |
| Catalogue | Full-width numbered groups; explanation beside related skill rows; stacked on mobile | Three primary groups and quieter optional practices reflecting our eleven skills; one sentence per skill on overview |
| Typography | DM Sans for UI/headings, JetBrains Mono for commands/metadata; Source Serif 4 for testimonial voice | DM Sans and JetBrains Mono; preserve Fraunces only for a small brand moment if it helps |
| Colour | Neutral light/dark surfaces, subtle line hierarchy, limited gold accent | Neutral light/dark surfaces with Field Guide's muted rust accent; contrast-tested text |
| Hierarchy | Skills h1 scales 34/44/52px; catalogue headings and commands are quieter | Comparable responsive hierarchy without tiny essential labels |
| Install | Separate installation options with copy controls, update guidance, and compatibility | Claude Code/Codex target selection with correct package-specific instructions |
| Detail page | Skill documentation, source link, page contents, related skills, installation | Human documentation plus canonical raw source, complete package guidance, examples and requirements |

Evidence: [design language](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/DESIGN.md), [theme tokens](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/src/styles/globals.css), [font definitions](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/src/app/layout.tsx), [type scale](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/src/components/landing/type.ts), [sidebar](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/src/components/navigation/sidebar/sidebar-shell.tsx), and [catalogue rows](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/src/app/%28content%29/skills/_components/skill-set.tsx).

The [skills route](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/src/app/%28content%29/skills/page.tsx) orders the catalogue before the explanatory material. This is useful here because our current overview spends much of its opening screen on setup.

### The full application contains infrastructure beyond the static catalogue

The inspected [manifest](https://github.com/badass-courses/ai-hero/blob/7a9fca045999c74112a13f26581872dda04e1b9e/apps/ai-hero/package.json) declares Next 16.2.7, React 19.2.3 and Tailwind 4.2.1, in a pnpm/Turborepo project with Course Builder packages. Although `/skills` is cached and marked `force-static`, it loads ordered CMS lists and changelog entries from a MySQL/Drizzle database. Stars and installation badges use GitHub and Skills.sh. The email course uses subscription and workflow services.

Our catalogue can live entirely in the repository. A CMS, accounts, payments, email course, remote popularity badges, and that server stack do not contribute to the requested first release. The plan uses verified local counts and dates; it does not add invented social proof.

## Historical site audit (before implementation)

The published homepage and local `index.html` have the same SHA-256 hash, verified during this research. The repository was clean when research began; current HEAD was `218a367`.

- Static HTML, CSS, and JavaScript; no root package manifest or framework build configuration.
- Eleven advertised skills, plus the `renhua` alias: twelve top-level `SKILL.md` files.
- All documentation and install instructions appear on one homepage. At a 1440px-wide browser viewport, the page was 7522px tall.
- The opening includes a long install-all script. Skill selection requires substantial scrolling; there is no persistent catalogue navigation or search.
- Existing strengths to retain: concrete descriptions, visible requirements, complete multi-file installation instructions, ZIP checksums, semantic articles, focus styles, a compass identity, and reduced-motion handling.
- Clipboard copying has success feedback but no rejection/fallback handling.
- Mixpanel currently records `mrcoder_site_page_viewed` with `site` and `page_path`. Preserve those fields and the existing privacy-related property exclusions when integrating the new pages.
- Both current ZIP checksums passed locally. History ZIP is approximately 1.8MB; no-bs ZIP is approximately 4KB.
- Remote Pages state: the site publishes `master` at `218a367` through the legacy Pages build; the repository default is the older `gh-pages` branch. No custom domain is configured. A new workflow must target the actual publishing branch, and branch/default-source settings must be coordinated explicitly. The Pages API reported `https_enforced: false`, although the HTTPS homepage was accessible; this is a configuration finding, not evidence that HTTPS failed.
- Older Git history contains a Jekyll blog with twenty posts from 2013–2017, plus about/posts/tags/feed routes. The current site has already replaced that content. Inventory the former URLs and decide which deserve restored content or useful destinations; avoid redirecting unrelated articles indiscriminately to the skills homepage. Restoring the whole blog is outside the default first-release scope.
- The history package has its own MIT license; the site repository has no root license. Do not label the entire catalogue MIT without establishing the applicable license for each package.

Local evidence: `index.html`, `assets/style.css`, `assets/script.js`, `assets/analytics.js`, `skills/`, `downloads/`, and `tools/package-*.py`.

## Approved content and page structure

### Overview at `/`

Header and sidebar follow the fork’s primitives, adapted to MrCoder Skills, Install, About, GitHub, search and theme preference. The original compass/brand treatment is secondary to the later request for close source fidelity.

Desktop: group navigation on the left, content on the right. The content starts with a short promise: “Practical skills for better work with coding agents.” Supporting copy names the actual purposes: recover context, expose gaps, guide work, and verify results. Show the eleven-skill count derived from catalogue data, a compact install entry point, and the grouped catalogue. Retain a simplified workflow map as supporting navigation.

Each group has a number, a plain-language purpose, and short linked rows. Each row has a slash-command name and one sentence describing the outcome. A visitor can follow a group recommendation or search by name, alias, or task. Keep the whole catalogue usable without JavaScript.

| Group | Skills | Suggested entry |
| --- | --- | --- |
| 01 Understand and recover | no-bs, search-conversation-history, diagram | Select by task; these are different tools, not a required sequence |
| 02 Work through the task | 1by1, unattended, overnight | 1by1 for a queue; unattended/overnight for their documented run conditions |
| 03 Verify and present | spot-check, merge-quiz, pitch-me | spot-check, then use the other skills when their conditions apply |
| Optional practices | blind-spot-pass, deviation-log | Choose these when unfamiliar context or changes to a plan warrant them |

### Skill pages at `/skills/<slug>/`

Every skill page includes: title and outcome; when to use it; invocation/example; what it produces; requirements and limitations; installation; raw source link; related skills. A contents rail appears only when it fits. On narrow screens, content stacks and the navigation becomes a menu.

Human documentation is separate from agent instructions. The published `SKILL.md` stays canonical. Human-facing copy will be checked against it, particularly for autonomous-run boundaries, Chinese output in no-bs, the history CLI's platform requirements, and multi-file packages.

`/skills/renhua/` explains the alias and links to no-bs. It does not count as a twelfth distinct skill.

### Installation at `/install/`

Use the official skills CLI as the primary command row, with generic interactive selection and explicit Claude Code/Codex options. Use `https://github.com/MrCoder/mrcoder.github.io/tree/master` as the source. Per-skill commands select no-bs plus renhua together, and overnight plus unattended together; all other selections name the canonical skill. Complete package files and executable bits have been tested in temporary destinations.

Expose our Claude plugin as the secondary channel: register `https://github.com/MrCoder/mrcoder.github.io.git#master`, then install `mrcoder-skills@mrcoder`. Local strict validation and installation pass; remote commands require publication of the local `.claude-plugin` manifests. Namespaced skills use `/mrcoder-skills:<name>`. Do not claim official marketplace listing or automatic updates. Keep raw/ZIP installation paths and explain history indexing separately.

## Technical approach and hosting decision

| Option | Benefit | Tradeoff | Decision |
| --- | --- | --- | --- |
| Expand vanilla HTML | Smallest immediate change | Repeated navigation, content and install markup across twelve pages | Suitable fallback; weaker content maintenance |
| Astro static output | Reusable layouts, typed catalogue, Markdown docs, HTML-first output | Adds a build and deployment workflow | Recommended |
| Full Next.js/Course Builder runtime | Matches the reference’s platform | CMS/auth/commerce/workflow services exceed the static catalogue’s needs | Keep its fork/source for faithful UI adaptation; render our owned catalogue with Astro |

Astro supports [typed content collections](https://docs.astro.build/en/guides/content-collections/) and [static deployment to GitHub Pages](https://docs.astro.build/en/guides/deploy/github/). Use ordinary CSS tokens and semantic components; browser JavaScript is limited to menu, local search, theme and copy controls. A React runtime is unnecessary for this first release.

Keep skill packages in their existing source directories. Add human docs and a validated catalogue model for ordering, aliases, summaries, requirements and installation variants. The build copies published raw package files and downloads into the same output URLs. It must not add a conflicting `public/skills` source of truth or publish intermediate build files.

GitHub Pages is sufficient for the proposed pages. No domain purchase or hosting migration is needed. A custom domain can also be added on [GitHub Pages](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages) later.

Cloudflare remains an option if preview deployments or future server features justify it. Astro also has an [official Cloudflare deployment guide](https://developers.cloudflare.com/pages/framework-guides/deploy-an-astro-site/). Current packages fit Pages' [25MiB per-asset limit](https://developers.cloudflare.com/pages/platform/limits/). An owned custom domain or a `pages.dev` address would be used for a Cloudflare-hosted site; retaining the GitHub-owned hostname as the Cloudflare origin is not a DNS option under our control. Keep the original raw-file/download service working if migrating, because browsers and shell installers need different compatibility treatment.

No new model provider, API billing channel, or authentication method is needed for the redesign. Any later service choice will be stated before configuration, including costs and access requirements.

## Implementation sequence after review

1. **Freeze the publishing contract.** Inventory raw URLs, download/checksum URLs, eleven anchor IDs, alias behaviour, package file trees, installer variants, and analytics fields. Establish a baseline so migration failures are visible. Read the actual remote Pages settings before changing deployment.
2. **Build one complete slice for review.** Astro layout, theme, desktop/mobile navigation, the overview scaffold, and one representative skill page. Use unattended to exercise multi-file installation requirements. Supply desktop and mobile previews before scaling the design.
3. **Migrate the catalogue and documentation.** Complete all eleven skill pages, alias page, three primary groups plus optional practices, install page, about page, and local search. Generate counts from data. Add meaningful examples derived from actual skill instructions.
4. **Integrate publishing and compatibility.** Preserve raw sources and downloads byte-for-byte. Preserve `/#<skill>` targets with short overview entries linking to detail pages. Add metadata, social previews, sitemap, robots rules and a useful 404. Keep the existing analytics event/fields; add other events only if there is a concrete measurement purpose.
5. **Validate and prepare release.** Build/type checks; internal links and old anchors; raw-file byte comparisons; ZIP checksum verification; temporary-directory installer checks for the advertised agents/platforms; keyboard and screen-reader semantics; clipboard rejection; search/no-results; both themes; reduced motion; mobile widths from 320px upward; console/network errors. Target WCAG AA text contrast, LCP under 2.5s, CLS under 0.1, and Lighthouse accessibility/performance of at least 95 under recorded test conditions. These are targets, not claimed results.
6. **Deploy after the preview is accepted.** Configure the Pages build workflow, deploy, and smoke-check the public pages and installer URLs. Retain the old commit and publishing settings for rollback. Use a Cloudflare/domain change only if a concrete requirement emerges and the destination is chosen.

## Acceptance criteria

- A visitor can identify a relevant skill from the opening catalogue and reach useful documentation in one click.
- All eleven skills and the renhua alias remain available, with accurate requirements and install instructions.
- Every currently published raw file and download remains accessible at the same URL with the same content unless an explicitly reviewed package update changes it.
- Existing homepage anchors reach their corresponding skill summaries and detail links.
- Navigation and documentation work without JavaScript; enhanced controls work with keyboard input and touch.
- The site has a coherent light/dark theme, readable code blocks, visible focus, usable narrow layouts and no invented credibility claims.
- The release is backed by recorded build, browser, compatibility and package-install evidence.

## Review boundary

The direction is approved and the local implementation is complete. The review boundary is now the built preview: no deployment, remote Pages source change, DNS change, or domain change before acceptance. The approved scope follows the fork’s UI primitives while retaining our owned content, English human-facing documentation, Astro static output, GitHub Pages, the existing hostname, and all eleven skills plus the alias.

## Historical review wireframes and current URL contract

These are the original low-fidelity illustrations reviewed before approval. They record the historical five-group proposal. The three primary groups and quieter optional practices above supersede their catalogue grouping. They are not screenshots of the implementation and their independent visual treatment is superseded by the fork adaptation.

The historical illustrations used the Field Guide identity: DM Sans headings, mono commands/metadata, a compass mark, thin dividers, and one muted rust accent. The original illustrations used five groups; implementation uses the approved three primary groups and optional practices.

Desktop overview (wide viewport):

```text
+----------------------------------------------------------------------------------+
|  [compass] FIELD GUIDE        SKILLS   INSTALL   ABOUT   GITHUB    [search] [◐]  |
+----------------------+-----------------------------------------------------------+
|                      |  Practical skills for better work with coding agents.      |
| 01  UNDERSTAND       |  11 skills   [Browse the install guide]                    |
|     AND RECOVER      |  PRE -------- DURING ---------------- POST                  |
|   no-bs              |                                                           |
|   search-history     |  01  UNDERSTAND AND RECOVER                            >  |
|   diagram            |      Choose the tool for the problem in front of you.      |
|                      |      no-bs                         Plain-language answers  |
| 02  BEFORE YOU BUILD |      search-conversation-history  Recover prior decisions  |
|   blind-spot-pass    |      diagram                       Make the flow visible  |
|                      |                                                           |
| 03  WORK THROUGH     |  02  BEFORE YOU BUILD                                   >  |
|     THE TASK         |      blind-spot-pass              Surface unknowns first  |
|   1by1               |                                                           |
|   unattended         |  03  WORK THROUGH THE TASK                             >  |
|   overnight          |      1by1   unattended   overnight                      |
|                      |                                                           |
| 04  KEEP THE BUILD   |  04  KEEP THE BUILD HONEST                             >  |
|     HONEST           |      deviation-log                 Record plan changes     |
|   deviation-log      |                                                           |
|                      |  05  VERIFY AND PRESENT                                >  |
| 05  VERIFY AND       |      spot-check   merge-quiz   pitch-me                   |
|     PRESENT          |                                                           |
|   spot-check         |  [Install all]  [Read about the Field Guide]             |
|   merge-quiz         |                                                           |
|   pitch-me           |  small note: original raw files and downloads remain      |
|                      |  available from their stable URLs                         |
+----------------------+-----------------------------------------------------------+
```

Mobile overview (stacked viewport):

```text
+--------------------------------+
| [☰] [compass] FIELD GUIDE [◐]  |
+--------------------------------+
| Practical skills for better    |
| work with coding agents.       |
| [Search skills] [Install]      |
| PRE ----- DURING ----- POST    |
|                                |
| 01  UNDERSTAND AND RECOVER  v  |
|     no-bs       Plain answers  |
|     search-history  Recall     |
|     diagram     Make visible   |
|                                |
| 02  BEFORE YOU BUILD        v  |
|     blind-spot-pass           |
|                                |
| 03  WORK THROUGH THE TASK   v  |
|     1by1  unattended  overnight|
|                                |
| 04  KEEP THE BUILD HONEST   v  |
|     deviation-log             |
|                                |
| 05  VERIFY AND PRESENT      v  |
|     spot-check  merge-quiz     |
|     pitch-me                   |
|                                |
| [Install all skills]           |
+--------------------------------+
```

Skill-detail page (desktop; the same regions stack on mobile):

```text
+----------------------------------------------------------------------------------+
| [compass] FIELD GUIDE        SKILLS   INSTALL   ABOUT   GITHUB    [search] [◐]  |
+----------------------+-----------------------------------------------------------+
| 01 UNDERSTAND...     | 01 / UNDERSTAND AND RECOVER                             |
|   no-bs              |                                                           |
|   search-history     | no-bs                                                     |
|   diagram            | Plain-language answers when the first explanation is too   |
|                      | long or technical.                         [Copy command]  |
| 02 BEFORE...         |                                                           |
|   blind-spot-pass    | USE IT WHEN                                                |
| ...                  | “说人话”, /no-bs, or $no-bs                                |
|                      |                                                           |
|                      | WHAT IT DOES                                               |
|                      | Short outcome, boundaries, and one realistic example.      |
|                      |                                                           |
|                      | INSTALL                                                     |
|                      | [Claude Code]  [Codex]  [Copy]                              |
|                      | requirements and platform notes                            |
|                      |                                                           |
|                      | RELATED                                                     |
|                      | search-conversation-history · diagram                       |
|                      |                                                           |
|                      | [View raw SKILL.md]  [Open package/downloads]               |
+----------------------+-----------------------------------------------------------+
```

The route contract keeps the existing shell-facing URLs stable while adding human documentation:

| Route | Contract in the redesign |
| --- | --- |
| `/` | New overview and catalogue. It contains three numbered primary groups, quieter optional practices, and links to each human skill page. |
| `/#<slug>` | Preserve the eleven existing homepage anchors (`no-bs`, `search-conversation-history`, `diagram`, `1by1`, `unattended`, `overnight`, `blind-spot-pass`, `deviation-log`, `spot-check`, `merge-quiz`, `pitch-me`). Each target lands on its overview row and exposes the detail link. |
| `/skills/<slug>/` | New human-facing documentation page for each skill. Include `/skills/renhua/` as the alias explanation page linking to `no-bs`. |
| `/skills/<slug>/SKILL.md` | Preserve each published raw skill file at its current URL and content. Nested reference, script, template, test, and package files remain available at their existing paths. |
| `/skills/renhua/SKILL.md` | Preserve the backwards-compatible raw alias as a separate stable file. |
| `/downloads/*` | Preserve ZIP archives and SHA-256 files byte-for-byte unless a package release is explicitly reviewed. |
| `/install/` | Installation hub with official skills CLI commands, a Claude plugin channel with marketplace registration and install commands, manual files, checksums, platform notes, and selectable fallback text. |
| `/about/` | Provenance page for Peng Xiao, MrCoder Skills, the source fork, licensing boundaries, and the relationship to the forked source. |
| `/404.html` | Add a useful static fallback with links back to the catalogue, install guide, and GitHub source. |

Former Jekyll blog routes remain a separate redirect/content inventory. They must be reviewed against the old permalink scheme before any redirect is added.

# MrCoder Skills: AI Hero source revision

The visual review below predates the later switch to AI Hero's installation methods. Its custom Bash installer results are historical. See `2026-10-05-skills-cli-validation.md` for the current CLI/plugin implementation and verification.

Date: 5 October 2026, Australia/Melbourne. Branch: `codex/field-guide-redesign`, based on `218a367`. At the time of this review, implementation was ready for preview review and release was pending acceptance. The user subsequently accepted the local preview and explicitly authorized release on 5 October 2026; deployment still requires verification.

## Source and implementation

The user explicitly requested imitation of AI Hero and permitted a direct fork. The real fork is [MrCoder/ai-hero](https://github.com/MrCoder/ai-hero), parent `badass-courses/ai-hero`, source revision `7a9fca045999c74112a13f26581872dda04e1b9e`. The current Astro site ports its front-end structure and styles; it does not run the upstream Next.js CMS, authentication, commerce or email services. See `../plans/upstream-source-map.md` for component and illustration provenance.

Live reference screenshots were inspected at 1440×1000 and 390×844. The revised UI follows the bordered shell with 8px outside gutters, 33px announcement, 62px header, 264px sidebar, 18/44px inner gutters, DM Sans and JetBrains Mono, neutral light/dark colours, gold buttons, hero installation rail, numbered catalogue groups, command rows, detail contents rail and footer. Our eleven skills, accurate package instructions and own navigation replace upstream course offerings and popularity claims. Blind-spot-pass and deviation-log remain optional practices. Source adaptations are recorded rather than described as independent design.

## Verification

- `npm run check`: nineteen checked files, zero errors, warnings or hints.
- `npm test`: fourteen tests pass. Seven cover installation behavior; seven cover publishing boundaries, immutable preservation, routes, archives and the installer endpoint.
- `npm run build`: sixteen HTML pages plus `/install.sh`; all sixty-six original file hashes, both ZIP checksums, internal links/fragments, original homepage anchors, alias, canonicals and sitemap pass.
- Published `/install.sh` matches its shared generator byte-for-byte and passes Bash syntax checks. Fifty emitted compact commands target the correct endpoint.
- Real curl-to-Bash downloads installed the complete set separately for Claude and Codex into explicit temporary destinations. Each contained exactly fifty-nine files with matching bytes and executable permissions. A real individual overnight installation also included unattended. Personal skill directories and `HOME` were not changed; temporary installation roots were removed.

## Browser review

Reviewed the production output at `http://127.0.0.1:4322/` through the browser.

- Desktop 1440×1000 and mobile 390×844 screenshots show the revised source-based UI. Every one of the sixteen HTML routes was checked at 320×740: document width equals viewport width. Commands scroll inside their own containers.
- Alias search for renhua returns one no-bs result; an unmatched query shows the empty state; Clear restores eleven results.
- Copying an installation command succeeds and matches the clipboard. An isolated copy of the production install HTML with a rejecting clipboard boundary shows the manual-copy message and visibly selects the command.
- Mobile navigation opens and reaches the install page. Selecting overnight and Codex produces the correct individual command. Native mobile contents expands and its requirements link reaches the correct fragment. The desktop detail page shows a separate 232px contents rail.
- Dark theme selection persists after reload; both themes were visually reviewed. Light theme was restored for preview.
- A scripts-disabled sandbox frame renders the production install page with ordinary links and Codex fallback markup, while enhanced copy controls are hidden. Activation inside scripts-disabled frames is unavailable in the automation backend and remains a manual check; no claim of that activation is made.
- The final page reload records no browser console warnings or errors.

Review found and fixed two issues introduced during adaptation: faint group numbers failed contrast, and intrinsic grid-child sizing let long detail pages overflow on mobile. Group numbers now use the readable muted colour; document grid children explicitly allow shrinking. The latter fix was checked on all sixteen routes.

## Local audit and preview

The final Lighthouse mobile navigation audit records accessibility 100, best practices 100, SEO 100 and agentic browsing 100, with fifty-four passed checks and zero failed checks. See `aihero-lighthouse-summary.json`. This tool excludes performance, and this homepage-only automated audit does not establish whole-site WCAG conformance or live production performance. Earlier independent-design performance traces are historical and do not describe this revision.

Current screenshots are `../previews/aihero-desktop.png`, `../previews/aihero-mobile.png` and `../previews/aihero-detail.png`. Temporary browser QA tabs and files were removed. The review preview remains open.

Historical state at the time of this review: no release had been pushed, and GitHub Pages configuration, repository default branch, DNS, hostname and the public deployment were unchanged. The prepared release workflow targets the publishing branch, `master`. The former `/install.sh` endpoint was subsequently superseded by the official skills CLI and removed; it will not become available after deployment. Consult the current CLI/plugin evidence for installation behavior and verified release evidence for deployment status.

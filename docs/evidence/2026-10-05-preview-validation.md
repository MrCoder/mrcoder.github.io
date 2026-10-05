# Field Guide preview validation

Historical evidence for the first independent design. The user later chose a real AI Hero fork and faithful source adaptation. See `2026-10-05-aihero-source-validation.md` for the current preview; scores and screenshots below must not be attributed to the revised UI.

Date: 5 October 2026, Australia/Melbourne. Branch: `codex/field-guide-redesign`, based on `218a367`. Status: implementation ready for preview review; production release pending acceptance.

## Implementation

Astro 7.3.5 generates sixteen HTML pages: the catalogue, install guide, about page, eleven skills, renhua alias, and 404. Human documentation lives in typed catalogue data rather than separate Markdown files. At this catalogue size, one structured record keeps overview copy, detail documentation, groups and related links together. Canonical agent Markdown remains separate and unchanged.

The three primary workflow groups are followed by Optional practices. All eleven skills remain available. The site uses local DM Sans and a system monospace font, an original compass, light/dark themes, search, agent-specific installation controls and original social artwork. No reference-site code or artwork was copied. Preview traffic does not emit production analytics; the existing analytics script and production event fields remain unchanged.

## Automated checks

| Check | Result |
| --- | --- |
| `npm run check` | No errors, warnings or hints |
| `npm test` | All twelve tests pass |
| `npm run build` | Sixteen HTML pages; compatibility verification passes |
| Immutable publishing contract | Sixty-six existing files match source and output SHA-256 baselines |
| ZIP verification | Both archives match their published checksums |
| Links and routes | Internal links, fragments, eleven old homepage anchors, alias, canonicals and sitemap pass |
| Installer regression checks | Complete package trees, permissions, aliases, dependencies, backups, obsolete-file removal, unrelated-package preservation and missing-package abort pass |

Installer regression tests replace only Git's network boundary and redirect the destination to temporary directories. A separate real-network check ran both generated full-set installers against `master` on GitHub. For each agent, all fifty-nine installed skill files matched baseline bytes and executable permissions. User skill directories were untouched. Actual execution was on this macOS machine; Linux and Windows execution were not verified.

## Browser checks

Reviewed the production build at `http://127.0.0.1:4322/`, and the development build during construction.

- Desktop at 1440×1000, mobile at 390×844, and narrow layouts at 320×740.
- Catalogue, installation page and the long history documentation page have no horizontal document overflow in the checked narrow views. Long commands retain their own horizontal scroll area.
- Search matches the renhua alias, shows an empty state, and clears back to all eleven skills. Singular result feedback uses “1 skill found”.
- Theme changes persist after reload. Both light and dark layouts were reviewed.
- Mobile catalogue disclosure opens; navigation reaches installation. Agent and skill selection generate the correct destination and unattended dependency for overnight.
- Keyboard traversal reaches the navigation controls; visible focus styles are present.
- Successful clipboard copying reports “Command copied.” An isolated copy of the production HTML with a rejecting clipboard boundary reports the manual-copy fallback; its command becomes visibly selected.
- A sandboxed frame with scripts disabled renders the real production install page, hides enhanced controls, and exposes the native Codex command disclosure and ordinary documentation links. The automation backend could not activate controls within that script-disabled frame, so disclosure activation and link navigation in this condition remain a manual preview check.
- No console warnings or errors were observed after the final production page reload.

Two visual issues found during review were fixed: long backup-path prose overflow at 320px, and a redundant rail note overlapping sticky navigation during scrolling. The latter note was removed.

## Lab audit

Lighthouse 13.4.1, mobile navigation audit of the local production homepage, recorded at `2026-10-04T23:51:33.727Z`: accessibility 100, best practices 100, SEO 100, agentic browsing 100. See `lighthouse-summary.json`. This tool excludes the performance category; no performance score is claimed. Automated accessibility scoring does not establish complete WCAG conformance.

A separate Chrome performance trace used a 390×844 touch viewport, Slow 4G network emulation and 4× CPU slowdown. It recorded LCP 715ms and CLS 0.00. This was a reload after the local audit, with prior browsing/cache state, against a localhost server. These are lab observations; live GitHub Pages latency and field metrics remain unmeasured. The final analytics hostname guard subsequently removed analytics loading from local previews.

## Release boundary

The local production preview remains available for review. Remote Pages configuration, the default branch, DNS and the public deployment are unchanged. The prepared workflow targets `master`; release requires coordinating the switch from legacy Pages publishing to GitHub Actions after preview acceptance. Review the design and the remaining no-script activation check, then deploy and smoke-check the public routes and download URLs.

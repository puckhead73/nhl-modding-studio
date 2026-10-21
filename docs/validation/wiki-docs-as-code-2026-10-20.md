# Public wiki as docs-as-code — implementation receipt (20 October 2026)

**Plain-language summary.** The repository now carries a complete public wiki under `wiki/`: a user guide for every Studio screen and tool, and a modding knowledge base distilled from the format specs, audits and research handoffs, with every claim labelled measured, inferred or open. A lint refuses broken links, unreachable pages and any content that must not be public (machine paths, file hashes, other tool repositories, executable internals, unreleased titles). Thirty pages are generated from the audit data and drift-checked. A GitHub Action mirrors the folder to the repository's GitHub Wiki; the first publish is a manual step for the user. Nothing is committed and nothing has been published yet. Screenshots were not captured.

Plan approved by the user on 19 October 2026 (hosting in-repo with a mirror; verified plus labelled inferred findings; executable, NHLViewNG-disassembly and NHL 27 / PS5 material excluded entirely; screenshots allowed).

## 1. Scope delivered

| Phase | Content | Pages |
|---|---|---|
| 0 | `wiki/Home.md`, `_Sidebar.md`, `_Footer.md`, `Contributing-to-the-Wiki.md`; `scripts/wiki_check.py` + tests; `.github/workflows/wiki.yml`; CI job; README link | 4 |
| 1 | Getting started (6), Mod Manager, NHLMOD Packages, Save Editor, Troubleshooting, Known Limitations, Reporting Bugs | 12 |
| 2 | Modding Studio reference (26 screen pages), Portable Projects, Save Editor per-platform notes, FAQ, Glossary | 30 |
| 3 | Knowledge base verified core: conventions, series overview, TDB (4 pages), schema drift, field encodings, roster model, save containers (2), archives, historical corpus, NHLViewNG comparison | 15 |
| 4 | Knowledge base research digests: textures, logos, meshes, head UV transfer, heads and portraits, PS4 vault, leagues and schedules, teams and kits, arenas, equipment, audio (5), Recompiled, Frostbite | 17 |
| 5 | `scripts/wiki_generate.py` + tests: 27 per-title schema pages, schema index, save formats, surface capabilities | 30 |
| 6 | Index and cross-link sweep; this receipt | — |

Total: 108 pages (78 hand-written, 30 generated), 4.4 MB, all under `wiki/`.

## 2. Changed paths

New: `wiki/` (108 `.md`), `scripts/wiki_check.py`, `scripts/wiki_generate.py`, `scripts/tests/test_wiki_check.py`, `scripts/tests/test_wiki_generate.py`, `.github/workflows/wiki.yml`, this receipt.

Modified: `.github/workflows/ci.yml` (new `wiki` job: generator self-test, generator drift check, lint), `README.md` (wiki link in the Documentation section).

Not part of this package and untouched: the pre-existing modified and untracked files in the working tree (`Cargo.lock`, `app/src/pages/AssetExport*.tsx`, `app/src/pages/PlayerAssets.tsx`, `crates/studio-domain/src/{assets,head_catalog,head_census,appearance_texture}.rs`, `crates/studio-domain/examples/appearance_audit.rs`, `docs/audits/head-extraction-20261019/`, `docs/validation/head-extraction-20261019.md`, `scripts/{appearance_inventory,check_appearance_exports,frostbite_head_probe,frostbite_mesh_measure,stage_appearance_fixtures}.py`).

## 3. Key decisions

- **Page names are flat** (`Page-Name.md`) because the GitHub Wiki flattens folders; `images/` is the only allowed folder.
- **Reachability** counts pages linked from the sidebar directly or through an index page the sidebar links, computed per sidebar section so a knowledge page linking into the user guide does not reclassify it. Every page under the knowledge-base section must carry an `**Evidence:**` line.
- **Banned patterns** (lint): drive paths, the private fixture root, 64-hex hashes, `local/runs` and `local/coordination`, the prior tool repository, the outside SDK, the NHLViewNG author's site, the user's home folder, NHL 25 to 29, PPSA ids, PS5, "eboot", XEX tooling, and "decrypt… executable". The decryption ban was narrowed from a bare `decrypt` after it refused the app's own "Use a decrypted PS3 ISO" wording.
- **Generated pages** carry a banner and a fixed `REVIEW_DATE` so `--check` is deterministic; a hand-written page occupying a generated name makes `--write` refuse before touching anything.
- **Research digests** were produced by four read-only exploration agents briefed to apply every correction checkpoint in the handoff folders and to strip hashes, paths, executable material, unreleased titles, third-party tool names and people's names; the wiki pages were then written from those digests. Withdrawn claims (for example the "slot = index value" schedule reading, the "rink logo" label, the fixed 40-slot roster block, the 60 000 pool floor before NHL 15) are stated as corrected where they aid understanding.
- **Items the plan left as user questions** and the pages' handling until answered: the EA-data statement (pages avoid it); wiki licence (footer carries the non-affiliation line only); evidence label wording (measured / inferred / open used); the private portrait library (described generically as an optional library folder).
- The in-app PS5 and NHL 25+ tiles are mentioned only as "later titles marked In development"; the Settings page describes console tabs without naming them.

## 4. Checks executed

| Check | Result |
|---|---|
| `python -m unittest discover -s scripts/tests -p "test_wiki_*.py"` | 15 tests, OK (10 lint, 5 generator) |
| `python scripts/wiki_generate.py --write` then `--check` | 30 pages rendered, no drift |
| `python scripts/wiki_check.py` | 108 pages, 0 findings |
| `git diff --check` on `ci.yml` and `README.md` | clean |
| `wiki.yml` | authored; not executed (no push; the wiki repository is not initialised). Uses `rsync`, `GITHUB_TOKEN` with a `WIKI_TOKEN` fallback, pushes to the wiki's `master` branch; `workflow_dispatch` has a `dry_run` input. |
| Desktop app | not run. Page text was written from the React page sources and their components, not from screenshots. |

Not executed: the end-to-end quick-start walk-through in the dev build; screenshot capture (no `wiki/images/` content); the first publish.

## 5. Findings recorded, not fixed

- The About page (`/about`, `/app/settings/about`) is unreachable from the menus; README and the download-page draft point users at "Settings → About & Licences". The Known Limitations page states the About page is unreachable.
- `/manager/install`, `DeploymentPanel.tsx`, `BuildPackagePanel.tsx` and `DiscImagePanel.tsx` are unmounted.
- `docs/STATE-OF-THE-REPO.md` counts (115 commands, 27 pages) are stale: 270 registered commands, 37 page modules.
- `docs/release/artwork-replacement-backlog.md` describes the title tiles as box art; the user states they are not.
- `docs/03-tdb-format.md`, `docs/05-save-data.md` and `docs/studio-vs-nhlviewng-comparison.md` are stale, the last with a private home-directory path.
- The Frostbite browser's intro and one backend error name the next console generation; the wiki paraphrases them.
- Two code-comment conflicts flagged by the digest: NHL 10 skin copies (byte-identical vs per-submesh rigs) and the tongue palette slot (20 vs 21). Both are stated as open on the meshes page.
- The appearance catalogue checkbox says `.dds` while the Appearance Studio reads textures as PNG; thumbnails may show "—". Not verified in the app.

## 6. Temporary artefacts

None under the session scratchpad. Agent transcripts and oversized tool outputs live in the harness's own task folders outside the repository. No build was run; `target/` was not touched.

## 7. Next review action

1. Codex review of the wiki tree and the two scripts; the lint's banned-pattern list is the main policy surface.
2. User decisions on the four open questions (EA-data statement, licence, label wording, portrait-library description).
3. Commit when authorised (suggested paths: `wiki/`, `scripts/wiki_*.py`, `scripts/tests/test_wiki_*.py`, `.github/workflows/wiki.yml`, `.github/workflows/ci.yml`, `README.md`, this receipt).
4. User initialises the GitHub Wiki once in the web UI and runs the *Wiki* workflow manually with `dry_run` first, then for real.
5. Screenshot pass into `wiki/images/`, and a quick-start walk-through in the dev build, as a follow-up package.

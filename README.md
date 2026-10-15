# NHL Modding Studio

A comprehensive, polished toolkit for modding EA's NHL games — the spiritual successor to **NHLViewNG**.

NHL Modding Studio aims to be the single place where the community edits, imports, and manages
the data behind the EA NHL series: rosters, players, teams, save data, and the game databases
themselves. The design goal is **simple and efficient for casual users, with deep capability for
power users** who want to go further.

> **Status:** In development, not released. The engine, database/roster editing, historical import
> and much of the asset tooling are implemented; the remaining work is sequenced in the
> [entity-editing parity roadmap](docs/plans/entity-editing-parity-roadmap.md). Projects/Builds are deferred stretch work. Nothing is released until
> the whole agreed project is complete, and **no title/platform target is game-verified yet** —
> implemented and automated-tested are not the same as verified in the game.

---

## What it covers

| Dimension        | Coverage |
|------------------|----------|
| **Game titles**  | **17 catalog entries** — NHL 07 → NHL 21, NHL Legacy, NHL Legacy Recompiled |
| **Platforms**    | Xbox 360, PlayStation 3, PlayStation 4, and the **NHL Legacy Recompilation** (based on the Xbox build). There is no Xbox One target. |
| **Targets**      | **27 shipping title × platform targets** — the catalog's actual combinations, not every title on every console. NHL 07 PS3 is a research copy, not a shipping target. |
| **Data formats** | EA **TDB** databases (with their per-year nuances), console save containers, and the games' texture/mesh/audio assets |
| **Imports**      | Players, teams, and rosters sourced from a built-in historical hockey database spanning **1909 → present** (117 years, hundreds of leagues) |

See [docs/01-scope-and-compatibility.md](docs/01-scope-and-compatibility.md) for the full
title × platform matrix and the evidence level behind each cell.

## Key capabilities

Implemented today (see [docs/STATE-OF-THE-REPO.md](docs/STATE-OF-THE-REPO.md) for the detailed
inventory, and the completion roadmap for what each still needs before it is finished):

- **TDB database editing** — byte-exact read/write of EA's TDB databases across the catalog, with
  evidence-based capability probing for the format's year-to-year and platform nuances.
- **Save data handling** — open, inspect, and edit console save containers (Xbox 360, PS3, PS4,
  Legacy). Original-console packaging and the full required save-mode set are still in progress.
- **Historical source database** — a curated database of players, teams, and rosters going back
  to 1909, used as an import source to populate any supported game.
- **Roster / team / player management** — high-level editing flows on top of the raw databases,
  including Legends roster construction.
- **Production asset tools** — portraits, logos, uniforms/nameplates, hockey shop/equipment,
  appearance and headmesh work, with localization and audio still to come.

## Technology

- **Core:** Rust (via [Tauri v2](https://tauri.app/)) — fast, safe binary parsing for TDB and save formats.
- **UI:** React + TanStack Query inside a Tauri shell.
- **Historical store:** SQLite (the FHM corpus).
- **Packaging:** Small native installers per OS.

See [docs/DESIGN.md](docs/DESIGN.md) for the architecture and rationale.

## Documentation

**Canonical:**

| Doc | Purpose |
|-----|---------|
| [docs/plans/entity-editing-parity-roadmap.md](docs/plans/entity-editing-parity-roadmap.md) | **Current execution sequence:** leagues, teams, players, equipment and presentation; per-step cross-game parity |
| [docs/plans/project-completion-roadmap.md](docs/plans/project-completion-roadmap.md) | Detailed C0–C10 requirements, subject to the current roadmap's scope and sequencing overrides |
| [docs/plans/completion-design-questions.md](docs/plans/completion-design-questions.md) | Accepted scope decisions and the open-question queue |
| [docs/DESIGN.md](docs/DESIGN.md) | Architecture, crate set, service layers, feature inventory, cross-cutting systems |
| [docs/EDIT-MODEL.md](docs/EDIT-MODEL.md) | **Binding** database-editing contract — metadata-driven, entity-first guardrails |
| [docs/STATE-OF-THE-REPO.md](docs/STATE-OF-THE-REPO.md) | What is actually implemented today |
| [docs/plans/full-game-qa-editability.md](docs/plans/full-game-qa-editability.md) | Full-game QA and editability verification contract |
| [docs/ROADMAP.md](docs/ROADMAP.md) | **Historical** phase record (original NHL 10–21 framing); superseded for sequencing by the completion roadmap |

**Topic deep-dives:**

| Doc | Purpose |
|-----|---------|
| [docs/00-vision.md](docs/00-vision.md) | Why this exists, principles, success criteria |
| [docs/01-scope-and-compatibility.md](docs/01-scope-and-compatibility.md) | Titles, platforms, and the compatibility matrix with per-cell evidence levels |
| [docs/03-tdb-format.md](docs/03-tdb-format.md) | EA TDB database format notes and per-year nuances |
| [docs/04-source-database.md](docs/04-source-database.md) | The 1909→present historical hockey database (FHM corpus) |
| [docs/05-save-data.md](docs/05-save-data.md) | Platform save-data handling |
| [docs/glossary.md](docs/glossary.md) | Domain terminology |
| [docs/audits/](docs/audits/) | Verified format evidence with provenance |
| [docs/validation/](docs/validation/) | Work-package baseline and validation records |

**Reference (from the prior app):**

| Doc | Purpose |
|-----|---------|
| [docs/rebuild-design.md](docs/rebuild-design.md) | Original design+roadmap brief (source for DESIGN/ROADMAP) |
| [docs/nhlviewng-functional-report.md](docs/nhlviewng-functional-report.md) | NHLViewNG feature/edit/save map (source for EDIT-MODEL) |
| [docs/studio-vs-nhlviewng-comparison.md](docs/studio-vs-nhlviewng-comparison.md) | Gap analysis + redesign direction (source for EDIT-MODEL) |
| [docs/MESH_IMPORT_EXCLUSIVE_PATH.md](docs/MESH_IMPORT_EXCLUSIVE_PATH.md) | Head-mesh import research (deferred / out of RC scope) |

## Legal & ethics

NHL Modding Studio is a fan-made tool for editing legally owned copies of EA NHL games. It ships
**no copyrighted EA game data**. The historical source database is independently compiled.
See [docs/00-vision.md](docs/00-vision.md#legal--ethics) for details.

## License

Studio-owned source code is released under the **MIT License** (decision Q46e,
[completion design questions](docs/plans/completion-design-questions.md), 7 September 2026). The `LICENSE`
file is added once its copyright line is recorded; until then release archives withhold it and say so. Package
metadata declares `MIT` (`tdb-speex`: `MIT AND BSD-3-Clause`, for its vendored libspeex).

Third-party components keep their own licences: the vendored libspeex 1.2.1 (BSD-3-Clause), the FFmpeg-based
XMA2 helper (LGPL-2.1-or-later, shipped with its source), the Rust crates and npm packages compiled into the app,
and the WebView2 SDK loader. A release archive lists each in `THIRD-PARTY-NOTICES.txt`, shown in the app under
Settings → About & Licences. The MIT licence does not cover EA game data, trademarks or the artwork embedded in
the app (see [the download-page draft](docs/release/draft-download-page-attribution.md)).

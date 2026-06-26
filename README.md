# NHL Modding Studio

A comprehensive, polished toolkit for modding EA's NHL games — the spiritual successor to **NHLViewNG**.

NHL Modding Studio aims to be the single place where the community edits, imports, and manages
the data behind the EA NHL series: rosters, players, teams, save data, and the game databases
themselves. The design goal is **simple and efficient for casual users, with deep capability for
power users** who want to go further.

> **Status:** Pre-release scaffold. Architecture and documentation are being established now; a
> detailed feature roadmap (derived from the prior rough-draft application) is being ingested.
> No application code has been committed yet.

---

## What it covers

| Dimension        | Coverage |
|------------------|----------|
| **Game titles**  | NHL 10 → NHL 21 |
| **Platforms**    | Xbox (360/One family), PlayStation 3, PlayStation 4, and the **NHL Legacy Recompilation** (based on the Xbox build) |
| **Data formats** | EA **TDB** databases (with their per-year nuances), platform save data |
| **Imports**      | Players, teams, and rosters sourced from a built-in historical hockey database spanning **1909 → present** (117 years, hundreds of leagues) |

See [docs/01-scope-and-compatibility.md](docs/01-scope-and-compatibility.md) for the full
title × platform compatibility matrix.

## Key capabilities (planned)

- **TDB database editing** — read/write EA's TDB databases across NHL 10–21 with version-aware
  schema handling for the format's many year-to-year nuances.
- **Save data handling** — open, inspect, and edit platform save data (Xbox, PS3, PS4, Legacy).
- **Historical source database** — a curated database of players, teams, and rosters going back
  to 1909, used as an import source to populate any supported game.
- **Roster / team / player management** — high-level editing flows on top of the raw databases.

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
| [docs/DESIGN.md](docs/DESIGN.md) | Architecture, crate set, service layers, feature inventory, cross-cutting systems |
| [docs/EDIT-MODEL.md](docs/EDIT-MODEL.md) | **Binding** database-editing contract — metadata-driven, entity-first guardrails |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phased plan and verification gates |

**Topic deep-dives:**

| Doc | Purpose |
|-----|---------|
| [docs/00-vision.md](docs/00-vision.md) | Why this exists, principles, success criteria |
| [docs/01-scope-and-compatibility.md](docs/01-scope-and-compatibility.md) | Titles, platforms, and the compatibility matrix |
| [docs/03-tdb-format.md](docs/03-tdb-format.md) | EA TDB database format notes and per-year nuances |
| [docs/04-source-database.md](docs/04-source-database.md) | The 1909→present historical hockey database (FHM corpus) |
| [docs/05-save-data.md](docs/05-save-data.md) | Platform save-data handling |
| [docs/glossary.md](docs/glossary.md) | Domain terminology |

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

To be determined. See [docs/00-vision.md](docs/00-vision.md).

# 02 — Architecture

> This describes the **intended** architecture. The application has not been scaffolded yet; this
> document is the target the initial implementation should aim for. Revisit it as the roadmap
> ([06-roadmap.md](06-roadmap.md)) lands.

## Technology choice

**Tauri** — a Rust core paired with a web UI in a native shell.

Rationale:

- **Binary parsing belongs in Rust.** TDB databases and platform save data are binary formats with
  byte-exact layouts, endianness differences, and version nuances. Rust gives us safe, fast,
  zero-copy-friendly parsing with strong typing — a far better fit than parsing buffers in JS.
- **Rich UI without a heavy runtime.** A web frontend gives us the flexibility to build the
  comprehensive database/roster browser; Tauri keeps the footprint small versus Electron.
- **Cross-platform desktop** from one codebase.
- The prior app was .NET; Tauri keeps the strengths (native, performant) while modernizing the UI
  layer and giving us best-in-class binary handling in the core.

## High-level layers

```
┌─────────────────────────────────────────────────────────┐
│  UI (web frontend, src/)                                 │
│  • Database browser / editors  • Roster & team flows     │
│  • Historical-DB import wizard • Save-data inspector      │
└───────────────▲─────────────────────────────────────────┘
                │  Tauri commands / typed IPC
┌───────────────┴─────────────────────────────────────────┐
│  Application core (Rust, src-tauri/)                      │
│  ┌───────────────┐  ┌───────────────┐  ┌──────────────┐  │
│  │  tdb crate    │  │  savedata     │  │  sourcedb    │  │
│  │  (EA TDB R/W, │  │  crate        │  │  crate       │  │
│  │  versioned)   │  │  (per-platform│  │  (1909→ hist │  │
│  │               │  │   containers) │  │   import)    │  │
│  └───────────────┘  └───────────────┘  └──────────────┘  │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  domain model: players / teams / rosters / leagues   │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

## Proposed crate / module breakdown

- **`tdb`** — read/write EA TDB databases. Version- and platform-aware: a schema/codec layer that
  resolves the right field layout for a given title (10–21) and platform (endianness, packing).
  See [03-tdb-format.md](03-tdb-format.md).
- **`savedata`** — open/inspect/write platform save containers (Xbox / PS3 / PS4 / Legacy).
  See [05-save-data.md](05-save-data.md).
- **`sourcedb`** — the historical hockey database and the import pipeline that maps its
  players/teams/rosters onto a target game's schema. See [04-source-database.md](04-source-database.md).
- **`domain`** — shared, format-agnostic model (Player, Team, Roster, League, Season) that the UI
  and import pipeline speak in, decoupled from any single game's raw schema.
- **IPC layer** — Tauri commands exposing the above to the UI with typed request/response shapes.

## Key cross-cutting concerns

- **Versioning:** a central registry mapping (title, platform) → schema/codec. New titles plug in
  here rather than scattering conditionals.
- **Safety of writes:** validate before write; prefer backups / reversible edits; never partially
  write a corrupt database.
- **Endianness:** handled in the codec layer, never leaked into domain or UI code.
- **Performance:** large databases and rosters must stay responsive; stream/lazy-load where needed.

## Repository layout (target)

```
nhl-modding-studio/
├─ src/                 # web frontend (UI)
├─ src-tauri/           # Rust core + Tauri shell
│  ├─ crates/
│  │  ├─ tdb/
│  │  ├─ savedata/
│  │  ├─ sourcedb/
│  │  └─ domain/
│  └─ src/              # Tauri command surface / app wiring
├─ docs/                # this documentation
├─ sample-data/         # ONLY synthetic/redacted fixtures (no real EA data)
└─ README.md
```

> Layout is provisional and will be created when the app is scaffolded per the roadmap.

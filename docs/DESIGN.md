# DESIGN — NHL Modding Studio

> **Canonical architecture document.** Derived from the prior app's design brief
> ([rebuild-design.md](rebuild-design.md), kept as the reference source) and updated for the
> agreed release-candidate scope. For sequencing see [ROADMAP.md](ROADMAP.md).
>
> **The database-editing model is binding and lives in [EDIT-MODEL.md](EDIT-MODEL.md)** — the
> metadata-driven, entity-first, illegal-states-unrepresentable contract distilled from NHLViewNG
> ([nhlviewng-functional-report.md](nhlviewng-functional-report.md),
> [studio-vs-nhlviewng-comparison.md](studio-vs-nhlviewng-comparison.md)). The "Cross-cutting
> systems" and `studio-domain` sections below implement it.

## Context

NHL Modding Studio is a greenfield, release-quality rebuild that treats the prior
`nhl-database-studio` app as a **validated rough draft / reference implementation**. That app is a
mature Tauri + React + Rust desktop tool for editing EA NHL **TDB** binary databases and their
console roster-save containers. It works and it encodes a large amount of hard-won, byte-exact
knowledge — especially the **NHLViewNG-parity save/write path**: CRC-correct TDB serialization,
save-container pack/unpack, the staged edit model with three-tier validation, the domain-integrity
engine, and the FHM historical import pipeline.

It grew by accretion: ~140 overlapping gui-api commands, an organically grown frontend presentation
layer, and experimental research code (recomp, NHL2K10 RE, rig-audit, head-import, FMT plugins)
interleaved with shipping features. The rebuild carries forward **100% of the core studio +
production-tool functionality** on the proven engine, with clean layering.

## Scope (release candidate)

- **Titles:** NHL 10 → NHL 21.
- **Platforms:** Xbox, PS3, **PS4**, and the NHL Legacy Recompilation (Xbox-derived).
- **In:** DB editing (players/teams/arenas/jerseys), roster/lines/transactions, parity save+write,
  FHM historical import, EA rating formulas, and the *shipped* asset tools (portraits, textures,
  equipment/hockey-shop, localization, music/PA/PBP audio).
- **Out (deferred to a later/separate roadmap):** ReXGlue recomp, NHL2K10 RE, rig-audit,
  head-import (see [MESH_IMPORT_EXCLUSIVE_PATH.md](MESH_IMPORT_EXCLUSIVE_PATH.md)), NHL24
  weight-remap, FMT plugins, EBOOT tunable patcher.

> **Scope delta vs. the reference app.** The reference app proved **Legacy/14/15** on
> **PS3/Xbox/Recomp**. The RC target adds **PS4** (net-new: little-endian x86 + a different save
> container) and the **full NHL 10–21 title range** (per-title schema reversing for years the
> reference app did not cover). These are net-new work, not ports — tracked explicitly in
> [ROADMAP.md](ROADMAP.md) and [01-scope-and-compatibility.md](01-scope-and-compatibility.md).

## Stack & rationale

The single most valuable asset in the reference app is the **Rust engine**: byte-exact TDB
parse/serialize with CRC recomputation, Huffman type-66 pool rebuild, save-container checksum
reversal, and the edit/validation model — verified byte-for-byte against real game saves and
in-game. Rewriting it in another language would re-litigate years of reverse-engineering with no
upside.

| Layer | Recommendation | Rationale |
|---|---|---|
| **Binary engine** | **Rust** | Proven, byte-exact, memory-safe, excellent test story. |
| **Service / IPC** | **Tauri v2** | Rust-native IPC, small footprint; v2 cleans up the command/state model. |
| **Frontend** | **React + TanStack Query** | Data-fetching/cache pattern fits; switching UI frameworks buys little. egui considered and rejected (loses web ecosystem, harder rich editors). |
| **Historical store** | **SQLite** (rusqlite) | Already the FHM corpus format; read-mostly. This SQLite corpus **is** the "historical source database (1909→present)". |

**Net:** keep **Rust core + Tauri v2 + React/TanStack + SQLite**, but **re-architect the boundaries**
rather than port the existing module graph verbatim. The value is in re-layering, not re-languaging.

## Target architecture (4 layers + sidecars)

```
┌─────────────────────────────────────────────────────────────────┐
│  app/  (React + TS)   — views, TanStack Query hooks, presentation │
└───────────────────────────┬─────────────────────────────────────┘
                            │  stable typed command + DTO contract (codegen'd types)
┌───────────────────────────▼─────────────────────────────────────┐
│  studio-api/  (Rust)  — Tauri command surface, project/session    │
│                         registry, DTOs, convert, orchestration     │
└───────────────────────────┬─────────────────────────────────────┘
                            │  domain operations (typed, GUI-agnostic)
┌───────────────────────────▼─────────────────────────────────────┐
│  studio-domain/  (Rust) — entity ops, import pipeline, validation │
│                           tiers, view-models, formulas, asset jobs │
└───────────────────────────┬─────────────────────────────────────┘
                            │  byte-level primitives
┌───────────────────────────▼─────────────────────────────────────┐
│  ENGINE crates (Rust) — tdb-core, tdb-edit, tdb-defs, tdb-savedata │
│                         tdb-enums, tdb-relations, codecs           │
└──────────────────────────────────────────────────────────────────┘
  sidecars: historical corpus (SQLite), instance registry (SQLite), bundled resources
```

### Design rules carried forward (keep explicit)

- **Engine never mutates parser truth.** Edits are staged; serialization patches a byte snapshot.
- **DTO boundary is stable & dumb.** Plain JSON primitives, deterministic field order, round-trip
  tested. No internal Rust types cross the wire.
- **Preview-then-commit** for anything destructive or bulk (imports, global edits, asset installs).
- **Atomic writes** (`.tmp` + rename, backup-on-first-write) with an **external-edit guard** (SHA check).
- **Three validation tiers:** stage-time (range/enum/read-only), operation-time (entity invariants),
  save-time (whole-DB domain audit).

### What changes vs. the reference app

- Collapse the ~25 engine crates into a **smaller, intentional crate set** (below). Many existing
  crates are research artifacts or one-feature splits that add graph complexity without payoff.
- Split the monolithic `tdb-gui-api` (commands.rs ~7k lines) into **`studio-api`** (thin Tauri +
  DTO + registry) and **`studio-domain`** (pure orchestration, fully testable without Tauri).
- **Generate TS DTO types from Rust** (`ts-rs` or `specta`) instead of hand-maintaining `dto.ts` —
  kills an entire class of drift bugs.
- Consolidate the frontend `presentation/*` grab-bag into a small number of cohesive modules.

## Engine crate set (consolidated)

Port logic faithfully — it is verified. Extend the codec/schema/platform layers for the new
RC scope (PS4 + full title range).

| Crate | Responsibility | Notes |
|---|---|---|
| **tdb-core** | TDB parse/serialize, header/table/field/record model, CRC, Huffman type-66, RawId namespace reversal, endian | Crown jewel. Endian handling extends to PS4 (little-endian x86). |
| **tdb-edit** | `EditSession`, `EditValue`, field/row ops, undo/redo, stage-time `validate_edit`, type-66 pool rebuild, swap-pop orphan guard | Fold `tdb-expand` (capacity growth) in here or adjacent. |
| **tdb-defs** | `FieldMeta` index (name/range/bit-depth/update/fk), multi-flavor loader, byte-reversed-namespace bridge | Metadata backbone. **Extends to per-title schemas across NHL 10–21.** |
| **tdb-savedata** | PS3/Xbox save containers (v1/v2/v4), extract + pack, checksum reversal | **Add PS4 save container + finish v1 / Xbox-STFS write-back.** |
| **tdb-enums** | Enum value↔label registry (TOML) | Small. |
| **tdb-relations** | Confirmed FK / relationship registry + (optional) discovery | Keep confirmed registry; demote heuristic discovery to a dev-only tool. |
| **tdb-domain-validation** | Whole-DB integrity audit + single-edit guard | Integrity engine. |
| **tdb-view-models** | UI-ready read models (Player/Team/Roster/Line) joined via confirmed relationships | Trim to what the UI consumes. |
| **codecs** (`tdb-archive-big`, `-rpsgl`, `-rx2`, `-audio`) | BIG archives, PS3/Xbox texture containers, music/speech codecs | Keep what the production asset tools need. |
| **tdb-fhm** + **tdb-fhm-store** + **tdb-fhm-build** | FHM CSV → SQLite corpus (the historical source DB); season/career/captaincy/jersey/contract/draft ingest | Build-time pipeline stays a separate bin. |
| **tdb-instance-registry** | SQLite sidecar ledger of imported player-season instances (dedup) | Non-blocking. |

**Drop / archive** (move to a separate `nhl-research` repo): `tdb-rig-audit`, `tdb-nhl24-weights`,
`tdb-vcpack` (2K10), `tdb-eboot`, FMT plugins, recomp work.
**Fold/merge:** `tdb-coverage`, `tdb-semantic`, `tdb-export` → optional dev-tools crate, not core.

### Cross-cutting: the (title, platform) registry

A central registry resolves **(title ∈ NHL 10–21, platform ∈ Xbox|PS3|PS4|Recomp) → schema + codec**
(field layout, endianness, packing, save container). New titles/platforms plug in here rather than
scattering conditionals. Endianness and container differences live in the codec layer and never leak
into `studio-domain` or the UI.

**Capability probing (implemented).** The schema half of this gating is *evidence-based, not
declared*: at load, `studio_domain::Capabilities::probe` inspects the opened database's own field
descriptors + metadata for the optional features the curated views depend on (team→arena link,
draft/junior/UFA bio fields, style enum, ratings-vocabulary coverage). The result ships in
`ProjectSummaryDto` (`capabilities`, plus the onboarding `game_id`), and the UI hides or hedges the
surfaces the loaded game doesn't carry (e.g. NHL 10–12 has no `kOtt.buDa` arena link; NHL 10
predates draft data). Probing the file — never the title id — keeps unproven (title, platform)
combinations honest by construction. Cross-title presence facts come from the 21-anchor schema diff
(2026-07-03) and are asserted in `crates/studio-domain/tests/capabilities.rs`.

**Era-spanning ratings (implemented).** Player/goalie ratings are classified by **metadata name**,
not field id — EA renamed the family at NHL 14 (`p_speed`/`g_gsh` → `c_speed`/`c_gsh`, new ids,
stable suffixes) — and the value **scale comes from the field's declared range**: PS3/X360 store
6-bit `0..63` (display/edit `+36` → 36..99; in-game-verified for 14/15/Legacy, provisional for
10–13), while every PS4 title stores 7-bit `0..99` — the human value directly. See
`studio_domain::attributes::rating_offset` and `crates/studio-domain/tests/ratings_scale.rs` (the
regression guard for the id-keyed `+36` that silently corrupted PS4 rating display/edit).

## Service layer — `studio-api` + `studio-domain`

**`studio-api`** (Tauri-facing, thin):
- **Project/session registry** — `ProjectHandle` (parsed `TdbFile`, defs, enums, relationships,
  `EditSession`, cached `FieldMetaIndex`, cache dirs, role). `ProjectRole = Nhlng | Roster |
  Localization`. `Platform = Ps3 | Xbox | Ps4 | Recomp`. Per-command brief lock scoping.
- **DTOs** grouped by family (project, editing, player, team, roster, entity, fhm, asset, audio,
  loc, formulas, bulk, savedata), with generated TS counterparts.
- **convert.rs** — view-model/internal ↔ DTO bridge; keep `raw_id_to_string`/`parse_raw_id`.
- **Tauri command registration** — one thin wrapper per command, forwarding to `studio-domain`.

**`studio-domain`** (pure, no Tauri — fully unit-testable):
- **Edit/session orchestration** — load → begin session → apply/revert/undo/redo → validate → save.
- **Entity ops** — `with_repairing_transaction` (snapshot → stage → auto-repair → audit →
  commit/rollback): move_slot, trade_slots, add/clone/delete player, free-agent, hide/unhide,
  retire/unretire jersey; atomic roster-block realignment.
- **Import pipeline** — two-phase `preview_change_set` / `commit_change_set` over an
  `ImportChangeSet` PLAN; both lower through the **same** `lower()` so preview and commit are
  byte-identical (golden-test invariant). Emits `ChangeSetPreviewDto`.
- **Formula engine** — FHM→EA rating compute, normalisation modes, archetype classification,
  profile save/load.
- **Asset jobs** — portraits, logos, textures, jersey/equipment clone, hockey-shop install, audio
  preview/replace. Each is preview-then-apply with a pending-writes manifest.
- **Bulk tools** — blank template, global field set, career stats clear/rebuild, asset backfill.

## Feature inventory → module map

Functional checklist for the rebuild (each row = a view/feature + backing systems).

### EA DB editing
| View / Feature | Frontend | Backing |
|---|---|---|
| **Dashboard** — summary, stats, validation log w/ inline repair | `DashboardPage` | project_summary, domain audit, repair ops |
| **Teams** master-detail (identity, jerseys, equipment, strategies, captaincy) | `TeamsPage` + tabs | view-models (Team), enums, jersey clone, strategies, draft picks |
| **Arenas** master-detail | `ArenasPage` | view-models (Arena) |
| **Players** master-detail (bio, ratings, career stats, roster memberships, portrait) | `PlayersPage` + tabs | view-models (Player), field-meta range validation, career-stats rows |
| **Lines / depth chart** (situational groups, per-slot archetype, captaincy, in-minors/scratched) | line view | `linePresentation`, joins, NHLViewNG parity edits |
| **Transactions** — two-team move/copy/free-agent/delete | `TransactionsPage` | entity ops (atomic, integrity-wrapped) |
| **Pending Edits** — review/revert/validate, save / save-as / pack save | `PendingEditsPage` | session summary, domain pre-flight, save spine, pack containers |

### Historical (FHM) + import
| View / Feature | Frontend | Backing |
|---|---|---|
| **Historical Teams** (cross-season, contracts, jersey/captaincy, depth chart) | `HistoricalTeamsPage` | fhm-store readers |
| **Historical Players** (bio, career, computed EA ratings, archetype, send-to-builder) | `HistoricalPlayersPage` | fhm-store, formula engine |
| **Formulas** — drag/drop FHM→EA rating builder, normalisation, live preview, profiles | `FormulasPage` | formula engine, attribute maxes, rating tiers |
| **Roster Builder / Checkout** — cart of FHM players/teams → match → preview → commit | `RosterBuilderPage` + modals | import pipeline, match_player (slug index), capacity, portrait match |

### Production asset tools
| View / Feature | Frontend | Backing |
|---|---|---|
| **Hockey Shop / Equipment Studio** — per-slot scan + 3D preview + store + clone + install | `EquipmentStudioPage` + `SlotShopStudio` | equipment slots, rx2/rpsgl codecs, clone/install jobs |
| **Appearance Studio** — head browser + 3D, eyes/skin enums, hair graft/bake | `AppearanceStudioPage` | head/hair preview, bake job |
| **Texture Lab** — cache index, decode preview, replace-from-PNG, used-by lookup, bulk export | `TextureLabPage` | codecs, asset-reference index, pending writes |
| **Portraits / Team logos** — DDS↔PNG preview/replace, logo variants | inline + team detail | FE-cache resolver, portrait/logo jobs |
| **Localization** — string table edit (type-66 pool-fit constraints) | `LocalizationEditorPage` | tdb-localization, type-66 pool rebuild |
| **Music / Arena PA / Play-by-Play** — slot list, preview, replace (EALayer3 / Speex) | `MusicEditorPage`, `SpeechBankEditorPage` | audio codecs, bundled sx.exe + vgmstream |

### Shell / cross-cutting
| Feature | Frontend | Backing |
|---|---|---|
| **Startup gate / onboarding** — platform select, emulator-root validate, vanilla-save import, `.big` extract, load nhlng + roster | `StartupModal`, `OnboardingWizard` | onboarding validate/scan, big extraction, save extract |
| **Builds** — copy-on-switch snapshots of install + save | `BuildsCard` | build create/list/switch/delete |
| **Settings** — portable config, bulk tools, exports | `SettingsPage` | settings.json sidecar, bulk tools, export |
| **Session header** — dirty/error/warning counts, undo/redo/save/discard, pending badge | `AppShell` | edit-session state |
| **Roster capacity normalization** — auto-grow trimmed roster tables to vanilla-save ceilings on load | (automatic) | tdb-expand + bundled vanilla saves |

## Cross-cutting systems (carry forward)

1. **Edit model** — `EditSession` over a parsed snapshot; `FieldEdit` (original vs current), row
   ops (append/swap-pop-delete/truncate), undo/redo (bounded), deterministic JSON serialization.
2. **Validation tiers** — (a) stage-time per-edit: kind/bit-width/range/enum/FK/read-only via
   `FieldMeta`; (b) operation-time: entity invariants in repairing transactions; (c) save-time:
   whole-DB domain audit (companion model, roster-block coherence, dup-on-team, dangling FKs).
3. **FieldMeta backbone** — canonical (table,field)→meta index; range bounds (signed-aware),
   update/read-only flag, FK parent. Extends per-title across NHL 10–21.
4. **Save spine** — produce bytes → parse+audit → external-edit guard → atomic write+backup →
   refresh handle → drop session.
5. **Save containers** — detect by body-marker (not the version field, which lies); extract inflates
   zlib, pack deflates + recomputes checksums, pads to allocation. Adds PS4 container handling.
6. **Import pipeline** — `ImportChangeSet` PLAN → single `lower()` shared by preview & commit →
   `ChangeSetPreviewDto`; instance-registry ledger for dedup (non-blocking).
7. **Project/session registry** — per-role handles, brief per-command locks, lazy `EaSlugIndex`.

## What to drop / restructure (cruft cleanup)

- **Drop from new repo:** recomp (ReXGlue), NHL2K10 RE (`tdb-vcpack`), rig-audit, head-import,
  NHL24 weight-remap, FMT plugins, EBOOT tunable editor. Archive in a `nhl-research` repo.
- **Split** `tdb-gui-api` → `studio-api` + `studio-domain`.
- **Generate** TS DTOs from Rust (`ts-rs`/`specta`); delete hand-written `dto.ts`.
- **Consolidate** frontend `presentation/*` into ~5 cohesive modules (formatting, line/roster,
  field-access, catalogs, errors).
- **Demote** discovery/semantic/coverage/export tooling to an optional dev-tools crate + dev UI.
- **Unify** the duplicated save extract/pack pairs and asset preview/clone/install commands behind
  generic slot/asset abstractions.

## Verification strategy

- **Engine:** port the existing test suites verbatim first (round-trip, CRC, Huffman pool, save
  container byte-equality, edit validation) — the parity guardrails must stay green.
- **Golden byte-equality vs. reference app:** for representative edits/imports, materialize bytes in
  both repos and assert identical output ("new == old"; the pipeline's "preview == commit == legacy"
  invariant generalizes).
- **DTO round-trip:** every DTO serialize→deserialize→equal; codegen keeps TS in lockstep.
- **Domain audit:** load known-clean and known-dirty saves; assert exactly the expected flags.
- **End-to-end smoke:** load a real emulator/PS4 save → edit → save → pack container → load in
  emulator/game for parity-critical paths.

## Risks

- **Hidden coupling** in the reference app's commands.rs (7k lines) — port behind the domain test
  harness, not copy-paste.
- **Asset codec edge cases** (rpsgl mip tails, type-66 pool-fit) — port with regression fixtures.
- **Save-container variants** — v1/Xbox write-back unfinished in the reference; PS4 is net-new.
- **Full title-range scope** — per-title schema reversing across NHL 10–21 is unproven for years the
  reference app did not cover; treat as discovery work, gate by verified support.

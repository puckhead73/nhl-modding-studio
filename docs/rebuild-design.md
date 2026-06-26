# Design Doc & Roadmap — NHL Database Studio v2 (Greenfield Rebuild)

## Context

`nhl-database-studio` is a mature Tauri + React + Rust desktop app for editing EA NHL "TDB" binary
databases (NHL Legacy / 14 / 15 family) and their console roster-save containers. It works, and it
encodes a large amount of hard-won, byte-exact knowledge — especially the **NHLViewNG-parity
save/write path**: CRC-correct TDB serialization, PS3/Xbox save-container pack/unpack (v1/v2/v4),
the staged edit model with three-tier validation, the domain-integrity engine, and the FHM
historical import pipeline.

But it grew by accretion. Multiple rewrites and research spikes (recomp, NHL2K10 RE, rig-audit,
head-import, FMT plugins) are layered into the same tree as the production studio. The gui-api
surface has ~140 commands with overlapping responsibilities, the frontend presentation layer has
organically-grown formatting modules, and the experimental asset-research code is interleaved with
shipping features.

**Goal:** treat the current app as a validated *rough draft / reference implementation* and stand up
a **fresh repository** with a clean, well-layered architecture that carries forward 100% of the
**core studio + production-tool** functionality, built on the proven parity save/write work. This
document is the roadmap for that rebuild.

**Decisions locked (from the user):**
1. **Posture:** cleaned-up redesign — leaner layering, name what to keep, explicitly drop/restructure cruft.
2. **Scope:** *core studio + production tools*. **In:** DB editing (players/teams/arenas/jerseys),
   roster/lines/transactions, parity save+write, FHM historical import, EA rating formulas, and the
   *shipped* asset tools (portraits, textures, equipment/hockey-shop, localization, music/PA/PBP audio).
   **Out (deferred to a later/separate roadmap):** ReXGlue recomp, NHL2K10 RE, rig-audit, head-import,
   NHL24 weight-remap, FMT plugins, EBOOT tunable patcher.
3. **Stack:** evaluate from scratch (incl. non-Rust engine options) — recommendation below.

---

## Stack Evaluation & Recommendation

The single most valuable asset in the rough draft is the **Rust engine**: byte-exact TDB parse/serialize
with CRC recomputation, Huffman type-66 pool rebuild, save-container checksum reversal, and the
edit/validation model. This is verified against real game saves (byte-for-byte vs. NHL12/13; in-game
validated). Rewriting it in another language would re-litigate years of reverse-engineering with no
upside.

| Layer | Options weighed | Recommendation | Rationale |
|---|---|---|---|
| **Binary engine** | Rust / Zig / C++ / Go | **Rust** | Proven, byte-exact, memory-safe, excellent test story. No reason to move. |
| **Service/IPC** | Tauri / Electron+native-addon / web server + local Rust daemon | **Tauri v2** | Rust-native IPC, small footprint, already the boundary. v2 cleans up the command/state model. |
| **Frontend** | React / Svelte / SolidJS / native (egui) | **React + TanStack Query** (keep), reconsider only if team prefers | The data-fetching/cache pattern is a good fit; the cost of switching UI frameworks is high and buys little. egui considered and rejected (loses web-tech ecosystem, harder for rich editors). |
| **Historical store** | SQLite / DuckDB / embedded KV | **SQLite** | Already the FHM corpus format; rusqlite is solid; read-mostly. |

**Net:** keep **Rust core + Tauri v2 + React/TanStack**, but *re-architect the boundaries* rather than
port the existing module graph verbatim. The value is in re-layering, not re-languaging.

---

## Target Architecture (4 layers + sidecars)

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

**Design rules carried forward (these were good in the rough draft — keep them explicit):**
- **Engine never mutates parser truth.** Edits are staged; serialization patches a byte snapshot.
- **DTO boundary is stable & dumb.** Plain JSON primitives, deterministic field order, round-trip tested.
  No internal Rust types cross the wire.
- **Preview-then-commit** for anything destructive or bulk (imports, global edits, asset installs).
- **Atomic writes** (`.tmp` + rename, backup-on-first-write) with an **external-edit guard** (SHA check).
- **Three validation tiers:** stage-time (range/enum/read-only), operation-time (entity invariants),
  save-time (whole-DB domain audit).

**What changes vs. the rough draft:**
- Collapse the ~25 engine crates into a **smaller, intentional crate set** (see below). Many existing
  crates are research artifacts or one-feature splits that add graph complexity without payoff.
- Split today's monolithic `tdb-gui-api` (commands.rs ~7k lines, tauri_adapter ~2.6k lines) into
  **`studio-api`** (thin Tauri + DTO + registry) and **`studio-domain`** (pure orchestration, fully
  testable without Tauri). Today this split is implicit; make it a crate boundary.
- **Generate TS DTO types from Rust** (e.g. `ts-rs` or `specta`) instead of hand-maintaining
  `app/src/types/dto.ts` — kills an entire class of drift bugs.
- Consolidate the frontend `presentation/*` grab-bag into a small number of cohesive modules.

---

## Engine Crate Set (consolidated)

Keep these as the byte-level foundation (port logic faithfully — it's verified):

| Crate | Responsibility | Notes vs. rough draft |
|---|---|---|
| **tdb-core** | TDB parse/serialize, header/table/field/record model, CRC, Huffman type-66, RawId namespace reversal, endian | Keep as-is; this is the crown jewel. |
| **tdb-edit** | `EditSession`, `EditValue`, field/row ops, undo/redo, stage-time `validate_edit`, type-66 pool rebuild, swap-pop orphan guard | Keep. Fold `tdb-expand` (capacity growth) in here or adjacent. |
| **tdb-defs** | `FieldMeta` index (name/range/bit-depth/update/fk), multi-flavor loader (nhlng-meta + nhl14-editor), byte-reversed-namespace bridge | Keep. This is the metadata backbone the parity work depends on. |
| **tdb-savedata** | PS3/Xbox save containers (v1/v2/v4), extract + pack, checksum reversal | Keep. Roadmap: finish v1 + Xbox-STFS write-back (see gaps). |
| **tdb-enums** | Enum value↔label registry (TOML) | Keep, small. |
| **tdb-relations** | Confirmed FK / relationship registry + (optional) discovery | Keep confirmed registry; demote heuristic discovery to a dev-only tool. |
| **tdb-domain-validation** | Whole-DB integrity audit + single-edit guard (companion model, roster blocks, dup-on-team, dangling pointers) | Keep. This is the integrity engine. |
| **tdb-view-models** | UI-ready read models (Player/Team/Roster/Line) joined via confirmed relationships | Keep, but trim to what the UI actually consumes. |
| **codecs** (`tdb-archive-big`, `-rpsgl`, `-rx2`, `-audio`) | BIG archives, PS3/Xbox texture containers, music/speech codecs | Keep the ones the production asset tools need. |
| **tdb-fhm** + **tdb-fhm-store** + **tdb-fhm-build** | FHM CSV → SQLite corpus; season/career/captaincy/jersey/contract/draft ingest | Keep. Build-time pipeline stays a separate bin. |
| **tdb-instance-registry** | SQLite sidecar ledger of imported player-season instances (dedup) | Keep, non-blocking. |

**Drop / archive from the new repo** (move to a separate `nhl-research` repo or leave in the old tree):
`tdb-rig-audit`, `tdb-nhl24-weights`, `tdb-vcpack` (2K10), `tdb-eboot`, FMT plugins, recomp work.
**Fold/merge:** `tdb-coverage`, `tdb-semantic`, `tdb-export` → optional dev-tools crate, not core.

---

## Service Layer — `studio-api` + `studio-domain`

**`studio-api`** (Tauri-facing, thin):
- **Project/session registry** — `ProjectHandle` (parsed `TdbFile`, defs, enums, relationships,
  `EditSession`, cached `FieldMetaIndex`, cache dirs, role). `ProjectRole` = `Nhlng | Roster | Localization`.
  `Platform` = `Ps3 | Xbox | Recomp`. Per-command brief lock scoping (carry forward — it works).
- **DTOs** grouped by family (project, editing, player, team, roster, entity, fhm, asset, audio, loc,
  formulas, bulk, savedata). Generated TS counterparts.
- **convert.rs** — view-model/internal ↔ DTO bridge. Keep `raw_id_to_string`/`parse_raw_id` helpers.
- **Tauri command registration** — one thin wrapper per command, forwarding to `studio-domain`.

**`studio-domain`** (pure, no Tauri — fully unit-testable):
- **Edit/session orchestration** — load → begin session → apply/revert/undo/redo → validate → save spine.
- **Entity ops** — `with_repairing_transaction` wrapper (snapshot → stage → auto-repair → audit →
  commit/rollback). Ops: move_slot, trade_slots, add/clone/delete player, free-agent, hide/unhide,
  retire/unretire jersey. Carry the atomic roster-block realignment (toWX block invariant).
- **Import pipeline** — two-phase `preview_change_set` / `commit_change_set` over an `ImportChangeSet`
  PLAN model; both lower through the **same** `lower()` so preview and commit are byte-identical
  (golden-test invariant). Emits `ChangeSetPreviewDto` (diffs, validation, capacity, same-person,
  storage blockers).
- **Formula engine** — FHM→EA rating compute, normalisation modes (nominal / position-scoped max),
  archetype classification, profile save/load.
- **Asset jobs** — portraits, logos, textures, jersey/equipment clone, hockey-shop install, hair bake,
  audio preview/replace. Each is preview-then-apply with a pending-writes manifest.
- **Bulk tools** — blank template, global field set, career stats clear/rebuild, asset backfill.

---

## Feature Inventory → Module Map (the views/features to rebuild)

This is the functional checklist for the rebuild. Each row = a view/feature with its backing systems.

### EA DB editing
| View / Feature | Frontend | Backing domain/engine |
|---|---|---|
| **Dashboard** — project summary, stats, validation log w/ inline repair tools | `DashboardPage` | project_summary, domain audit, repair ops |
| **Teams** master-detail (identity, jerseys, equipment, strategies, captaincy) | `TeamsPage` + team detail tabs | view-models (Team), enums, jersey clone, strategies, draft picks |
| **Arenas** master-detail | `ArenasPage` | view-models (Arena) |
| **Players** master-detail (bio/ubPc, ratings/dSvy+mHuy, career stats, roster memberships, portrait) | `PlayersPage` + tabs | view-models (Player), field-meta range validation, career-stats rows |
| **Lines / depth chart** (situational line groups, per-slot archetype, captaincy badges, in-minors/scratched status) | line view components | `linePresentation`, eGlu/ZBac joins, NHLViewNG parity edits |
| **Transactions** — two-team move/copy/free-agent/delete | `TransactionsPage` | entity ops (atomic, integrity-wrapped) |
| **Pending Edits** — review/revert/validate, save / save-as / pack PS3/Xbox save | `PendingEditsPage` | session summary, domain pre-flight, save spine, pack containers |

### Historical (FHM) + import
| View / Feature | Frontend | Backing |
|---|---|---|
| **Historical Teams** (cross-season, contracts, jersey/captaincy, depth chart) | `HistoricalTeamsPage` | fhm-store readers |
| **Historical Players** (bio, career, computed EA ratings, archetype, send-to-builder) | `HistoricalPlayersPage` | fhm-store, formula engine |
| **Formulas** — drag/drop FHM→EA rating builder, normalisation modes, live preview, profiles | `FormulasPage` | formula engine, attribute maxes, `ratingColor` tiers |
| **Roster Builder / Checkout** — cart of FHM players/teams → match → preview → commit | `RosterBuilderPage` + modals | import pipeline (preview/commit), match_player (slug index), capacity, portrait match |

### Production asset tools
| View / Feature | Frontend | Backing |
|---|---|---|
| **Hockey Shop / Equipment Studio** — per-slot scan + 3D preview + store + clone + install-to-game | `EquipmentStudioPage` + `SlotShopStudio` | equipment slots, rx2/rpsgl codecs, clone/install jobs |
| **Appearance Studio** — head browser + 3D, eyes/skin enums, hair graft/bake | `AppearanceStudioPage` | head/hair preview, bake job |
| **Texture Lab** — cache walk/index, decode preview, replace-from-PNG, used-by reverse lookup, bulk export | `TextureLabPage` | codecs, asset-reference index, pending writes |
| **Portraits / Team logos** — DDS↔PNG preview/replace, logo variant generation | inline + team detail | FE-cache resolver, portrait/logo jobs |
| **Localization** — vCJG string table edit (type-66 pool-fit constraints) | `LocalizationEditorPage` | tdb-localization, type-66 pool rebuild |
| **Music / Arena PA / Play-by-Play** — slot list, preview, replace (EALayer3 / Speex) | `MusicEditorPage`, `SpeechBankEditorPage` | audio codecs, bundled sx.exe + vgmstream |

### Shell / cross-cutting
| Feature | Frontend | Backing |
|---|---|---|
| **Startup gate / onboarding** — platform select, emulator-root validate, vanilla-save import, `.big` extract, load nhlng + roster | `StartupModal`, `OnboardingWizard` | onboarding validate/scan, big extraction, save extract |
| **Builds** — copy-on-switch snapshots of install + save | `BuildsCard` | build create/list/switch/delete |
| **Settings** — portable config (USRDIR, caches, normalisation), bulk tools, exports | `SettingsPage` | settings.json sidecar, bulk tools, export |
| **Session header** — dirty/error/warning counts, undo/redo/save/discard, pending badge | `AppShell` | edit-session state |
| **Roster capacity normalization** — auto-grow trimmed roster tables to vanilla-save ceilings on load | (automatic) | tdb-expand + bundled vanilla saves |

---

## Cross-Cutting Systems (carry forward, re-stated cleanly)

1. **Edit model** — `EditSession` over a parsed snapshot; `FieldEdit` (original vs current), row ops
   (append/swap-pop-delete/truncate), undo/redo (bounded history), deterministic JSON serialization.
2. **Validation tiers** — (a) stage-time per-edit: kind/bit-width/range/enum/FK/read-only via
   `FieldMeta`; (b) operation-time: entity invariants in repairing transactions; (c) save-time:
   whole-DB domain audit (companion model, roster-block coherence, dup-on-team, dangling FKs).
3. **FieldMeta backbone** — canonical (table,field)→meta index merged from nhlng-meta (byte-reversed
   namespace) + nhl14-editor; carries range bounds (signed-aware), update/read-only flag, FK parent.
4. **Save spine** — produce bytes → parse+audit → external-edit guard → atomic write+backup →
   refresh handle (reuse audited parse) → drop session.
5. **Save containers** — v1/v2/v4 detection by body-marker (not version field, which lies);
   extract inflates zlib, pack deflates + recomputes checksum_a (CRC-32) / checksum_b (CRC-32 BZIP2),
   pads to allocation.
6. **Import pipeline** — `ImportChangeSet` PLAN → single `lower()` shared by preview & commit →
   `ChangeSetPreviewDto`; instance-registry ledger for dedup (non-blocking).
7. **Project/session registry** — per-role handles, brief per-command locks, lazy `EaSlugIndex`
   (slug→EA player, ambiguity-aware).

---

## What to Drop / Restructure (cruft cleanup)

- **Drop from new repo:** recomp (ReXGlue), NHL2K10 RE (`tdb-vcpack`), rig-audit, head-import,
  NHL24 weight-remap, FMT plugins, EBOOT tunable editor. Archive in a `nhl-research` repo.
- **Split** `tdb-gui-api` → `studio-api` (Tauri/DTO/registry) + `studio-domain` (pure orchestration).
- **Generate** TS DTOs from Rust (`ts-rs`/`specta`); delete hand-written `dto.ts` drift surface.
- **Consolidate** frontend `presentation/*` (`ratingColor`, `linePresentation`, `*Presentation`,
  `fieldAccess`, `bulkToolsCatalog`, etc.) into ~5 cohesive modules (formatting, line/roster,
  field-access, catalogs, errors).
- **Demote** discovery/semantic/coverage/export tooling to an optional dev-tools crate + dev-mode UI.
- **Unify** the duplicated save extract/pack command pairs and the asset preview/clone/install
  commands behind generic slot/asset abstractions (the rough draft already started this with
  `SlotShopStudio` + slot-parameterized backends — finish it).

---

## Phased Roadmap

**Phase 0 — Repo scaffold & contracts.** New cargo workspace + Tauri v2 + Vite/React. Wire
`ts-rs`/`specta` DTO codegen. CI: `cargo test`, `tsc`, lint. Bundle resources layout (games/, vanilla-saves/, corpus/).

**Phase 1 — Engine port.** Bring over `tdb-core`, `tdb-edit`, `tdb-defs`, `tdb-enums`,
`tdb-relations`, `tdb-domain-validation`, `tdb-savedata` with their tests. Gate: full test suite green,
round-trip + CRC + save-container byte-equality tests pass.

**Phase 2 — Service skeleton.** `studio-api` registry + project load/save spine + `studio-domain`
edit-session orchestration. Gate: load real nhlng.db, stage an edit, validate, save, byte-diff vs.
rough-draft output.

**Phase 3 — EA DB core views.** Dashboard, Teams, Arenas, Players (+ career stats), Lines/depth chart,
Pending Edits, save-as/pack. This delivers a usable editor.

**Phase 4 — Roster ops & transactions.** Entity ops (repairing transactions), Transactions view,
roster capacity normalization, repair tools.

**Phase 5 — Historical + import.** FHM corpus build, Historical Teams/Players, Formulas, Roster
Builder (preview/commit pipeline + instance registry).

**Phase 6 — Production asset tools.** Texture Lab, Portraits/Logos, Equipment/Hockey Shop, Appearance
Studio, Localization, Music/PA/PBP.

**Phase 7 — Shell polish & onboarding.** Startup/onboarding, Builds, Settings + bulk tools/exports,
session header UX.

**Phase 8 — Parity gap closure.** Finish deferred parity items: v1 + Xbox-STFS save write-back,
companion-row auto-creation on position flip, multi-select transactions, clone varchar-name edit.

---

## Verification Strategy

- **Engine:** port the existing test suites verbatim first (round-trip, CRC, Huffman pool, save
  container byte-equality, edit validation). These are the parity guardrails — they must stay green.
- **Golden byte-equality vs. rough draft:** for representative edits/imports, materialize bytes in both
  the old and new repo and assert identical output (the import pipeline's "preview == commit == legacy"
  invariant generalizes to "new == old").
- **DTO round-trip:** every DTO serialize→deserialize→equal; codegen keeps TS in lockstep.
- **Domain audit:** load known-clean and known-dirty saves; assert the integrity engine flags exactly
  the expected issues.
- **End-to-end smoke:** load a real RPCS3/Xenia save → edit a player → save → pack container →
  (manually) load in emulator/game for the parity-critical paths.

## Risks
- **Hidden coupling** in the rough draft's commands.rs (7k lines) — mitigate by porting behind the
  domain test harness, not copy-paste.
- **Asset codec edge cases** (rpsgl mip tails, type-66 pool-fit) — port with their regression fixtures.
- **Save-container variants** — v1/Xbox write-back is still unfinished in the rough draft; treat as new
  work in Phase 8, not a port.

## Deliverable
On approval, write this as the canonical roadmap doc in the **new repo** (e.g. `docs/DESIGN.md` +
`docs/ROADMAP.md`), and optionally drop a copy into the current repo at
`docs/rebuild-design.md` for reference. The rebuild itself proceeds by the phases above.

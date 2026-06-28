# Phase 3 — EA DB core views (implementation plan)

> **Status:** ready to execute. Authored 2026-06-28 after Phase 2 (commit `c2a54c2`).
> **Gate (ROADMAP.md:35):** a *usable editor* for the proven titles — Dashboard, Teams, Arenas,
> Players (+career stats), Lines/depth chart, Pending Edits, save-as/pack.
>
> **Locked decisions:** (1) full editor — read views **plus** validated inline field editing incl.
> name writes, through the audited session; (2) **thin vertical slice first** (Players end-to-end),
> then breadth; (3) frontend = **React Router + CSS Modules + TanStack Query** (Query mandated by
> DESIGN.md).
>
> Each phase below is self-contained for execution in a fresh context. Frame tasks as **copy the
> cited pattern**, not "transform existing code". Verify format facts against in-repo anchors; never
> assert binary layout you have not confirmed (CLAUDE.md hard rule 3).

---

## Phase 0 — Documentation discovery (consolidated; read before any coding)

### 0.1 The id-convention bridge (THE bug-class to prevent)

There are **two id namespaces** for the same 4-char tags, related by character reversal:

| Namespace | Used by | Example |
|---|---|---|
| **Studio** ids | `tdb-relations` (`RosterModel`, `ids::*`), `tdb-enums`, `tdb-domain-validation`, `TdbFile::table_canonical`, `RawId::canonical(endian)` | `ubPc`, `eGlu`, `dXSB`, `wBIz` |
| **On-disk** ids (= EA meta `shortname`) | the raw bytes, `TdbFile::table`/`Table::field`, **`FieldMetaIndex::field(table, field)`** | `cPbu`, `ulGe`, `BSXd`, `zIBw` |

- Reverse with `tdb_enums::reverse_id(&str) -> String` (`tdb-enums/src/lib.rs:132`).
- **Rule:** to fetch a field's `FieldMeta`, pass **on-disk** ids: `meta.field(&reverse_id(studio_table), &reverse_id(studio_field))`. To read the value from a parsed table, resolve the `FieldDescriptor` by `f.id.canonical(db.endian) == studio_field` (the pattern already used throughout `entity.rs`).
- Verified reversed pairs (sanity anchors): `toWX↔XWot`, `XSWT↔TWSX`, `dXSB↔BSXd`, `sVRt↔tRVs`, `mCcl↔lcCm`, `wBIz↔zIBw`, `vjla↔aljv`.

### 0.2 Allowed backend APIs (exact signatures; do not invent)

**tdb-core** (`crates/tdb-core/src/lib.rs`)
- `TdbFile::parse(&[u8]) -> Result<TdbFile, TdbError>` (:150); fields `endian`, `db_size`, `tables`.
- `TdbFile::table_canonical(&self, studio_id: &str) -> Option<&Table>` (:252); `table(raw_id)` (:246).
- `TdbFile::read_u64(&self, &Table, index: u32, &FieldDescriptor) -> Option<u64>` (:260).
- `TdbFile::read_inline_str(&self, &Table, index, &FieldDescriptor) -> Option<&[u8]>` (:279) — **byte-aligned only** (offset & width multiples of 8); returns NUL-trimmed bytes.
- `TdbFile::set_u64(&self, buf: &mut [u8], &Table, index, &FieldDescriptor, value: u64) -> bool` (:481).
- `TdbFile::recompute_crcs(&self, buf: &mut [u8])` (:424); `serialize()` (:473).
- `write_bits(data, bit_offset, width, value, endian)` (:368); `read_bits(...)` (:301).
- `FieldDescriptor { kind, bit_offset, id: RawId, bit_width }` (:99). `Table { id, fields, current_records, record_block, record_len_bytes, … }` (:112).
- **CONFIRMED ABSENT:** `write_inline_str`. Phase 3.2 adds it. No string-write exists at any layer today.

**tdb-defs** (`crates/tdb-defs/src/lib.rs`)
- `FieldMeta { table, field, name, kind: FieldType, bit_depth, range_low, range_high, key, updatable, fk_parent }` (:34).
- `FieldMeta::signed()` (:53), `neg1_sentinel()` (:65), `decode_raw(u64)->i64` (:70), `encode(i64)->u64` (:86), `validate(i64)->Result<(),ValidationError>` (:96).
- `FieldMetaIndex::field(&self, table, field) -> Option<&FieldMeta>` (:221, **on-disk ids**), `table_name(table)` (:226).
- `FieldType::{Integer, String, Real, Other(String)}`.

**tdb-enums** (`crates/tdb-enums/src/lib.rs`)
- `EnumRegistry::for_field(&self, table, field) -> Option<&EnumDef>` (:172, **studio ids**).
- `EnumRegistry::label(&self, table, field, raw: i64) -> Option<&str>` (:179).
- `EnumDef::label(raw)->Option<&str>` (:68), `raw_of(label)` (:76), `contains(raw)` (:84), `entries: Vec<EnumEntry{raw,label}>`.

**tdb-relations** (`crates/tdb-relations/src/{lib,entity}.rs`)
- `RosterModel::build(&TdbFile) -> Result<RosterModel, EntityError>` (:205). Fields `teams`, `players`, `slots`; `player(id)`, `team(id)`, `team_roster(team_id)`, `slot_player`, `slot_team`.
- `Player { bio_record, player_id, position: Position, kind: PlayerKind, skater_ai, skater_equip, goalie_ai, goalie_equip }` — the `*_ai`/`*_equip` are `Option<u32>` record indexes into `dSvy`/`xmja`/`mHuy`/`fMVl`.
- `RosterSlot { record, slot_key, team_id, index_key, player_id, roster_status, jersey, captaincy }`.
- `entity::ids::*` — full studio-id constant set (BIO=`ubPc`, ROSTER_LINES=`eGlu`, TEAM=`kOtt`, SKATER_AI=`dSvy`, GOALIE_AI=`mHuy`, SKATER_EQUIP=`xmja`, GOALIE_EQUIP=`fMVl`, INDEX=`ZBac`, PLAYER_ID=`wBIz`, POSITION=`vjla`, …).
- `RelationRegistry::resolve(table, field) -> Option<ResolvedEdge>` (:141), `outgoing(table)` (:151).

**studio-domain** (`crates/studio-domain/src/*.rs`)
- `Project { platform, role }` + accessors `db_bytes()`, `source()`, `meta()`, `relations()`, `enums()`, `package(inner)->Result<Vec<u8>>`.
- `EditSession::{begin, project, bytes, is_dirty, undo_depth, redo_depth, undo, redo, overview, model, validate, save, save_to}` + the six Layer-3 ops. `run<R>(op)` is **private** — new edit ops live in `studio-domain` and reuse it.
- Re-exports (`lib.rs`): `Endian`, `Finding/IntegrityReport/Severity`, `OpOutcome`, `Player/RosterModel/RosterSlot/Team`.

**studio-api** (`crates/studio-api/src/*.rs`)
- Pattern triplet to copy for every new feature: a `Registry` method (`registry.rs`) → a `convert::*_dto` fn (`convert.rs`) → a `#[tauri::command]` wrapper (`commands.rs`), then register in **two** places: the `command_handler!` list (`lib.rs:27`) and the `export_typescript` test (`lib.rs:54`).
- Command template: `set_captaincy` (`commands.rs:49`). DTO+convert template: `OpOutcomeDto` (`dto.rs:59`) + `outcome_dto` (`convert.rs:69`). Custom-DTO command: `create_player` (`commands.rs`).
- DTOs derive `#[derive(Serialize, TS)]` (no `#[ts(export)]` attribute — the single `export_typescript` test is the sole bindings writer; keep it that way to stay deterministic).

### 0.3 Allowed frontend APIs (current docs)

- **TanStack Query v5** (`npm i @tanstack/react-query`): `QueryClient`/`QueryClientProvider`; `useQuery({queryKey, queryFn})`; `useMutation({mutationFn, onSuccess})` + `useQueryClient().invalidateQueries({queryKey})`. **Pitfall:** v5 uses `gcTime`, not `cacheTime`. Source: tanstack.com/query/latest.
- **React Router** (`npm i react-router-dom`): use `createBrowserRouter` + `RouterProvider`; app-shell layout route with `<Outlet/>`; `useParams` for `/players/:id`. **Tauri caveat:** if browser-history routing misbehaves on reload in the packaged app, fall back to `createHashRouter` (same API). Source: reactrouter.com.
- **@tauri-apps/api v2:** `import { invoke } from "@tauri-apps/api/core"`; `invoke<T>(cmd, args?) -> Promise<T>`; a Rust `Err(String)` surfaces as a **promise rejection** with that string. Source: v2.tauri.app/develop/calling-rust.
- **CSS Modules:** built into Vite, `*.module.css`, `import styles from "./X.module.css"`. No deps.
- **ts-rs bindings** emit `export type X = { … }` per file with cross-file `import type`. Numbers are `number`; `u64` → `number` (acceptable for our id ranges; note >2^53 is theoretical-only here).
- **⚠️ VERIFY ON FIRST COMMAND (don't assume):** Tauri v2 argument-name casing. Our command params are snake_case (`project_id`, `to_team`). Empirically confirm whether the JS `invoke` args object must be camelCase (`projectId`) or snake_case, and lock the convention in the typed client. This is the first thing Phase 3.4 settles.

### 0.4 Verified format facts (nhllegacy anchor; studio ids unless noted)

- **Names are fixed inline strings** (`FIELDTYPE_STRING`, fixed `depth` in bits, e.g. `ubPc.firstname` 200 bits): read with `read_inline_str`; **edit needs the new `write_inline_str`** (Phase 3.2). Not Huffman varchar.
- **~99% of fields are INTEGER** → `read_u64` + `FieldMeta::decode_raw` (sign/sentinel aware) + `EnumRegistry::label` for enum-bound fields.
- **Team** `kOtt`: `fullname`/`abbrname`/`artabbr`/`shortname` (strings), colors (RGB 8-bit), league/division/conference groups, jerseys, goalie masks, `rivalteam`. **Arena FK** = `kOtt.buDa → StEO.vfEq` (record-index join; per `defs/relationships/kott-buda-to-steo.toml`, audited 2026-05-24 — use `buDa`, *not* a ClpB lookup).
- **Arena** `StEO`: `arenaname`, `cityname`, `arenatype`, `maxattendance`, plus visual/audio/physics ids.
- **Player bio** `ubPc`: key `wBIz`; `firstname`/`lastname`; `height`/`weight`; birth `day`/`month`/`year`, `birthplace`, `intlcountry` (nationality enum), `state` (birth-state enum); `handedness` (enum), `position`=`vjla` (enum); `jersey`; `hasportrait`/`headid` (portrait display only — assets are Phase 6); contract & draft fields.
- **Ratings:** skater `dSvy` (6-bit 0–63 sliders: overall/speed/strength/accel/agility/…); goalie `mHuy` (5-bit signed −16..15 deltas). Join to bio by `wBIz`. The model's `Player.skater_ai`/`goalie_ai` already give the companion record index.
- **Enum-bound (table, field) sites** (19 defs): `(ubPc,vjla)` position, `(ubPc,pkRG)` handedness, `(ubPc,hleL)` nationality, `(ubPc,QvxI)` birth-state, `(ubPc,jfuZ)` eye, `(ubPc,QDTK)` skin, `(eGlu,mCcl)` captaincy, `(dSvy,sFgQ)` player-style, `(mHuy,QgFs)` goalie-style, and the `xmja` equipment enums (stick/glove/helmet/skate/visor/tuck/tape). Use `EnumRegistry::for_field` (studio ids).
- **Lines** `eGlu`: per-slot `playerindex`(`XSWT`)/`team`(`dXSB`)/`jersey`(`sVRt`)/`captain`(`mCcl`)/`playerstyle`(`sFgQ`)/`rosterstatus`, **plus exactly 71 depth-1 situational flags** whose semantics are in the meta `name` attribute (`l1c`, `pp1lw`, `pk4_2ld`, `ot_1c`, shootout `s1..s5`, `g1/g2`, `x1/x2`). The **shortnames are opaque; the names carry meaning** → label slots from `FieldMeta::name`. **Mark in-game correctness as meta-derived/unverified** until an emulator check.
- **Career stats:** stock read-only `ySst` (`goals`/`assists`/`points`/`gamesplayed`/`year`/`shots`/`plusminus`/…), joined by `zIBw`(=`wBIz`). League-scoped variants exist (`CtBM`, `TKKI`, `NFkQ`). **No exhibition-scoped editable career rollup.** → Phase 3.7 displays read-only; editing is out of scope.

### 0.5 Anti-patterns (grep-able guards)

- ❌ Looking up `FieldMeta` with studio ids. ✅ Always `meta.field(&reverse_id(t), &reverse_id(f))`.
- ❌ A new `write_inline_str` that mis-handles width/padding. ✅ Byte-aligned only; overwrite the *full* field byte-width, NUL-pad the remainder, then `recompute_crcs`.
- ❌ Bypassing `with_repairing_transaction` for edits. ✅ Every edit (incl. generic field set) runs through it so the Layer-4 audit fires and it's undoable.
- ❌ Hand-editing `app/src/bindings/*.ts` or `dto.ts`. ✅ Regenerate via `cargo test -p studio-api`.
- ❌ Hardcoding all ~200 fields per entity. ✅ Curate the *semantic* fields for the detail layout; render the remainder meta-driven in a progressive-disclosure "all fields" table (two-audiences rule).
- ❌ Asserting situational-slot or career-stat semantics as in-game-true. ✅ Label as meta-derived; gate "verified" on emulator round-trip.
- ❌ `cacheTime` (React Query v4). ✅ `gcTime` (v5).

---

## Phase 3.1 — Backend: view-model foundation (`studio-domain::view`)

**What to implement**
1. New module `crates/studio-domain/src/view.rs` (declare `mod view;` + re-export in `lib.rs`).
2. A generic, meta-driven **`FieldView`** value type (pure data):
   ```
   FieldView { studio_table, studio_field, name, kind, raw: u64|string, decoded: i64?, display: String,
               enum_label: Option<String>, range_low: Option<i64>, range_high: Option<i64>,
               editable: bool, is_string: bool }
   ```
3. A builder `fn read_field(db, meta, enums, &Table, index, &FieldDescriptor) -> FieldView` that:
   - resolves `FieldMeta` via **on-disk** ids (`reverse_id`), falling back to a minimal view if meta is absent (skip-don't-flag for cross-title robustness);
   - for strings: `read_inline_str` → UTF-8 lossy `display`, `is_string=true`, `editable = meta.updatable`;
   - for integers: `read_u64` → `decode_raw` → `decoded`; `enum_label` via `EnumRegistry::label` (studio ids); `display` = enum label or number; `editable = meta.updatable`.
4. A helper `fn entity_fields(db, meta, enums, studio_table, index) -> Vec<FieldView>` that maps **every** field of a record (meta-driven) — the backbone for the progressive-disclosure "all fields" panel.

**Copy-from**
- Field-by-studio resolution: `entity.rs:171` (`field()` helper) and the `f.id.canonical(db.endian) == studio` pattern.
- Sign/enum decode: `FieldMeta::decode_raw` (`tdb-defs:70`) + `EnumRegistry::label` (`tdb-enums:179`).
- String read + NUL-trim: `tdb-core::read_inline_str` (:279).

**Verification checklist**
- Unit/integration test in `crates/studio-domain/tests/view.rs` against `anchors/ps3/nhllegacy/databases/nhlng.db`:
  - `entity_fields` for an `ubPc` record returns `firstname`/`lastname` as non-empty strings;
  - `vjla` decodes with an `enum_label` of `Center/.../Goalie`;
  - a known signed rating in the player's `mHuy`/`dSvy` companion decodes in range;
  - every `FieldView.editable == false` where `meta.updatable == false`.
- `cargo test -p studio-domain` green; `cargo clippy -p studio-domain` clean.

**Anti-pattern guards:** `reverse_id` used for every meta lookup; no panics on missing meta/field (Option-driven).

---

## Phase 3.2 — Backend: PlayerView + generic validated edit + commands (slice backend)

**What to implement**
1. `view.rs`: `PlayerView` builder `fn player_view(db, meta, enums, model, player_id) -> Option<PlayerView>`:
   - bio identity (curated): name, position(+label), height, weight, nationality(+label), handedness(+label), jersey, hasportrait/headid;
   - ratings: the companion record (`Player.skater_ai`/`goalie_ai`) → `entity_fields` of `dSvy`/`mHuy`;
   - roster memberships: `model.slots` filtered to `player_id` (team_id, jersey, captaincy, status);
   - `all_fields`: `entity_fields(ubPc)` for progressive disclosure.
   - A lightweight `PlayerSummary { player_id, first, last, position_label, team_id }` for the master list.
2. `tdb-core`: add **`write_inline_str(&self, buf, &Table, index, &FieldDescriptor, &str) -> bool`** — byte-aligned guard like `read_inline_str`; write up to `bit_width/8` bytes, NUL-pad remainder, return false if out of range. Add a round-trip unit test (`write_inline_str` then `read_inline_str` == input, and `recompute_crcs` keeps `header_crc_ok`).
3. `studio-domain`: `EditSession::set_field(studio_table, record, studio_field, value: FieldValue) -> Result<OpOutcome>` where `FieldValue = Int(i64) | Text(String)`:
   - run inside `self.run(|tx| …)` (private helper) → a new `EditTransaction` method `set_field_validated` that: looks up `FieldMeta` (reverse ids), `meta.validate(value)` for ints (and length check for strings), encodes, writes via `set_u64`/`write_inline_str`, and lets `with_repairing_transaction` audit. Returns `EditError::Invalid` on range/`NotUpdatable`.
4. `studio-api`: DTOs `PlayerSummaryDto`, `PlayerViewDto`, `FieldViewDto` + `convert` fns; commands `list_players(project_id)`, `player_view(project_id, player_id)`, `set_field(project_id, table, record, field, value)`. Register in `command_handler!` + `export_typescript`.

**Copy-from**
- Edit op shape (validate→write→audit, undoable): `EditTransaction::set_captaincy` (`txn.rs:228`) and the `set_field` private writer (`txn.rs:72`).
- Entry-time validation: `EditSession::set_field_checked` exists in `tdb-edit/src/lib.rs:139` (uses `FieldMeta::validate`+`encode`) — copy its validate/encode logic into the transactional path.
- DTO/convert/command triplet: §0.2 templates.

**Verification checklist**
- `crates/studio-domain/tests`: stage `set_field` on an integer rating (in range) → `validate` clean → bytes change; out-of-range → `EditError::Invalid`; `update="False"` field → rejected; a name edit via `write_inline_str` round-trips and saves CRC-valid; `undo` restores baseline.
- `studio-api` registry test: `list_players` count matches `model.players.len()`; `player_view` returns curated + all_fields; `set_field` then `validate` clean.
- `cargo test --workspace` green; bindings regenerated (new `*Dto.ts` present).

**Anti-pattern guards:** edits go through `with_repairing_transaction`; `write_inline_str` zero-pads full width; no hand-written DTO TS.

---

## Phase 3.3 — Frontend foundation (Query + Router + typed client + shell)

**What to implement**
1. `cd app && npm i @tanstack/react-query react-router-dom`.
2. `app/src/lib/client.ts`: typed invoke wrapper over `@tauri-apps/api/core` `invoke`, with a `CommandMap` keyed by command name → `{args, return}` using the generated `bindings/*` types. **Lock the arg casing decided in §0.3/3.4 here.**
3. `app/src/main.tsx`: wrap `<RouterProvider/>` in `<QueryClientProvider>` (one `QueryClient`, `gcTime`/`staleTime` set).
4. `app/src/routes.tsx`: `createBrowserRouter` with an `AppShell` layout route (`<Outlet/>` + left-nav: Dashboard/Teams/Arenas/Players/Lines/Pending Edits) and a `ProjectGate` that requires a loaded project.
5. Replace `app/src/App.tsx`: a **Project open** screen calling `load_project` (path + platform + role + meta_xml + defs_root) → store the returned `ProjectSummaryDto.project_id` in a small context/store; **delete the `open_database`/`DbSummary` usage**.
6. `AppShell` header shows `ProjectSummaryDto` (platform/role/dirty) and Save / Save-as buttons (wired in 3.6).

**Copy-from**
- Provider/router/hook snippets: §0.3. Typed wrapper: §0.3 Part 3 (adapt imports to relative `../bindings/...` unless a `@/` alias is added to `tsconfig.json` + `vite.config.ts`).
- Existing invoke usage shape: current `app/src/App.tsx:14` (then remove it).

**Verification checklist**
- `cd app && npm run build` (tsc + vite) passes with no references to `DbSummary`/`open_database` (grep clean).
- App launches (`npm run tauri dev` or the project's run skill), Project-open calls `load_project` against an anchor and renders the summary; nav routes switch via the shell.

**Anti-pattern guards:** no `cacheTime`; bindings imported, not redeclared; single `QueryClient`.

---

## Phase 3.4 — Players page end-to-end ⟵ **THIN-SLICE GATE**

**What to implement**
1. `app/src/pages/Players/` (CSS Modules): master list from `useQuery(['players', projectId], list_players)`; detail at `/players/:id` via `useParams` + `useQuery(['player', projectId, id], player_view)`.
2. Curated detail card + a collapsible **"All fields"** table from `PlayerViewDto.all_fields` (progressive disclosure).
3. Inline edit: editable `FieldViewDto` rows render an input (enum → `<select>` from enum entries; int → number with min/max from range; string → text). On commit, `useMutation(set_field)` → `onSuccess` `invalidateQueries(['player', …])` + `['players']` + `['validation']` + project summary (dirty flag).
4. Confirm the **Tauri arg-casing** decision end-to-end and bake it into `client.ts`.

**Verification checklist (this is the gate)**
- Load anchor → open Players → select a player → see name/position/ratings.
- Edit an in-range rating → persists, dirty flips true, validation stays clean.
- Edit out-of-range → backend rejects, UI surfaces the error string, no state corruption.
- Edit a name (string write path) → persists and re-reads correctly.
- Save → Pending/summary reflects it; reload project → change present.
- `npm run build` + `cargo test --workspace` green.

**This proves the whole pattern (view-model → DTO → Query → page → edit → audit → save). Do not start breadth until this passes.**

---

## Phase 3.5 — Breadth: Teams + Arenas

**What to implement**
- Backend: `TeamView` (curated `kOtt` identity/colors/groups + arena via `buDa→StEO` + roster summary from `model.team_roster`) and `ArenaView` (`StEO`), each with `*Summary` list builders, DTOs, convert fns, and `list_teams`/`team_view`/`list_arenas`/`arena_view` commands. Reuse `entity_fields` for the all-fields panels. Edits reuse `set_field`.
- Frontend: `Teams` and `Arenas` master-detail pages cloned from the Players page shape; team detail links to its arena (`/arenas/:id`).

**Copy-from:** Phase 3.1/3.2 view-model + DTO/command triplet; Phase 3.4 page structure. Arena FK rule: §0.4 (`buDa`, not ClpB).

**Verification:** team list = `model.teams.len()`; team→arena resolves for the audited majority (note any unresolved as warnings, consistent with the Layer-4 referential predicate); edits validate clean; build + tests green.

---

## Phase 3.6 — Lines/depth chart, Dashboard, Pending Edits, Save-as/Pack

**What to implement**
- **Lines:** `LinesView` per team = `model.team_roster(team_id)` joined to each slot's player + the 71 situational flags **labelled from `FieldMeta::name`** (group by parsing the name prefix: `l#`, `pp#`, `pk#`, `ot_#`, `s#`, `g#`, `x#`). DTO + `lines_view(project_id, team_id)`. Render a depth-chart grid; situational flags shown **read-only with a "meta-derived (unverified)" badge** (toggling them is deferred until emulator-verified — keep within Phase 3 only if a quick anchor round-trip + in-game check passes; otherwise leave read-only).
- **Dashboard:** reuse `validate_project` → `IntegrityReportDto` (already exists) + `project_summary` counts; render findings grouped by severity with the file/record locators. (Inline repair = a button that runs the relevant existing op; wire only the ones backed by Phase 1 ops, e.g. demote-extra-captain via `set_captaincy`.)
- **Pending Edits:** session state view — `is_dirty`, `undo_depth`/`redo_depth`, undo/redo buttons (commands exist), and Save / **Save-as** / **Pack**. Add `save_project_as(project_id, out_path)` (already effectively `save_project` with a path) and confirm container **pack** uses the existing `Project::package` path (raw vs container). A staged-edit *log* (human descriptions) can come from accumulated `OpOutcome.repairs`; if a persistent log is wanted, add a `Vec<String>` history to `EditSession` (small, optional).
- Frontend pages for each, in the shell nav.

**Verification:** Dashboard shows the same clean/error counts as `cargo test` audits; undo/redo depth reflects edits; Save-as writes a new file byte-validated (`header_crc_ok`); container project (a ROSTER save) packs and re-extracts equal (reuse the Phase 2 `v4_container_save_spine` invariant).

---

## Phase 3.7 — Career-stats spike (bounded) + read-only display

**What to implement**
1. **Spike — DONE (verified 2026-06-28).** Against the NHL Legacy anchor: `stockcareerseasonplayerstats` (on-disk `ySst`, studio `tsSy`) **is present and populated**, joins by `game_id`(`zIBw`) == `ubPc.wBIz`, and decodes — e.g. player wBIz 6 has 9 season rows × 12 columns (year/teams/GP/G/A/P/+−/PIM/shots/PPG/SHG/GWG). It is **stock reference data → displayed read-only**. League-mode tables (`CtBM` season, `TKKI` career, `NFkQ` playoffs) exist for league saves but are not used for the base `nhlng.db` display. Builder: `studio_domain::view::career_stats` (resolves columns by metadata `name`, so it adapts across titles and skips absent columns).
2. Backend: `career_stats(project_id, player_id) -> Vec<CareerStatRowDto>` (read-only) joined by `wBIz`.
3. Frontend: a **read-only** career-stats table on the Player detail page.

**Out of scope:** editing career stats (stock table is reference data; bulk rebuild is Phase 8).

**Verification:** rows render for a player with known stats; values match a hand-decode of the anchor for one row (spot-check, documented).

---

## Phase 3.8 — Final verification (gate sign-off)

1. **Functional gate:** with an anchor loaded, all six views render and the Players/Teams/Arenas detail pages support validated inline edits that audit clean and survive save→reload. (Lines situational-toggle and career-stats editing explicitly excluded — documented.)
2. **Tests:** `cargo test --workspace` and `cargo clippy --workspace --all-targets` clean; `cd app && npm run build` clean.
3. **Anti-pattern grep:** no `open_database`/`DbSummary` in `app/`; no `cacheTime`; no `meta.field(` called with a studio id without `reverse_id`; no hand-edited `dto.ts`.
4. **Format-fact honesty:** every situational-slot label and career-stat table carries its provenance/confidence; nothing unverified is asserted as in-game-true (CLAUDE.md hard rule 3).
5. Update `docs/DESIGN.md`/`ROADMAP` notes if any view-model decision changed architecture; record the career-stats + Tauri-arg-casing findings here.

---

## Risks & deferrals (carried into the phase)

- **Tauri arg casing** — settled empirically in 3.4; everything downstream depends on it.
- **Career-stats authority** — bounded spike in 3.7; display-only.
- **Situational line flags** — labels are meta-derived; *editing* them stays deferred unless a quick emulator round-trip confirms semantics (don't block the phase on it).
- **Portraits** — display the id only; rendering is Phase 6.
- **Entity transactions** (trade/move/free-agent UI) — Phase 4; Phase 3 stays at field-level edits (the existing entity ops remain reachable but are not the focus).
- **Cross-title drift** — view-models are meta-driven and skip absent tables/fields, but only **proven titles (Legacy/14/15)** are in-scope to *verify*; do not claim others.

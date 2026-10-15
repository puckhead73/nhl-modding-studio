# State of the repository — handoff brief

> **Current progress, 2026-10-11:** Use the [feature roadmap's current handoff](plans/feature-work-package-roadmap.md#current-handoff--2026-10-11) for active priorities and acceptance. NHL07 Xbox360 MS-01–03 and DB-01–06 are accepted within their documented scopes; historical assets remain deferred to DB-08. Package progression is paused before DB-07 while the user reviews Studio and supplies direct observations. The September architectural snapshot below is historical, not the current task queue.

> **Purpose:** orient a planning agent (Codex) taking over structural planning and roadmap work.
> **Written:** 2026-09-05, from the working tree of branch `feat/phase5-asset-tooling`.
> Canonical companions: [DESIGN.md](DESIGN.md) (architecture), [ROADMAP.md](ROADMAP.md) (phases),
> [EDIT-MODEL.md](EDIT-MODEL.md) (binding editing contract), [audits/](audits/) (verified evidence).

---

## 1. What the product is

**NHL Modding Studio** is a release-quality desktop modding toolkit for EA's NHL games — the
spiritual successor to **NHLViewNG**. It edits EA **TDB** binary databases, handles console save
containers, imports players/teams/rosters from a built-in historical hockey corpus (1909→present),
and edits the games' graphical and audio assets.

- **Titles:** NHL 07 → NHL 21 — 17 catalog entries covering 07–21 plus Legacy and Legacy
  Recompiled, on the catalog's actual platform combinations (27 shipping targets). The original
  [ROADMAP.md](ROADMAP.md) still reads 10→21 and is retained as history only.
- **Platforms:** Xbox 360, PS3, PS4, NHL Legacy Recompilation.
- **Stack:** Rust engine crates plus `studio-domain` and `studio-api` under **Tauri v2**, a React and
  TanStack Query frontend, and SQLite for the historical corpus and sidecar ledgers.
- **Two audiences:** a simple common path for casual users, with depth behind progressive disclosure.

### Non-negotiable operating rules (from CLAUDE.md)

1. **Clean-room repo.** No reading or copying from other tool repos on this machine (the prior
   `nhl-database-studio`, `E:\Tools\rexglue-sdk`) unless the user explicitly opens that door for a
   specific instance.
2. **EA data policy.** Vanilla anchors ship in-repo as plain git binaries. Real user game data and
   stray saves stay out.
3. **Format facts verified, never guessed.** Every binary-layout claim is reproduced from anchors and
   lands in `docs/audits/` with provenance.
4. **Version- and platform-aware by default.** Nothing assumes a title's layout matches its neighbor.
5. **Human-readable surfaces.** User-facing text resolves metadata names and entity names. Raw TDB
   ids appear only behind a details affordance, and resolution happens in the domain or API layer.

---

## 2. Repository shape

| Area | Contents |
|---|---|
| `crates/` | 19 Rust crates: engine (`tdb-core`, `tdb-edit`, `tdb-defs`, `tdb-enums`, `tdb-relations`, `tdb-domain-validation`, `tdb-savedata`, `tdb-formula`), codecs (`tdb-rx2`, `tdb-archive-big`, `tdb-archive-codec`), historical (`tdb-fhm`, `tdb-fhm-build`, `tdb-fhm-store`), sidecars (`tdb-instance-registry`), QA (`tdb-qa`), mesh (`tdb-headmesh`, `blender-job`), and the two service layers (`studio-domain`, `studio-api`). |
| `app/` | React frontend: 27 pages, about 31 components, generated TypeScript DTO bindings in `app/src/bindings/`. |
| `src-tauri/` | Thin Tauri v2 shell, `capabilities/`, `tauri.conf.json`. |
| `anchors/` | Immutable ground truth: 552 files, 27 main-database anchors plus metadata, localization databases, and save fixtures. |
| `defs/` | Metadata, enum, and relationship definitions, plus `defs/appearance/*.toml` (head ids, UV classes, bone maps, donors). |
| `docs/` | Canonical design and roadmap docs, 25 audit documents, machine-readable audit CSVs, phase plans. |
| `corpus/` | Built FHM SQLite corpus, untracked, roughly 800 MB. |

Scale: about **82k lines of Rust** across crates and **24k lines of TypeScript** in the app.
`studio-api` exposes **115 Tauri commands**. `studio-domain` carries **33 integration test files**.

**Continuous integration** (`.github/workflows/ci.yml`) runs `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test --all`, and a frontend `npm ci && npm run build` job.

---

## 3. What is implemented

### 3.1 Engine and editing core (Phases 1–2, done)

- Byte-exact TDB v6 parse and serialize with CRC recomputation, Huffman type-66 pool rebuild, the
  namespace and id reversal rule, and both endiannesses (PS3 and Xbox big-endian, PS4 little-endian).
- An `EditSession` staged-edit model with field edits, row operations, undo and redo, and
  deterministic serialization. The engine never mutates parser truth; saves patch a byte snapshot.
- Three validation tiers: stage-time field validation, operation-time entity invariants through
  `with_repairing_transaction`, and a save-time whole-database domain audit.
- A save spine with atomic writes, backup on first write, an external-edit hash guard, and handle
  refresh.
- **Evidence-based capability probing.** `Capabilities::probe` inspects the loaded database's own
  descriptors, so unproven title and platform combinations stay honest by construction and the user
  interface hides surfaces the loaded game does not carry.
- Era-spanning codecs verified per anchor. The rating scale comes from the declared field range
  (6-bit plus 36 on gen7, 7-bit direct on PS4). Birthdate indexing drifts (0-indexed on 11–21,
  1-indexed on NHL 10). Main-team indexing drifts on `dXSB`. NHL 07 gen6 has its own schema shape.

### 3.2 Database surfaces (Phase 3, done)

Teams, Players, and Arenas master-detail with attributes, career stats, contracts, draft picks,
lines and depth chart, strategies, captaincy, jerseys, roster memberships, raw field disclosure, a
Pending Edits review surface, and a Dashboard carrying the validation audit with inline repair.

### 3.3 Roster operations and save containers (Phase 4, done)

Move, trade, add, clone, delete, free agent, hide and unhide, retire and unretire, position flip,
atomic roster-block realignment, capacity normalization to vanilla ceilings, and a Transactions
view. Save containers are detected by content rather than the version word, which lies. Versions 1
through 4 and the PS4 LZ4 container extract, edit, repack, and reopen with valid checksums.

### 3.4 Historical corpus and import (Phase 5, done)

The FHM corpus builder turns season-partitioned CSV exports into SQLite. Historical Players and
Teams browse on top of it. The Formulas page builds FHM-to-EA rating mappings with normalization,
archetypes, and saved profiles. The Roster Builder runs a two-phase preview and commit pipeline;
both lower through the same code path, which is enforced as a golden-test invariant. An
instance-registry sidecar dedupes imported player seasons.

**Legends team builder** is a scope addition beyond the original roadmap, now engine-complete with
an API and its own navigation section. Curated build strategies auto-populate a roster by ranking
historical players on computed overall, backed by a `legends_cache.db` sidecar.

### 3.5 Asset tooling (Phase 5 assets and Phase 6, partial — the active workstream)

Shipped and committed on the current branch:

- **Texture read and write** through `tdb-rx2`: the Xbox 360 BIGF to RefPack to XPR2 portrait chain,
  texlib RX2 DXT, PS4 GNF, BC4 and BC5, and full mip-chain regeneration on replace.
- **Portraits and logos** browse, decode, preview, export, and replace on extracted files.
- **Team Outfits and Uniforms:** a full-kit 3D preview on the shared 122-bone bind-pose rig, jersey
  and equipment textures, goalie gear swap-in, and a live nameplate editor with a verified jersey
  font atlas, draggable zones, and experimental curved baselines.
- **Hockey Shop:** per-slot equipment browse, 3D preview, a store library with sync, import, and
  delete, plus clone and install-to-game by equipment id.
- **Appearance Studio:** a head and hair census stage, a submesh classifier separating head core,
  hair, facial hair, and body, a multi-output catalog export producing a model and texture per head,
  a parallelized catalog build, and a browser with a textured 3D preview.
- **Headmesh porting groundwork:** the `tdb-headmesh` crate (extract, convert, remap, palette,
  transfer, morph, placement, pack, quality checks) and a `blender-job` harness crate. The
  NHL24-to-Legacy Path A round trip is verified on a real head against EA's own PS4 UV hash.

### 3.6 Verification apparatus

The `tdb-qa` crate regenerates every machine artifact deterministically from `anchors/`: an anchors
manifest with a hash for all 552 files, 27 per-anchor schema snapshots, a field-level cross-title
schema diff, and matrices for surfaces, scalar mutation, relational operations, and save containers.
Regeneration is byte-identical, so any diff means an anchor or a schema actually changed. Twenty-five
audit documents in `docs/audits/` carry the verified format findings.

---

## 4. What remains

### 4.1 Committed QA ledger status

From `docs/audits/data/game-qa-matrix.csv`, one row per game, platform, and feature family:

| Status | Rows |
|---|---|
| missing-fixture | 203 |
| edit-roundtrip-pass | 80 |
| discovered | 53 |
| read-pass | 27 |
| not-applicable | 19 |
| blocked | 10 |

The dominant gap is fixtures, not code. Every asset family reads `missing-fixture` because no asset
archives live under `anchors/`. The ten blocked rows are Xbox 360 STFS outer packaging.

**No cell anywhere is `game-verified`.** Automated round trips are proven, but in-game acceptance is
a separate manual gate that has not been run. This is the single largest outstanding risk to release
parity, and it gates write paths for every title.

### 4.2 Feature work not yet built

- **Texture Lab** is still a placeholder route. The codec layer beneath it exists.
- **Localization editing**, the type-66 pool-fit string table, is not built. Localization databases
  are discovered and listed only.
- **Audio (Phase 6.6)** is entirely unbuilt, including its bank-layout spike. That covers menu,
  in-game, play-by-play, and public-address preview and replace with bundled codecs.
- **Portrait and logo import (Phase 6.1):** the universal player slug and EA/FHM normalization are
  already implemented and used by Player Assets/headmesh export. The source-library index, portrait
  import/match review, on-import format converter, and logo icon generator remain open. Decode,
  replacement primitives, and export exist; per-target encode/install qualification is incomplete.
- **Appearance enum editing (Phase 6.3):** hair, facial hair, eye color, and skin tone, with offsets
  verified per anchor, plus the premodeled create-a-player mesh import library.
- **Headmesh porting (Phase 6.5)** beyond the one verified NHL24-to-Legacy round trip. The source and
  target matrix generalization and the install plus in-game gate per pair remain.
- **Blender harness (Phase 6.2):** the crate is scaffolded but uncommitted. Bundling decisions on
  size, licensing, and per-platform layout are open, and the golden round-trip gate is not closed.
- **Shell polish and onboarding (Phase 7):** the startup gate, onboarding wizard, archive extraction
  flow, and copy-on-switch build snapshots are partial at best.
- **Parity gap closure (Phase 8):** version 1 and Xbox STFS save write-back, companion-row auto
  creation on position flip, multi-select transactions, and clone name editing.
- **PS4 platform (Phase 9)** is largely de-risked ahead of plan. PS4 databases parse, build roster
  models, and round-trip, and the PS4 archive container has a native streaming extractor. What
  remains is the save-container write path, still marked experimental, and runtime topology.
- **Title-range expansion (Phase 10)** is far ahead of the roadmap text. Schemas are snapshotted and
  read-proven for all 27 anchors. The remaining work is per-title in-game verification, not
  reverse engineering.

### 4.3 Known defects and flagged assumptions

- The roster overall column applies the EA overall regression uncalibrated. That regression is an
  NHL 14 and 15 fit, exact there and roughly 13 points low on NHL 10 through 13. The Attributes tab
  gates it; the roster column does not.
- The dressed and scratched feature is wired to `ZBac.RREB`, which is the player type field rather
  than status. This is a real bug recorded in the audits.
- Goalie type codes on `hHbv` are a flagged best guess, not verified.
- Curved nameplate baselines ship marked experimental and preview-only.

---

## 5. Working-tree state (read this before planning)

Branch `feat/phase5-asset-tooling`, 139 commits, ahead of `main`. The tree is **not clean**:

- 56 tracked files modified, roughly 4,200 insertions and 800 deletions uncommitted.
- Substantial **untracked new work**: the `tdb-headmesh` and `blender-job` crates, the entire Legends
  builder across domain, store, two pages, and five DTOs, a player manifest module, eleven appearance
  definition files, three new audit documents covering headmesh UV compatibility, the Legacy donor
  census, and the NHL24-to-Legacy round trip, plus two new test files.

Any plan written against this repo should assume a large uncommitted asset and mesh workstream is in
flight. Three other branches exist: `main`, `feat/phase-5-stage-3-roster-builder`, and
`fix/move-player-block-coherence`.

---

## 6. Documentation drift — resolved by C0.3 (2026-09-07)

This section recorded the drift as it stood on 2026-09-05. Milestone **C0.3** reconciled the active
guidance; the record is kept so the corrections are traceable. See
[validation/c0-03-guidance-reconciliation.md](validation/c0-03-guidance-reconciliation.md).

| Drift recorded 2026-09-05 | Disposition |
|---|---|
| **README.md** said the project is a pre-release scaffold with no application code committed. | **Corrected.** README now states in-development status and points at the completion roadmap. |
| **ROADMAP.md** and **DESIGN.md** scoped NHL 10 through 21. | **Corrected in DESIGN.md** (NHL 07–21, 17 catalog entries, 27 shipping targets). **ROADMAP.md is retained as history** with a banner marking its scope wording stale. |
| The roadmap treated PS4 and title-range expansion as mostly unstarted net-new work. | **Corrected** in DESIGN.md and `01-scope-and-compatibility.md`: both are implemented with automated anchor coverage, and neither is game-verified. |
| The **compatibility matrix** showed every cell as partial, described as inherited reference-app confidence. | **Corrected.** The matrix now states an evidence level per cell (implemented / automated-tested / research-only / blocked / missing). Nothing was promoted: no cell is game-verified. |
| **Legends** was a shipped section with no roadmap slot. | **Noted** in the ROADMAP banner; carried as its own workstream in the completion roadmap section 2. |
| The **full-game QA plan** was the live sequencing document in practice. | **Superseded for sequencing** by the [project completion roadmap](plans/project-completion-roadmap.md) (milestones C0–C10). The QA plan remains the detailed verification contract; its phase numbers are historical. |

Still open: the generated-versus-manual QA evidence merge (C0.1 D9/D10) is owned by **C0.4**, and the
C0.2 baseline defects B1–B7 are unfixed.

---

## 7. Status update — 2026-09-13 (C0-NEXT1)

This brief was written on 2026-09-05 and sections 1–6 are left as written. The following corrects
what has changed since; the five levels — implemented, tested, accepted, committed, activated — are
kept apart deliberately.

| Subject | 2026-09-05 wording | State on 2026-09-13 |
|---|---|---|
| Canonical Project schema / target adapters / asset profiles | not mentioned (did not exist) | **Committed** at `ef0fdd7` (`crates/tdb-canonical`, `defs/canonical/`, `defs/adapters/`), `1ede5ba` (2026-09-14, C0-COMMIT-REG1) and `58d1bc0` (2026-09-14, C0-COMMIT-REG1b): **4 of 28 database adapters** (all partial) and 1 of 14 asset profiles populated. The fourth adapter, `nhl15-ps3` (C0-REG1b: 10/2969 fields, capacities read from its own committed PS3 ROSTER fixture, birth-date origin / companion partition / main-team encoding measured on the PS3 anchor, `team.name` round trip automated-tested, `player.shoots` / `player.team` blocked; the PS3 ↔ Xbox 360 NHL 15 main-DB byte identity is recorded as an observation that carries nothing), ships with the shared `target_evidence` test module. The third adapter, `nhllegacy-xbox360`, is partial (10/2965 fields; capacities read from the committed Xbox 360 Legacy ROSTER fixture, position discriminator / companion rule / main-team encoding measured on the anchor, `team.name` edit round trip automated-tested; handedness and the main-team parent table still blocked), anchor-snapshot cross-check tests with a target/snapshot identity guard and source-scoped capacity checks, sentinel `raw` units fixed in the contract (`literal-code` = stored bit pattern), and `tdb-core` + `tdb-savedata` as `tdb-canonical` dev-dependencies. The registry deliverable is not met. **Committed since (all 2026-09-15):** `2ec9fde` six further partial adapters (`nhllegacy-ps3`, `nhl14-xbox360`, `nhl14-ps3`, `nhl13-xbox360`, `nhl12-xbox360`, `nhl11-xbox360`; shared `target_batch_tests`; `target_evidence` generalised to RawV1 / RawV2 / ZlibV4 — [C0-REG2](receipts/c0-reg2-database-asset-registry.md)); `0251b57` two **represented, unpopulated** asset profiles (`assets-logos`, `assets-jerseys`) and the cross-game mapping inventory; `26d5e44` four PS3 adapters (`nhl10-ps3`, `nhl11-ps3`, `nhl12-ps3`, `nhl13-ps3` — NHL 10 measured 1-origin birth dates, an as-is main-team pointer with no distinguishable absence code, and a contested rating regime; [C0-REG3](receipts/c0-reg3-database-asset-design.md)); `6ba9af4` the NHL 10 Xbox 360 rating quarantine; `c3f4181` `asset-output-profile/4` (composite identities from named fields, ordered resolution chains with owned/shared steps and a resolved-path write exclusion; `/3` documents validate in place) with `assets-jerseys` moved to `/4` and its composite measured on the Legacy anchor ([C0-AS4](receipts/c0-as4-composite-resolution.md)). **Totals: 14 of 28 database adapters committed (12 measured on their own fixture, 2 structural; all partial, none game-verified) and 3 of 14 asset profiles committed — of which only `assets-portraits` has any established block; `assets-logos` / `assets-jerseys` are represented, not populated.** `cargo test -p tdb-canonical` 366 on the isolated `c3f4181` tree. **Committed at `c5e3a2b` (C0-REG4-CLOSE, 2026-09-15 — [receipt](receipts/c0-reg4-ps4-database-adapters.md), [R1](receipts/c0-reg4-r1-attribution-and-verification.md)):** seven partial PS4 adapters (`nhl15-ps4` … `nhl21-ps4`; gen8, little-endian `nhl_game.db`), each measured on its own anchor: capacities from the main anchor's own directory (`main-db-max-records` — the PS4 main DB is not packed; the ROSTER save's directory is a separate measurement, made through the committed PS4 codec on NHL 16–19, **blocked** on NHL 15 whose ROSTER `DATA` the codec does not decode, **missing-fixture** on NHL 20/21 which commit no ROSTER save), 7-bit 0..99 ratings with the identity transform (no offset), 0-origin birth dates, slot-team + 1 main-team pointer at 8/9/10-bit widths with pattern 0 = absent, `team.name` on `fullname` (NHL 15) / `teamname` (NHL 17–21) and **unresolved on NHL 16** (no such field), `player.shoots` / `player.team` blocked. Findings kept visible: the committed **NHL 19 and NHL 20 PS4 anchors are byte-identical** (main DB, metadata, auxiliary DBs; no capture provenance for either — [C0-REG4-R1](receipts/c0-reg4-r1-attribution-and-verification.md) makes both adapters **non-operative**: every measurement a candidate under an unknown block, every mapping blocked, nothing established; the NHL 20 Threes save disagrees with that anchor on every roster-core directory figure, cause not attributed); 44 un-rostered bio rows on NHL 18–21 store main-team patterns outside the team key set; the engine's team-name resolver reads `fullname` and so reads nothing on NHL 16–21 PS4. With these, **19 of 28 database units have an attributed partial adapter** (17 measured on their own anchor) **plus 2 registered as representation only**; 449 canonical cases on the exact staged tree, metadata XML pins as LF-normalised text digests. **Committed at `039de62` (C0-REG5-CLOSE, 2026-09-15 — [receipt](receipts/c0-reg5-early-generation-adapters.md)):** the six early-generation units — `nhl07-xbox360` (gen6: no equipment companions, names on `stockteamtable` through the thin `kOtt` index, gen6 rating offset 50), `nhl08-*` / `nhl09-*` (as-is main-team pointer, 1-origin birth dates, rating regime named by no audit row → candidate; NHL 09 ROSTER fixtures carry the ceiling, NHL 07/08 fixtures decode to a TDB without the roster core → capacities unknown; 4 companion-less rows on NHL 09), and `nhl07-ps3` as representation only (research copy, bytes = Xbox 360 anchor's). **27 of 28 database units registered — 24 attributed partial adapters, 3 representation-only (`nhl19-ps4`, `nhl20-ps4`, `nhl07-ps3`); Legacy Recompiled absent (FR4)**, 519 canonical cases on the exact staged tree. **Committed at `cc913f9` (C0-AS5 + C0-AS5-CLOSE — [receipt](receipts/c0-as5-shared-legacy-asset-profiles.md)):** `asset-output-profile/5` adds `reuses` (shared definitions by reference; boundary fixed in `asset_reuse.rs`; `/3` and `/4` documents refuse it); the three committed profiles move to `/5` and register `legacyrecomp-recomp` by reference to `nhllegacy-xbox360` per Q47 — inherited blocks keep their source states (nothing operative), destination blocks unknown, no hash invented; validation is sequenced so cross-field checks (an archive install's container) run on the effective profile while destination evidence stays the reuser's own; 532 canonical cases on the exact staged tree. **Committed at `9e02acf` (C0-AS6 + C0-AS6-CLOSE, 2026-09-16 — [receipt](receipts/c0-as6-asset-family-coverage.md); review passed after two in-place corrections):** the family inventory reconciled — of the fourteen QA-matrix families **seven are asset-bearing** and seven are database/save/onboarding workflows that carry no asset profile (`FAMILY_CATEGORY`, refusal rule `asset-family-not-asset-bearing`, `asset_family_coverage`), so the registry reads **1 populated (`assets-portraits`), 3 represented (`assets-logos`, `assets-jerseys`, `assets-equipment`), 3 absent (heads — external workstream; arenas — stretch; audio — C7) of 7**, never "11 of 14 missing"; `assets-equipment.toml` represents NHL Legacy Xbox 360 and PS3 (eight slot id fields **measured** on the byte-identical Legacy anchors, filename grammar from HEAD source, the PS3 operator stick record carried as asserted candidates — the only in-game asset-binding observation in the repository) and Recompiled by reference; the portrait profile gains a **measured identity candidate** on all three declared targets (`ubPc.lOnr`: 5 450 distinct ids on Legacy, 350 of them above every observed bucket range; 7 244 on NHL 21 PS4), still unknown because no id was placed beside a file; `docs/receipts/data/c0-as6-asset-db-references.csv` records the 19 asset-referenced database fields on all 27 anchors, drift-guarded (513 rows: `xjgw` absent before NHL 11, `kOtt.lOnr` absent on gen6, equipment widths 4–5 (8 for sticks) → 7–9 → 10 bits by title, the `kOtt` style fields single-valued 0 wherever present). 539 canonical cases on the isolated candidate; nothing became operative. **Committed at `8d60a6d` (C0-AS7 + C0-AS7-CLOSE, 2026-09-16 — [receipt](receipts/c0-as7-asset-evidence-measurements.md); Codex review passed after P2/P3 closed in place):** the **file half** measured on the two already-identified copies through committed readers: the member tables of Legacy Xbox 360 `cacherender.big` (12 845 members) and `nocacherender.big` (3 237) — both `rendering/`-only, so no portrait/crest/thumbnail member is in either — joined to the anchor (`docs/receipts/data/c0-as7-x360-member-join.csv`, re-derived by a case): all 698 jersey composite ids name a bundle, nameplate and sock member; every stored id of all eight measured equipment fields names a member under the HEAD grammar (the 74 stick texlibs = the 74 enum ids), while a ninth comparison — the goalie-stick folder against the `xmja.wXFx` field HEAD source says it shares — is **contradicted** (members 6..43 vs ids 35..112); 180 of 218 logo art ids name the declared `texlib_{id}_0_0` and all 218 some `texlib_{id}_{a}_{b}` (348 distinct, 130 unstored). A per-id census of the PS3 portrait folders (18 663 files hashed) joined to the anchor: **4 500 of 5 449** non-zero ids have a loose `playerheads` file, 949 do not (350 in 12403..12814, 599 inside 1..12401); the PS3 `patch.big` read (459 members: 83 portrait `BIGF`s at the loose tree's paths, 23 colliding with differing content; 328 `.rpsgl`, of whose 329 `chunkzip` members the committed codec fails on 257 multi-chunk ones) supplies 33 of the missing ids. **No block promoted; no readiness changed**: discovery candidates measured but one root cannot cover families split across two archives; identities stay candidates (not total, policy-bundled, D7). 552 canonical + 28 archive cases on the isolated candidate (review P2: loose-file absence is `NotFound` only, other read errors fail the run; P3: summaries reconciled to the records); records 6.4 MB, retained per Codex. NHL 07's team-name join remains a canonical execution requirement (the contract has no join-path block; the field round trip does not implement it). Registration is not support: none is game-verified. |
| QA evidence system (`crates/tdb-qa/src/evidence/**`, `qa-evidence` CLI, `defs/qa/scopes.toml`) | §3.6 describes only the legacy `qa-inventory` generator | **Implemented and tested** (253 passed · 0 failed · 2 ignored on the isolated staged candidate, `--locked --offline`; the ignored cases are the file-symlink privilege case and the uncommitted headmesh-registry case). **Evidence core committed at `819940a`** ([receipts/c0-commit-evid1.md](receipts/c0-commit-evid1.md)); **emitter integration and fixture freshness committed at `52a12fd`** ([receipts/c0-close-emit-development-transition.md](receipts/c0-close-emit-development-transition.md), 2026-09-15): the four `studio-domain` matrix suites capture executed observations with fixture identity under `QA_EVIDENCE_ROOT`, and discovery/build/check verify recorded fixture identity (`qa-evidence-provenance/2`). **Not activated, by direction parked until the RC QA transition (C9)**: no live migration, publication, consumer transition, vocabulary/variant freeze, storage/retention policy or CI gate — all remain open tasks at that gate |
| `tdb-headmesh`, `blender-job` | "scaffolded but uncommitted", active workstream | Still uncommitted and still referenced by the live root `Cargo.toml`. **Conversion work has moved to a separate repository** (roadmap C5 direction update); Studio keeps the code, fixtures and receipts and owns native integration + qualification later |
| NHL 19 PS4 head imports | — | User-reported in-game success for the Mogilny and CC5 Demitra fixtures on game build CUSA11126 / APP_VER 01.60 under shadPS4 (emulator executable version **not recorded**) (`docs/validation/2026-09-1*-n19-*`). Fixture-specific; **not** an evidence record; promotes no cell |
| Working-tree state (§5) | 56 modified, large untracked mesh workstream | 269 status entries, index empty; additionally untracked: `docs/receipts/`, the evidence slice, `defs/qa/`, `scripts/`, N19 validation records, `crates/tdb-rx2/src/mcd_write.rs`, example binaries. Nothing here is recoverable from HEAD |
| `cargo fmt`/`clippy` at HEAD | B1–B7 unfixed | Unchanged: repo-wide checks still fail **at HEAD**; the repairs remain working-tree only |

**No cell anywhere is game-verified.** Nothing in this update promotes support.

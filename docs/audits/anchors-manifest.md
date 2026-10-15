# Anchors Manifest

> Inventory of every file under `anchors/` with sha256, size, platform, game, and role.
> Machine-readable: [`data/anchors-manifest.csv`](data/anchors-manifest.csv) (**552 files**).
> Regenerated deterministically by `cargo run -p tdb-qa --bin qa-inventory` (Phase 0 — see
> [qa-ledger.md](qa-ledger.md)); do not hand-edit the CSV.
> Table-level capacity & editable-data breakdown:
> [anchor-capacity-and-editable-tables.md](anchor-capacity-and-editable-tables.md).

`anchors/` is the **single source of truth** for vanilla game databases and save files — the
clean-slate starting point to be packaged with the program for users who start a fresh project.

## Current population

All three platforms are populated (NHL 07–09 gen6/early-gen7 added since the prior pass).

| Platform | Games populated | Per game |
|---|---|---|
| **ps3** | nhl07*, nhl08, nhl09, nhl10, nhl11, nhl12, nhl13, nhl14, nhl15, nhllegacy | `databases/` + `savefiles/` |
| **ps4** | nhl15, nhl16, nhl17, nhl18, nhl19, nhl20, nhl21 | `databases/` + `savefiles/` |
| **xbox360** | nhl07, nhl08, nhl09, nhl10, nhl11, nhl12, nhl13, nhl14, nhl15, nhllegacy | `databases/` + `savefiles/` |

> \* PS3 NHL 07 (`NHL07NG.db`) is a **research copy**, not a supported catalog platform — see
> [qa-ledger.md](qa-ledger.md). NHL 07 is gen6 (`NHL07NG.db`); NHL 08–15/Legacy are gen7 (`nhlng.db`).

> **PS3 = Xbox360 databases are byte-identical** per game (same tables, record counts, and record
> lengths — verified for all of NHL 10–15 + Legacy). The two platforms diverge only in the *save-file*
> container, never in the DB. See the capacity audit for the full comparison.

## Databases per game

Presence matrix (✓ = shipped in that game's `databases/`). PS3 and Xbox360 carry the same set.

### PS3 / Xbox360 (`anchors/<ps3|xbox360>/<game>/databases/`)

| File | Games | Role |
|---|---|---|
| `nhlng.db` | all (10–15, Legacy) | main game/roster database (TDB v6, big-endian) — audited |
| `nhl_eng_us.db` | all | localization strings (single `LanguageStrings`/`vCJG` dictionary) |
| `draft_roster.db` | **nhl11+** | draft-class roster (10-table `exhibition*` projection) |
| `dynamic_ratings.db` | **nhl14+** | live rating overlay (`dynamicskaterai`/`dynamicgoalieai`, 0 rows in anchor) |
| `nhlng-meta.xml` | nhl11+ (Xbox360 **also nhl10**) | schema / FieldMeta backbone |
| `nhl_eng_us-meta.xml` | nhl11+ (Xbox360 **also nhl10**) | localization schema |

> PS3 NHL 10 ships **no meta XML**; its schema is recovered from the byte-identical Xbox360 NHL 10
> anchor (which does carry `nhlng-meta.xml`).

### PS4 (`anchors/ps4/<game>/databases/`)

| File | Games | Role |
|---|---|---|
| `nhl_game.db` | all (15–21) | main game/roster database (TDB v6, **little-endian** analogue of `nhlng.db`) |
| `initial_experience.db` | all | onboarding default full-game snapshot (35–43 tables) |
| `eng_us.db` | 15, 16, 17, 18, 20 | localization — **absent for nhl19 & nhl21** (meta present, DB not shipped) |
| `entry_draft.db` | **nhl18+** | entry-draft roster pool (`exhibition*` projection) |
| `expansion_draft.db` | **nhl18+** | expansion-draft roster pool (`exhibition*` projection) |
| `threes_campaign.db` | **nhl18+** | Threes mode roster projection |
| `proam.db` | **nhl20 only** | Pro-Am roster projection |
| `nhl_game-meta.xml` | all | schema backbone |
| `eng_us-meta.xml` | all | localization schema (present even where `eng_us.db` is not) |

> **Anchor completeness gaps:** PS4 `eng_us.db` is missing for **nhl19** and **nhl21**; `proam.db`
> exists only for **nhl20**.

## Save files per game (`anchors/<platform>/<game>/savefiles/`)

Each save is a container directory. Layout and naming differ by platform/generation:

| Platform | Per game | Container naming | Types observed |
|---|--:|---|---|
| **ps3** | 6 | `BLUS#####<slot> - <TYPE>` (`PARAM.SFO` + `ICON0.PNG` + `SYS-DATA`) | SETTINGS, BEAGM, ROSTER, BEAPRO, HOCKEYCARD, SEASON |
| **xbox360** | 5–6 | `<TYPE><n> <timestamp>` | ROSTER, SEASON, BEAPRO, DYNASTY, HOCKEYCARD, PROFILE (nhl10 has no PROFILE) |
| **ps4 (15–16)** | 3–5 | `<hash> - <TYPE>` | PROFILE, ROSTER, BEAGM, SEASON, BEAPRO |
| **ps4 (17–21)** | 5–6 | `<Type><n>` | Profile0, RosterCustom0, BeAGM0, BeAPro0, Season0, ThreeOnThree0 |

Per-save file inventory (every `PARAM.SFO` / icon / data blob) is in
[`data/savefile-inventory.csv`](data/savefile-inventory.csv); PS3 container details in
[../formats/ps3-save-container.md](../formats/ps3-save-container.md).

## Integrity

Every file is recorded with its sha256 in the manifest CSV, so the anchor set is integrity-checkable
and any change to a "vanilla" anchor is detectable. The largest single anchor is now
`ps4/nhl16/databases/nhl_game.db` (~18.5 MB) — larger than the ~6.5 MB PS3-era ceiling noted in
`CLAUDE.md`; still comfortably within git's own packing (no LFS). Real game data remains git-ignored
by policy elsewhere; the anchors are the intentional packaged baseline and live under `anchors/`.

## Provenance corrections (REG-1, 2026-09-18 — pending coordinator acceptance; no anchor bytes changed)

Established by hashing the installed titles' own files (record:
[league-parity-batch-2026-09-18](../validation/league-parity-batch-2026-09-18.md) §1–§2). The
anchors keep their paths and bytes; this table says what they actually are, and where the correctly
identified sources live (private fixtures under `NHL_STUDIO_FIXTURES`, never committed — layout in
`crates/tdb-loc/tests/private_dictionaries.rs` and `crates/studio-api/tests/league_private_sources.rs`).

| Anchor path | Committed identity | What the bytes are (witness) | Correct source for the labelled title |
|---|---|---|---|
| `anchors/ps4/nhl19/databases/{nhl_game.db, nhl_game-meta.xml}` | `265e4a68…` / `c67f4bfd…` | **NHL 19 base**: byte-identical to CUSA11126 (APP_VER 01.60) `databigs/inxp_00.big` members #2089 / #1179. The install also ships `patch_inxp_51..56.big`, each with its own `nhl_game.db` (patch 56: `61332e66…`) — which one the game loads is unverified | the anchor (base); patch-56 copy = fixture key `databases/nhl19-ps4-cusa11126/patch_inxp_56.big/nhl_game.db` |
| `anchors/ps4/nhl20/databases/{nhl_game.db, nhl_game-meta.xml, entry_draft.db, expansion_draft.db, initial_experience.db, threes_campaign.db}` | same blobs as nhl19 | **NHL 19 base under an NHL 20 path** (same witness) | none in the repository. NHL 20 (CUSA15439, APP_VER 01.00, no patch archives) ships `nhl_game.db` `1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b` (10 226 516 B, 31 `stockleagues` rows) and `nhl_game-meta.xml` `2fac0e73…` = fixture keys `databases/nhl20-ps4-cusa15439/inxp_00.big/…` |
| `anchors/ps4/nhl20/databases/eng_us.db` | `62260005…` (37 734 rows) | **NHL 19 base dictionary** (member #936 of the NHL 19 base archive) | NHL 20's own `eng_us.db` `0fc95ec84b71aaf23c2447783b277ba2df22b4ed762f901acf220af83f7739ad` (40 535 rows) = fixture key `databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db` |
| `anchors/ps4/nhl20/databases/proam.db` | `nhl20 only` | unattributed (not compared against any install member in this batch) | — |
| `anchors/ps4/nhl21/databases/` (no `eng_us.db`) | — | — | NHL 21's own dictionary `24693e2f…` (CUSA18182 `inxp_00.big` #1156; 55 766 rows) = fixture key `databases/nhl21-ps4-cusa18182/inxp_00.big/eng_us.db` (coordinator decision: not committed) |
| `anchors/ps3/nhl08/databases/nhl_eng_us.db` | `308021e7…` (267 068 B) | **the Xbox 360 file** (identical to `anchors/xbox360/nhl08/…`) | PS3 disc BLUS30062 `USRDIR/fe/loc/nhl_eng_us.db` `b9fb3361dc6db09f33fec4d0adababa39b04664cd2bfb6959580fd1b1b78580b` (265 236 B, 5 718 rows) = fixture key `databases/nhl08-ps3-blus30062/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db` |
| `anchors/ps3/nhl09/databases/nhl_eng_us.db` | `d9e0463b…` (513 564 B) | **the Xbox 360 file** | PS3 disc BLUS30168 `USRDIR/fe/loc/nhl_eng_us.db` `537319ced258cdb4892876085b3ed94a9d4d3e0fb2adb3f1cb94ef042e8a121f` (511 324 B, 9 434 rows) = fixture key `databases/nhl09-ps3-blus30168/PS3_GAME/USRDIR/fe/loc/nhl_eng_us.db` |

Every other main-database and dictionary anchor with an installed counterpart was re-hashed
against the install and matched (PS4 NHL 15/16/17/18/21; PS3 NHL 08–15 and Legacy discs; Xbox 360
Legacy loose tree and `cacheboot.big`). Xbox 360 NHL 07–15 have no installed counterpart
(anchor-only). Consumers that read the mislabelled paths and how each was handled:

| Consumer | Disposition |
|---|---|
| `defs/adapters/nhl19-ps4.toml`, `nhl20-ps4.toml` (registered representation only since C0-REG4-R1) | header notes record the witness / mislabel; blocks unchanged — promotion of NHL 19 is a separate canonical package |
| `crates/tdb-loc/tests/dictionaries.rs` (`"nhl20"` row) | annotated: NHL 19 base measurement under the anchor path; real sources in `private_dictionaries.rs` |
| `crates/studio-api/tests/leagues.rs`, `league_offline_campaign.rs` (`nhl19`/`nhl20` rows) | annotated; the campaign's NHL 19 row is base-attributed, its NHL 20 row is NHL 19 base bytes |
| `docs/plans/league-identity-parity-matrix.md` §0.2 | rows corrected in place (REG-1) |
| `docs/audits/data/schema/ps4-nhl20.json`, `docs/audits/roster-capacity.md`, `crates/tdb-qa` catalog, `crates/studio-domain/tests/{birthdate_encoding,ea_match_key,zz_diag}.rs`, `capability.csv` tree | read the path as a label only; **not edited** (generated evidence / C0.4-owned merge); they describe NHL 19 base bytes wherever they say "nhl20" |
| `crates/studio-domain/src/discovery.rs` (`nhl20` → `CUSA15439`) | correct: it maps the game id to the install, not to the anchor |

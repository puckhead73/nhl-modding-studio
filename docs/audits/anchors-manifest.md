# Anchors Manifest

> Inventory of every file under `anchors/` with sha256, size, platform, game, and role.
> Machine-readable: [`data/anchors-manifest.csv`](data/anchors-manifest.csv) (**497 files**, ~608 MB).
> Table-level capacity & editable-data breakdown:
> [anchor-capacity-and-editable-tables.md](anchor-capacity-and-editable-tables.md).

`anchors/` is the **single source of truth** for vanilla game databases and save files — the
clean-slate starting point to be packaged with the program for users who start a fresh project.

## Current population

All three platforms are now populated (PS4 and Xbox360 added since the first pass).

| Platform | Games populated | Per game |
|---|---|---|
| **ps3** | nhl10, nhl11, nhl12, nhl13, nhl14, nhl15, nhllegacy | `databases/` + `savefiles/` |
| **ps4** | nhl15, nhl16, nhl17, nhl18, nhl19, nhl20, nhl21 | `databases/` + `savefiles/` |
| **xbox360** | nhl10, nhl11, nhl12, nhl13, nhl14, nhl15, nhllegacy | `databases/` + `savefiles/` |

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

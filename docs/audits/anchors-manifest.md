# Anchors Manifest

> Inventory of every file under `anchors/` with sha256, size, platform, game, and role.
> Machine-readable: [`data/anchors-manifest.csv`](data/anchors-manifest.csv) (203 files).

`anchors/` is the **single source of truth** for vanilla game databases and save files — the
clean-slate starting point to be packaged with the program for users who start a fresh project.

## Current population

| Platform | Games populated | Per game |
|---|---|---|
| **ps3** | nhl10, nhl11, nhl12, nhl13, nhl14, nhl15, nhllegacy | `databases/` + `savefiles/` |
| ps4 | nhl17–nhl21 (dirs only) | **empty** — out of scope this pass |
| xbox360 | nhl10–nhl15, nhllegacy (dirs only) | **empty** — out of scope this pass |

## Databases per game (`anchors/ps3/<game>/databases/`)

| File | Present | Role |
|---|---|---|
| `nhlng.db` | all games | main game/roster database (TDB v6) — audited |
| `nhl_eng_us.db` | all games | localization strings (TDB v6) |
| `draft_roster.db` | nhl11+ | draft roster database |
| `dynamic_ratings.db` | nhl14+ | dynamic ratings database |
| `nhlng-meta.xml` | nhl11+ | schema / FieldMeta backbone (nhl10 has none) |
| `nhl_eng_us-meta.xml` | nhl11+ | localization schema |

## Save files per game (`anchors/ps3/<game>/savefiles/`)

Six PS3 saves per game — `BLUS#####<N> - <TYPE>` for TYPE ∈ {SETTINGS, BEAGM, ROSTER, BEAPRO,
HOCKEYCARD, SEASON}. Structure and container details in
[../formats/ps3-save-container.md](../formats/ps3-save-container.md); per-save inventory in
[`data/savefile-inventory.csv`](data/savefile-inventory.csv).

## Integrity

Each file is recorded with its sha256 in the manifest CSV so the anchor set can be integrity-checked
and so future changes to "vanilla" anchors are detectable. Note: real game data is git-ignored by
policy elsewhere, but the anchors are the intentional packaged baseline and live under `anchors/`.

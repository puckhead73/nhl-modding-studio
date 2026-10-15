# Handoff R68 — PS4 venue directories are selected by the vault level's `DebugName`: `exhibitionarena.arenaartid` → `ge_level.ID` → `DebugName` → `rendering/stadiums/<DebugName>/` on every title NHL 15–21 (32 / 33 … 79 / 79 levels; every venue directory named; the only unnamed directories are the shared `common*` / `unused` / `vancouvertest` folders) — closes Q-R64-1

> **Correction note — 2026-09-22 pm (coordinator review F3; research handoff [R69](../overnight-2026-09-22-pm/01-correction-checkpoint-f1-f3-r66-r68.md)).** Exact counts replace "one level per title ships no folder": NHL 15 `carolinageneric`, NHL 18–21 `team_canada_training`, **NHL 16 and NHL 17 none** (the §1 table was already exact; the summary and queue were not). Not stated below: level **ids are shared by several names** on NHL 16–21 (ID 0 by `anaheim` + `common_ice_int` on 18–21, by 27 names on NHL 17 and 5 on NHL 16; NHL 16 also ID 18 = `nybarclays` / `nyislanders`, ID 28 = `vancouver` / `custom_vancouver`), so the §2 chain reaches a *set* of names for those ids. **Restated:** the case-normalised name match supports navigation / listing with provenance and does not trace a runtime consumer; `DebugName` is not established as a runtime selector; Q-R64-1 is closed for directory naming only, the runtime-selection half is **Q-R69-1**; the §3 `arenaartid` re-pointing proposal is **withdrawn** pending a relationship / cascade and consumer contract; shared `common*` / `unused` / `vancouvertest` directories and the unresolved levels stay visible in any listing. The bounded listing contract with exact residuals is research R73 (2026-09-22 pm campaign).

Research lane, 22 September 2026 (prompt priority 3, Teams 2.5 PS4). Claim `research-r68-ps4-venue-dir`. Read-only archive member
listings (the r64 `scan`) joined to the 2026-09-19 r7 vault decode; nothing extracted, decoded, launched or written. Nothing
game-verified.

Machine-readable: [`data/r68-ps4-venue-dir-by-level-name.json`](data/r68-ps4-venue-dir-by-level-name.json) (per title: the level
names that equal a directory with their `ID` / `ArenaID` / collection key, the names without a directory, the directories without a
name). Probe `r68_ps4_venue_dir_by_level_name.py` (≈ 3 min, seven titles).

## Plain-language summary

Yesterday's inventory found that PlayStation 4 arenas live in folders named after their market and that nothing in the arena's database
row spells those names. The missing link is the vault record the 2026-09-19 work already tied to the arena row: that record's debug name
*is* the folder name, on every title. So the PS4 arena page can now find its venue folder in two hops — arena row → vault level (by the
arena art id, already established) → folder (by the level's name) — with no guessing. The handful of mismatches are exactly what they
should be: one level per title that ships no folder (a generic / training level) and a few shared folders (`common`, `unused`, a test
folder) that are not venues.

## 1. Join (exact, lower-cased; both directions)

| Title | levels | venue dirs | level name = dir | level names without a dir | dirs without a level name |
|---|---|---|---|---|---|
| NHL 15 | 33 | 33 | **32** | `carolinageneric` | `common` |
| NHL 16 | 41 | 44 | **41** | — | `common`, `common_boards_international`, `common_props_roof2` |
| NHL 17 | 79 | 83 | **79** | — | `common`, `common_boards_international`, `common_ice_edmonton`, `common_props_roof2` |
| NHL 18 | 65 | 68 | **64** | `team_canada_training` | `common`, `common_boards_international`, `common_props_roof2`, `unused` |
| NHL 19 | 70 | 74 | **69** | `team_canada_training` | + `vancouvertest` |
| NHL 20 | 74 | 78 | **73** | `team_canada_training` | + `vancouvertest` |
| NHL 21 | 76 | 80 | **75** | `team_canada_training` | + `vancouvertest` |

No level name is shared by two levels; no alternative name form was needed (0 hits on suffix / `custom_` variants — none were required).

## 2. The chain, end to end

`exhibitionarena.arenaartid` ↔ `ge_level.ID` (2026-09-19 handoff 07: 32 / 32 … 62 / 62 arena rows) → `ge_level.DebugName` →
`nhl/data/kettle/rendering/stadiums/<DebugName>/**` (this handoff). Two consequences the pm handoff 21 could not state:

- A venue directory is **not** keyed by a number and never by the DB row's `arenaname` / `cityname`; it is keyed by a vault text attribute.
  `DebugName` is a debug label by name, so the binding is *observed on every shipped title*, not documented; label it "vault level name".
- The `common*` directories are shared content referenced by many levels (their members are the `ice_*` / board / roof families of
  handoff 21) and are not venues; `unused` and `vancouvertest` are shipped folders no level names.

## 3. Consequence for the developer (Arena page, PS4)

- Resolve the venue folder through the vault: `arenaartid → ge_level (ID) → DebugName → stadiums/<DebugName>/`; show the folder and its
  member count; refuse with "level ships no venue folder" for the one level per title without one.
- Listing the venue's textures = the GNF members under that folder (the existing GNF route); nothing new to decode.
- Re-pointing an arena at another venue would be a DB `arenaartid` edit validated against the level set — a DB cell edit, **not** a
  vault write; no vault writer exists or is proposed.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-22/probes/r68_ps4_venue_dir_by_level_name.py docs/research/overnight-2026-09-22/data/r68-ps4-venue-dir-by-level-name.json   (≈ 3 min)
```
Executed: one smoke run (NHL 21 / 15, scratchpad, removed at closeout), one full run. Inputs: `<CUSA>/databigs/*.big` member listings
(read only) and `docs/research/overnight-2026-09-19/data/r7-ps4-vault-collections.json`.

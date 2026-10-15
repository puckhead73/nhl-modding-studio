# Handoff R73 — PS4 venue **listing** contract with exact residuals: 80 arena rows per title resolve to a vault level on every title, 79 / 80 unambiguously on NHL 15 and 17–21, 77 / 80 on NHL 16; the ambiguity is real and named (Honda Center's art id 0 is shared with non-venue level records on 16–21; Barclays Center and Rogers Arena share ids on NHL 16); a level record also binds ice texpacks, a rink id and typed placement references, so re-pointing `arenaartid` is a cascade, not a cell edit — read-only discovery only

Research lane, 22 September 2026 pm (prompt item 3; Teams 2.5 PS4; coordinator review F3; supersedes the write proposal in the
morning handoff 04 §3). Claim `research-r73-ps4-venue-listing`. Joins existing decodes only (the 2026-09-19 r7 vault sets and arena-row
join, the 2026-09-22 r68 directory listing); no archive, DB or vault was opened; nothing game-verified.

Machine-readable: [`data/r73-ps4-venue-listing.json`](data/r73-ps4-venue-listing.json) — per title every arena row with its level(s),
each level's directory (or `null`), class, and the level's `ArenaID` / `RinkId` / `CenterLogo` / `CenterLine` / `IceLine`; residual lists;
dependency counts. Probe `r73_ps4_venue_listing.py` (seconds).

## Plain-language summary

The morning result said a PlayStation 4 arena's folder can be found in two hops. Checked row by row against the arena table, that is
true for 79 of the 80 arena rows on six titles and 77 of 80 on NHL 16 — but the exceptions are real arenas, not corner cases: Honda
Center's art id 0 is also carried by non-venue vault records (a shared ice record on every title from NHL 16 on, 25 lighting presets
on NHL 17), and on NHL 16 Barclays Center and Rogers Arena each share their id with a second level record. The vault holds no attribute
that settles which record the game uses, so the listing must show every candidate with its folder and say why. A level record also
names the arena's centre-logo, centre-line and ice-line texture packs, a rink id and typed placement references, so changing an arena's
art id would swap all of those at once — that is why the morning's "re-point by editing the DB cell" proposal is withdrawn: the listing
below is read-only, and any future write would need its own cascade and runtime contract.

## 1. Listing per arena row (exact; `data/r73-ps4-venue-listing.json`)

| Title | arena rows | unambiguous (one level, one folder) | ambiguous (several levels share the id) | level without folder | no level |
|---|---|---|---|---|---|
| NHL 15 | 80 | 79 | 0 | 1 — `Carolina Arena` (art id 5 → `carolinageneric`, no folder) | 0 |
| NHL 16 | 80 | 77 | 3 — `Honda Center` (id 0: 5 levels), `Barclays Center®` (id 18: `nybarclays`, `nyislanders`), `Rogers Arena` (id 28: `vancouver`, `custom_vancouver`) | 0 | 0 |
| NHL 17 | 80 | 79 | 1 — `Honda Center` (id 0: `anaheim` + `common_ice_int` + 25 `lighting_custom_*`) | 0 | 0 |
| NHL 18–21 | 80 each | 79 | 1 — `Honda Center ®` (id 0: `anaheim`, `common_ice_int`) | 0 | 0 |

Every listed level name has a shipped folder except `carolinageneric` (NHL 15) and `team_canada_training` (NHL 18–21, no arena row uses
it). Folders no level names: `common`, `common_boards_international`, `common_props_roof2` (16–21), `common_ice_edmonton` (17), `unused`
(18–21), `vancouvertest` (19–21) — shared or unreferenced content, to be listed as such.

**Disambiguation is not available from the vault.** For the shared ids the sibling records differ only in content: `anaheim` carries
`CenterLine` 13 and `CenterLogo` 0 where `common_ice_int` and the lighting presets carry `CenterLogo` 9 / `CenterLine` 0; `custom_vancouver`
carries `CenterLogo` 1001 (a value outside the centre-logo texpack set on NHL 17). Those are observations about the records, not a
selection rule; a listing must present all candidates.

## 2. Dependencies a level record carries (the cascade)

| Attribute | NHL 15 | 16 | 17 | 18 | 19 | 20 | 21 |
|---|---|---|---|---|---|---|---|
| `CenterLogo` value inside the centre-logo texpack set | 33 / 33 | 41 / 41 | 78 / 79 (miss: 1001) | 65 / 65 | 70 / 70 | 73 / 74 (miss: 100) | 74 / 76 (miss: 100) |
| `CenterLine` inside the centre-line texpack set | 33 / 33 | 41 / 41 | 79 / 79 | 65 / 65 | 70 / 70 | 74 / 74 | 76 / 76 |
| `IceLine` inside the ice-line texpack set | 33 / 33 | 41 / 41 | 79 / 79 | 65 / 65 | 70 / 70 | 74 / 74 | 76 / 76 |
| `RinkId` | absent | absent | absent | absent | 0 × 65, 2 × 5 | 0 × 66, 2 × 8 | 0 × 67, 2 × 9 |
| `ArenaID` | absent | absent | present (`ID ≠ ArenaID` on 7) | present (8) | present (8) | present (8) | present (8) |
| typed references resolved | `ge_crowdplacement` 33 | 40 | 55 | 65 + `ge_nhl_3v3` 1 | 70 + 1 | 74 + 1 | 76 + `ge_nisoverridesgroup` 34 + 1 |

`ID ≠ ArenaID` cases are the non-NHL venues (`world_cup_toronto` 800 / 27, `team_canada_training` 36 / 27, the `euroswiss*` 326–331 / 0,
`common_ice_nhl` 552 / 0): `ArenaID` is not a second key to the arena table and must not be used as one.

So an arena's art id selects, through one vault record: the venue folder (by name), the centre-logo / centre-line / ice-line texture
packs (by numeric key into three other vault sets), a rink id (19–21), crowd placement and, on NHL 21, NIS override groups. A write
contract for `arenaartid` would have to state all of that, plus what the runtime reads — **not proposed**; Q-R69-1 stays open.

## 3. Listing contract input (read-only; not Codex-accepted)

- Key label: **"vault level name (observed on every shipped title; not a documented selector)"**.
- For an arena row: show every level whose `ID` equals `arenaartid`, each with its folder (`rendering/stadiums/<name>/`, exact
  lower-cased match) or "no folder shipped"; class the row as in §1 and say why when ambiguous; never pick one candidate silently.
- Show the level's texpack keys (`CenterLogo` / `CenterLine` / `IceLine`) resolved into their sets, with the two known misses (1001,
  100) reported as "no texpack with this key"; show `RinkId` where present; show typed references by class name.
- Shared folders (`common*`, `unused`, `vancouvertest`) appear once, under "shared / unreferenced content", never under an arena.
- No re-pointing, no edit affordance on `arenaartid` from this surface.
- Card **T-VEN-1** (listing binding) stays as revised by R69; a runtime-selection card is not requested.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r73_ps4_venue_listing.py docs/research/overnight-2026-09-22-pm/data/r73-ps4-venue-listing.json
```
Inputs: `docs/research/overnight-2026-09-19/data/r7-ps4-vault-collections.json` (levels, texpack sets, arena-row join) and
`docs/research/overnight-2026-09-22/data/r68-ps4-venue-dir-by-level-name.json`. Executed once. Not executed: archive or DB reads,
launches, writes.

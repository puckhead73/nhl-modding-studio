# Handoff R64 — PS4 arena / ice assets (Teams 2.5, parity attempt with the gen7 handoff 19): the venues are **named directories** (`rendering/stadiums/<market>/…`), not art-id-named files, and the arena row carries no name that reaches them — 22 of 68–83 directories per title match a `cityname`, 46–61 match nothing; so on PS4 the venue binding runs through the Attrib/VLT vault, not through a file-name rule

Research lane, 22 September 2026, roadmap step 2.5 PS4. Claim `research-r64-ps4-arena-asset-join`. Read-only archive member listings on
`H:` plus the PS4 anchors' `nhl_game.db`; nothing extracted, nothing decoded, nothing game-verified.

Machine-readable: [`data/r64-ps4-arena-asset-join.json`](data/r64-ps4-arena-asset-join.json) (per title: venue directories with member
counts, the `common/` ice families and their id spaces, the name-join results for `arenaname` / `cityname` with unmatched samples, the
numeric candidates, and the directories no arena row names). Probe `r64_ps4_arena_asset_join.py` (a few minutes for all seven).

## Plain-language summary

On the older consoles an arena's art id is literally its file name (handoff 19). PlayStation 4 works differently: each venue is a folder
named after its market — `detroit`, `boston`, `losangeles`, `vegas`, plus `custom_<market>`, `outdoor_canal`, `outdoor_farm`, `threes_5`,
`eashl_tier3`, `euroswissdavos`, `ahlgeneric` and a shared `common` folder. The arena table's text columns do not reach those names: the
arena names are trademarked building names (`Honda Center®`, `Bell MTS Place™`), and the city column is `"Sunrise, FL"` where the folder is
`florida`, so only 22 folders per title can be matched from a row by name and 46 to 61 folders match nothing at all. There is therefore **no
file-name rule** for PS4 venues; the binding between an arena row and its shipped venue is the typed vault data already decoded in the
2026-09-19 Attrib / VLT work (arena ID → texture reference), and a Studio arena page on PS4 must go through that, not through a name or an
id-in-the-filename convention. What *is* directly usable is the shared ice set in `stadiums/common/`: centre-ice logos, centre-line art,
hotspots and power rings, all keyed by number.

## 1. Venue directories and the shared ice families

| Title | venue dirs (members) | dirs matched from a `cityname` | dirs no row names | `ice_centerlogo` ids | `ice_cline` ids | `powerring` ids | `ice_hotspot` ids |
|---|---|---|---|---|---|---|---|
| NHL 15 | 33 | 22 | 12 | — (not shipped under `common/`) | 33 (0–33) | 31 | 2 |
| NHL 16 | 44 | 22 | 23 | — | 36 | 32 | 2 |
| NHL 17 | 83 | 22 | 61 | 246 (0–701) | 76 | 33 | 2 |
| NHL 18 | 68 | 22 | 46 | 276 (0–701) | 105 | 35 | 2 |
| NHL 19 | 74 | 22 | 52 | 287 (0–1 005) | 120 | 35 | 2 |
| NHL 20 (install) | 78 | 22 | 56 | 274 | 137 | 34 | 2 |
| NHL 21 | 80 | 22 | 58 | 283 (0–1 005) | 145 | 34 | 2 |

`arenaname` never matches a directory on any title (0 exact, 2–7 prefix-only). `cityname` matches exactly 30–133 **rows** but only ever the
same 22 directories — the NHL markets whose city word equals the folder name; everything else (`florida` vs "Sunrise, FL", `newyork*` vs
"Brooklyn, NY", all the EASHL / outdoor / Threes / European / AHL folders) does not.

## 2. Numeric candidates (explicitly not established)

`arenaartid` (32–62 distinct values) lands inside the `ice_cline` id set on 393–406 of the rows and inside `ice_centerlogo` on 152–160 —
suggestive, but the `ice_cline` space is a dense 0…N range that several columns satisfy, exactly as in the gen7 case (handoff 19 §3), so
this is a **candidate**, not a binding. `architectureid`, `scoreboardid` and `seatpatternid` carry 1–7 distinct values each and cannot key
anything of this size.

## 3. Consequence (developer)

- Arena page, PS4: resolve the venue through the vault (`gccollections` arena ID → texture reference, 2026-09-19), not by name or art id;
  say "venue selected by vault reference" rather than implying a file-name rule.
- The `stadiums/common/` ice families are numbered and directly listable / replaceable through the GNF route; their key is a candidate
  (`arenaartid`) and must be labelled as such.
- The non-NHL venues (EASHL tiers, outdoor, Threes, European, AHL generic) are shipped folders with **no arena row** naming them — list
  them as shipped venues, never as missing or unused.
- Gen7 and PS4 therefore need different arena-asset code paths; the gen7 rule of handoff 19 does not transfer.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r64_ps4_arena_asset_join.py docs/research/overnight-2026-09-21-pm/data/r64-ps4-arena-asset-join.json   (a few minutes)
```
Executed: one trial run on NHL 21 (scratchpad, deleted; it showed the `"City, ST"` normalisation was needed), then one final run over all
seven titles; one shell survey of the arena-related member names. Inputs: the install archives under `<CUSA>/databigs/*.big` (listing only)
and `anchors/ps4/<title>/databases/nhl_game.db` + `-meta.xml`.

# Handoff R63 — PS4 player portraits (Players 3.4, parity with the gen7 handoff 11): `artid` names the shipped portrait member on NHL 15–21, and unlike gen7 the `hasportrait` flag and the shipped files never contradict each other — **0 rows flagged without a file on all seven titles**; 1 703–5 999 portraits per title belong to no row

Research lane, 22 September 2026, roadmap step 3.4 PS4 (parity of the gen7 containment measurement). Claim
`research-r63-ps4-portrait-containment`. Read-only archive member listings on `H:` plus the PS4 anchors' `nhl_game.db`; nothing extracted,
nothing decoded, nothing game-verified.

Machine-readable: [`data/r63-ps4-player-portrait-containment.json`](data/r63-ps4-player-portrait-containment.json) (per title: portrait
members per archive, id range and duplicates, the format, coach portraits, and for `exhibitionplayerbiotable` / `customplayerbiotable` the
art-id column, its range, the rows with a file, the `hasportrait` census, both contradiction directions with samples, the unreferenced
portrait ids, and the `headid` / `hairid` value spaces). Probe `r63_ps4_player_asset_containment.py` (a few minutes for all seven — archive member listing only, nothing extracted).

## Plain-language summary

On PlayStation 4 a player's art id is the name of the portrait file, exactly as on the older consoles: art id 10 000 is
`playerportraits/p10000.gnf` (`.png` under `ui/assets/portraits` on NHL 15 — the format changed with NHL 16). Where the last-generation
titles contradicted themselves — players flagged as having a portrait with no file on disk — the PS4 games never do: on all seven titles
**every** row whose `hasportrait` flag is set has a portrait file. The opposite case exists but is small (3 to 451 rows per title carry a
file while the flag is clear). What every title does ship is a large surplus: between 1 703 and 5 999 portraits that no player row points
at — the pool a modder can draw from without adding files. So a Studio player page can show the portrait from `artid` on PS4 with no
special handling, and "no portrait" means what the flag says.

## 1. Containment per title (`exhibitionplayerbiotable`)

| Title | portrait ids (format) | coach portraits | rows | distinct `artid` (range) | rows with a portrait file | `hasportrait` = 1 | flagged, **no file** | file, flag clear | portraits no row references |
|---|---|---|---|---|---|---|---|---|---|
| NHL 15 | 8 377 (`.png`) | 0 | 5 555 | 5 376 (0…11 688) | 4 379 | 4 376 | **0** | 3 | 3 998 |
| NHL 16 | 9 246 (`.gnf`) | 28 | 5 632 | 5 450 (0…12 814) | 4 515 | 4 378 | **0** | 137 | 4 734 |
| NHL 17 | 10 282 | 28 | 6 128 | 6 125 (1…14 266) | 4 961 | 4 947 | **0** | 14 | 5 324 |
| NHL 18 | 11 442 | 29 | 7 236 | 7 095 (1…16 287) | 5 584 | 5 569 | **0** | 15 | 5 999 |
| NHL 19 | 7 865 (11 duplicated across archives) | 29 | 7 136 | 7 134 (1…18 400) | 6 164 | 6 139 | **0** | 25 | 1 703 |
| NHL 20 (install; the anchor DB is the NHL 19 base) | 9 149 | 235 | 7 136 | 7 134 | 6 590 | 6 139 | **0** | 451 | 2 561 |
| NHL 21 | 10 421 | 235 | 7 247 | 7 244 (1…21 813) | 6 481 | 6 469 | **0** | 12 | 3 943 |

The NHL 20 row pairs the **install's** archives with the NHL 19-base anchor database (the recorded anchor caveat), so its row counts repeat
NHL 19's; its portrait population is the install's own.

Also measured, for reference only: `customplayerbiotable` exists from NHL 17 (1 208–1 214 rows); `headid` spans 439–605 distinct values
with the 60 000+ pool ids present on every title, `hairid` 54–58 values from NHL 17 (0 on 284–424 rows). Their asset binding is the vault
work of the 2026-09-20 handoff 08 and is not repeated here.

## 2. Contrast with gen7 (handoff 11)

Gen7 had 93 / 127 contradictions between `hasportrait` and the shipped files, so a gen7 page must check the file. PS4 has **none** in that
direction on any title; the reverse direction (a file with the flag clear) is 0.05–7 % of rows and is a surplus, not a defect. A reader
should still prefer file existence over the flag for display, but the refusal "flagged but missing" does not arise on PS4.

## 3. Consequence (developer)

- Player page, PS4 15–21: resolve the portrait as `playerportraits/p<artid>.gnf` (NHL 15: `assets/portraits/p<artid>.png`) and show it;
  the archives holding them are named per title in the JSON (`game_60` / `inxp_10` on 21).
- A portrait *replacement* route on PS4 therefore needs no new identity work — it is the GNF texture route of handoffs 03 / 15 applied to
  that member (PNG on NHL 15 — a different codec path, flagged).
- The unreferenced portraits are a listed pool ("shipped, no player row"), not an error.
- Refusal: an `artid` with no member (1 000–1 600 rows per title, all with `hasportrait` clear) — report "no portrait shipped for this
  player", never "not consumed".

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r63_ps4_player_asset_containment.py docs/research/overnight-2026-09-21-pm/data/r63-ps4-player-portrait-containment.json   (a few minutes)
```
Executed: one trial run on NHL 21 / 15 (scratchpad, deleted — it revealed the NHL 15 PNG path), then one final run over all seven titles;
two shell listings of the portrait member names. Inputs: the install archives under `<CUSA>/databigs/*.big` (listing only) and
`anchors/ps4/<title>/databases/nhl_game.db` + `-meta.xml`.

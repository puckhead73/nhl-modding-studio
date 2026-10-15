# Handoff R51 — Arena venue parameters inventoried on all 27 anchors (Phase 2.5 "venue parameters beyond identity — not inventoried in Studio terms"): 159 distinct fields in seven era signatures, value ranges, constants, and candidate classes

> **Reclassification — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observation O-5).** The 12 `seat1zone1–6` / `seat2zone1–6` fields per PS4 signature (NHL 17–21) are **`colour`**, not `capacity`: their value space is 24-bit RGB (e.g. `0xdfddd7`, `0x878787`; max `0xfd9581`). The name-based rule mis-fired on "seat". `data/r51-arena-venue-field-inventory.json` keeps the original class; this note is the correction of record.

Research lane, 21 September 2026 pm, roadmap step 2.5 (Teams: arena / ice). Claim `research-r51-arena-venue-inventory`. Read-only over the
repository anchors (27 main DBs; the NHL 20 anchor is the NHL 19 base — its row is labelled `ps4/nhl20` here as the anchor is, and its
field set equals NHL 19's); nothing game-verified; no consumer trace — the classes are name-based candidates, not established semantics.

Machine-readable: [`data/r51-arena-venue-field-inventory.json`](data/r51-arena-venue-field-inventory.json) (per anchor: every field with
distinct count, min / max, dominant value and share, constant flag, text examples; `field_presence` = which anchors carry each field).
Probe `r51_arena_venue_field_inventory.py` (≈ 10 s).

## Plain-language summary

The arena table has 3 fields on NHL 07, 13–18 on NHL 09–16 / Legacy (both consoles), 33 on NHL 08 (a one-off set of rink-dressing
selectors), 124 on NHL 17 (a one-off set of "signature effect" and colour-zone fields), 60–67 on NHL 18–21 (six seat / wall / tunnel /
roof / railing / stair colour zones, scoreboard, goal light, netting colour, plus the horn-builder columns from 19 and `isoutdoor` /
`timeofday` on 21). Nine fields exist on every title from NHL 08 on: the arena's name, city, art id, type flag, three audio ids
(`arenaaudioid`, `cityaudioid`, `chantaudioid` to 17), the two horn ids and the attendance maximum. The 24-bit colour-zone fields on
NHL 17–21 are RGB triples (e.g. `0xDFDFD7`, `0xFFF200`, `0xFFFFFF`) — the custom-arena palette — and are 0 on 96–99 % of shipped rows
(only the few custom / created arenas carry them). Studio can now show every venue field with a class label and a "constant on this
title" mark, and refuse edits on the fields whose meaning is only a name guess; nothing here says which fields the game reads at runtime.

## 1. Era signatures (which anchors carry which fields)

| Signature | Fields | Titles |
|---|---|---|
| Universal | `index` | all 16 titles (NHL 07: `index`, `arenaid`, `table` only — the gen6 thin table) |
| Core (9) | `arenaname`, `cityname`, `arenaartid`, `arenatype` (0 / 1), `arenaaudioid`, `cityaudioid`, `goalhorn`, `periodhorn`, `maxattendance` (2 000–65 000) | NHL 08–Legacy, PS4 15–21 |
| Gen7 extras | `avgattendance` (44–100 %), `chantaudioid`, `puck` (0–703; 999 / 99 = none) — 08–17; `centerline` (0–76) — 11–17 / Legacy; `bouncemap`, `powerring` (0–999; 999 dominant = none), `centericelogo` — 13–17 / Legacy; `stanchion` (0 / 1) — 13–21 | as listed |
| NHL 08 rink-dressing set (20) | `icelogo`, `icelogoshown`, `icead1..3`, `boardad`, `boardadshown`, `boardtrim`, `doorlogo`, `jumbotron`, `roof`, `walls`, `step`, `redline`, `crease`, `seatlow / medium / high` (0–71), `crowdtype`, `icequality` | NHL 08 only (both consoles) — 6 of them constant 0 on the anchor |
| PS4 colour zones (43) | `seat1zone1..6`, `seat2zone1..6`, `wallzone1..6`, `tunnelzone1..6`, `roofzone1..3`, `railingzone1..3`, `stairzone1..3`, `scoreboardzone1..3`, `scoreboardid` (1–11), `seatpatternid` (0–4), `goallightid` (0–4), `nettingcolorid` (0 / 1), `architectureid` (constant 0), `announcer` (0 / 1) | NHL 17–21 (zones 4–6 of roof / scoreboard only on 17 / 18) |
| NHL 17 signature set (57) | `signaturegoaleffect{,2..5}{id,zone1..3}`, `signatureintroeffect{,2..5}{id,zone1..3}` (all constant 0), `boardtrimzone1..3`, `goalspotlight{id,zone1}`, `skaterspotlight{id,zone1}`, `goalpostcolor`, `centericelogostyle` / `centericelogotype`, `goalsongid` / `winsongid` / `powerplaysongid` (25-bit values, 0 on 99 %), `intromusicid`, `outropresentationid`, `powerplaycrowdreaction`, `propid`, `yearopened` (1968–2016) | NHL 17 only |
| Horn builder (11) | `goalhornelement1..3` (501 / 0 / 0 constant), `goalhornflair`, `goalhorniscustom`, `goalhornblastsnum` (1), `goalhornblast1..3length`, `goalhornpause1..2length` (all 0) | NHL 19–21 (value space = the catalogue rows 500–686 of handoff 08) |
| NHL 21 additions | `isoutdoor` (0 / 1), `timeofday` (constant 0), `yearopened` (1940–2020) | NHL 21 (`yearopened` also 17) |

## 2. Value observations (candidates, not consumer traces)

- Colour zones: values 0 or 24-bit RGB (`14671319 = 0xDFDFD7`, `16773632 = 0xFFF200`, `16777215 = 0xFFFFFF`, `8882055 = 0x878787`,
  `12467774 = 0xBE3F3E` …); non-zero on 1–4 % of rows (the created / custom arenas). A colour picker is the natural editor; whether the
  stock arenas ignore the field (0 = "use the art") is a runtime question.
- `arenaartid`: 0–910 with a dominant sentinel (255 on 08–12, 494 / 50 / 51 later) — the art-id space of the arena packs (2026-09-20
  handoff 11); `arenatype` 0 / 1 (stock / custom or indoor / outdoor — name guess; NHL 21 adds an explicit `isoutdoor`).
- `puck` / `powerring` / `bouncemap`: small id spaces with 99 / 999 as the dominant "none" value; `centerline` 0–76 — rink-marking /
  physics selectors by name only.
- `maxattendance` 2 000–21 273 on stock arenas (65 000 on Legacy = the outdoor stadium rows); `avgattendance` 44–100 (percent).
- Audio ids: `arenaaudioid` 0–1023 / `cityaudioid` 0–1999 / `chantaudioid` 0–511 (keyword ids on PS4 per 2026-09-20 handoff 07; the
  gen7 tag is Q-R35-1); sentinels 255 / 1023 / 1999 dominate on the older titles.
- The "nonzero values within another table's index range" flag in the JSON is a *range* test only (many small-valued fields trip it);
  it is not a reference proof.

## 3. Consequences (developer)

Arena detail page: list every field of the title's table with the class label of §1, show constants as read-only "constant on this
title (value v)", render the PS4 zone fields as colours, and refuse edits on fields whose class is `unknown` / name-guessed until a
consumer or a runtime card exists. This closes the inventory gap of the 2.5 matrix row; it does not establish any runtime effect.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r51_arena_venue_field_inventory.py docs/research/overnight-2026-09-21-pm/data/r51-arena-venue-field-inventory.json   (≈ 10 s)
```
Executed once (final). Inputs: the 27 anchor DBs + metadata under `anchors/`.

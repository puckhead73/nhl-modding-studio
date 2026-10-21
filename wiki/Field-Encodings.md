# Field Encodings

**Evidence:** mixed, labelled per row. The rating offsets at NHL 07 (+50) and NHL 14, 15 and Legacy (+36) rest on in-game observation by an earlier program; the intermediate titles are inferred from cross-title continuity. Birth-date bases, weight and height bases, position codes and nationality shifts are measured on the reference databases.

Many values are not stored the way the game shows them. A rating of 85 may sit in the file as 49; a January birthday may be month 0 or month 1 depending on the year; a weight of 200 lb may be 50 or 80. These rules are per title and sometimes per field, and getting one wrong corrupts data silently. The Studio keeps them in one encoding contract keyed by schema family; this page is the human-readable version.

## Ratings

Skater and goalie ratings are 6-bit fields on Xbox 360 and PlayStation 3 and 7-bit fields on PlayStation 4. The displayed value is the stored value plus an offset:

| Titles | Stored | Offset | Displayed | Label |
|---|---|---|---|---|
| NHL 07 (gen6) | 0 to 63 | +50 | 50 to 99 | measured in game |
| NHL 08, 09, 10 | 0 to 63 | +50 | 50 to 99 | inferred (same stored values for the same players as NHL 07) |
| NHL 11, 12, 13 | 0 to 63 | +36 | 36 to 99 | inferred (stored values rise by a median of 14 from NHL 10 to 11, and match NHL 14 after) |
| NHL 14, 15, Legacy | 0 to 63 | +36 | 36 to 99 | measured in game |
| PS4 NHL 15 to 21 | 0 to 99 | 0 | 0 to 99 | measured (the same players' stored values are 36 higher than on Xbox 360 NHL 15) |

Writing is the inverse: `stored = displayed − offset`, validated against the stored range. Two fields do not behave like ratings: goalie `consistency` stores only 0, 1 or 2 on every title whatever its declared range, and goalie `floppiness` (NHL 07 to 10 only) changes scale between titles. A tool should not apply the rating offset to either.

A calculated "overall" is not stored for skaters; EA's own formula was fitted on NHL 14 and 15 and reads roughly 13 points low on NHL 10 to 13.

## Birth dates

Month and day are 4-bit and 5-bit fields; year is 6, 7 or 8 bits with a base year.

| Titles | Month and day index base | Year base |
|---|---|---|
| NHL 07, 08, 09, 10 (X360 = PS3) | **1-based** (January is 1) | 1959 (NHL 07, 08), 1900 (09, 10) |
| NHL 11 to Legacy (X360 = PS3) | 0-based (January is 0) | 1900 |
| PS4 NHL 15 to 21 | 0-based | 1900 |

The declared range does not reveal the base: NHL 10 (1-based) and PS4 NHL 15 (0-based) both declare 0 to 15 and 0 to 31. PS4 NHL 16 to 21 declare 0 to 11 and 0 to 30. On every title except those, the declared range admits values that are not dates, so calendar validity has to be checked separately.

## Height and weight

- **Height:** stored in a 5-bit field as `inches − 54`. A stored 0 would display as 4 ft 6 in; about 180 unnamed placeholder rows on NHL 08 to 15 and PS4 NHL 15 carry 0 for both height and weight.
- **Weight:** stored in an 8-bit field as `pounds − base`, with base **150 on NHL 07 to 10** and **120 on NHL 11 onwards** including PS4. The same players' raw values rise by exactly 30 between NHL 10 and NHL 11.

## Position and handedness

- **Position** (3 bits): 0 centre, 1 left wing, 2 right wing, 3 defence, 4 goalie, on every title. Confirmed by the roster line slots, where goalie slots hold only code 4.
- **Handedness** (1 bit): two values, stable across titles.

## Nationality and birth state

The nationality field (`ubPc.Lelh`, 7 bits through NHL 19, 8 bits on NHL 20 and 21) holds a country code, and **the codes were renumbered between the older titles** as countries were inserted into the list. The national teams' own rosters show the shifts:

| Country | NHL 08, 09 | NHL 10 | NHL 11, 12 | NHL 13 to PS4 21 |
|---|---|---|---|---|
| Canada, USA, Finland, Sweden, Denmark, Norway, Belarus, Kazakhstan | 0, 14, 67, 69, 66, 68, 70, 72 | same | same | same |
| Austria | 77 | 78 | 78 | 78 |
| Russia, Ukraine, Latvia | 75, 76, 73 | 76, 77, 74 | same | same |
| Czech Republic | 79 | 81 | 82 | 82 |
| Great Britain | 80 | 82 | 83 | 83 |
| France | 81 | 83 | 84 | 84 |
| Germany | 82 | 84 | 85 | 85 |
| Italy | 84 | 86 | 87 | 87 |
| Poland | 86 | 89 | 91 | 91 |
| Slovakia | 88 | 93 | 95 | 95 |
| Switzerland | 89 | 95 | 97 | 97 |
| Japan | 96 | 105 | 108 | 109 |

From NHL 13 on, the codes are stable (the Legacy-era list). Code 127 means "no country" from NHL 10 on and is unused before that. Codes that no national team covers have no title-owned label; a careful tool shows them as numbers. The birth-state field (`ubPc.QvxI`) uses codes 1 to 13 for Canadian provinces, 15 to 65 for US states and the District of Columbia, and from 66 up the same country codes as nationality, with the same shifts.

## Other enumerations

| Field | Codes (NHL 11 to Legacy, PS4) |
|---|---|
| Captaincy (`eGlu.mCcl`, 2 bits) | 0 none, 1 captain, 2 alternate |
| Skater style (`eGlu.QgFs`, `dSvy.QgFs`, 4 bits) | 1 defensive D, 2 offensive D, 3 enforcer D, 4 two-way D, 5 grinder, 6 playmaker, 7 sniper, 8 power forward, 9 two-way forward, 10 enforcer forward; 0 unset. The skater code is 1-based; the goalie style code (`mHuy.QgFs`) is 0-based: 0 standup, 1 hybrid, 2 butterfly. Continuity suggests the skater codes were renumbered at NHL 10 and NHL 11 (open). |
| Player type (`ZBac.RREB`, 2 bits) | 0 built-in, 1 user-created, 2 special, 3 developer. This is **not** roster status; see [Roster Model](Roster-Model). |
| Dressed or scratched (`eGlu.hSZj`, 1 bit) | 1 dressed, 0 scratched |
| Skin colour (`ubPc.KTDQ`, 4 bits) | 1 white 1, 2 white 2, 3 tanned white, 4 pale white, 6 light brown, 9 dark brown, 10 black |
| Eye colour (`ubPc.Zufj`) | 0 blue, 1 light blue, 2 brown, 3 light brown, 4 hazel, 5 green, 6 yellow; per-title labels on PS4 are open (bulk re-populations observed between titles) |

## Negative values

A field whose metadata declares `rangelow < 0` is stored without a sign bit. The two measured sites, the career plus-minus fields (8-bit `[−64..127]` and 6-bit `[−32..31]`), store `raw = value − rangelow` (offset-binary), confirmed against real stat lines (a +10 season reads raw 74). The same rule explains the "+1, raw 0 means none" convention of `[−1..N]` reference fields such as the main-team pointer. Whether every negative-range field is offset-binary is **open**; the Studio applies the offset decode only to the measured sites.

## Reference pointers

| Field | Meaning | Encoding |
|---|---|---|
| `ubPc.dXSB` | main team | 0-indexed on NHL 09 and 10; 1-indexed with 0 meaning absent on NHL 11 onwards and PS4 |
| `ubPc.dbBW` | pro team | 1-indexed everywhere |
| `eGlu.toWX` | roster slot unique id | globally monotonic key with inter-block gaps; see [Roster Model](Roster-Model) |

Last reviewed: 2026-10-20

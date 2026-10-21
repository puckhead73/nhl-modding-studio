# Schema Drift Across Titles

**Evidence:** measured: a field-level diff across all 27 reference databases, re-generated deterministically from the files. Per-title schema pages generated from the same data are planned.

The core entity graph is the same from NHL 08 to NHL 21: teams, a roster table, a player index, player bios and per-player companion rows (see [Roster Model](Roster-Model)). Around that core, tables and fields are added, removed, widened and renumbered from year to year. This page lists the drift that matters when writing anything title-aware.

## Table sets

| Title group | Main-database tables |
|---|---|
| NHL 07 (gen6) | a distinct schema: no skater or goalie equipment companion tables, a thin two-level team table (stock and custom teams addressed through an index table, discriminator 0 stock / 1 custom), fewer bio fields |
| NHL 08 to Legacy (X360 = PS3) | the gen7 schema; 134 tables on the NHL 11 to Legacy range |
| PS4 NHL 15 to 21 | a newer schema: 111 tables on NHL 19 and 20, 115 on NHL 21, with new tables for arena ids, league `artid` and `renderid`, settings catalogues and more |

## Field-level drift worth knowing

| What | Drift |
|---|---|
| Roster table (`eGlu`) | 66 fields on NHL 11 and 12, 78 from NHL 13 on: twelve one-bit overtime line slots were added, taking the per-slot line flags from 59 to 71. |
| Birth month and day declared ranges | 4-bit 0 to 15 and 5-bit 0 to 31 everywhere except PS4 NHL 16 to 21, which declare 0 to 11 and 0 to 30. The stored index base changes elsewhere: see [Field Encodings](Field-Encodings). |
| Birth year | 6 bits on NHL 07, 7 on NHL 08, 8 from NHL 09; year base 1959 on the narrow fields, 1900 elsewhere. |
| Ratings | 6-bit 0 to 63 on every Xbox 360 and PS3 title, 7-bit 0 to 99 on PS4. |
| Nationality | 7 bits through NHL 19; 8 bits on NHL 20 and NHL 21 (values still at most 127). |
| Birth state | same widening on NHL 20 and 21. |
| Eye colour | absent on NHL 07; 3 bits on gen7; 5 bits on PS4 17 to 19; 4 bits on PS4 20 and 21. |
| Skater style | 3 bits on NHL 07 to 09, 4 bits from NHL 10; named `p_style` through NHL 13 and `playerstyle` from NHL 14. |
| Rating field names | `p_*` prefixes through NHL 13, `c_*` from NHL 14. |
| Equipment id widths | 4 to 5 bits (8 for sticks) on early titles, 7 to 9 bits on later gen7, 10 bits on PS4. |
| Arena foreign key on teams | present on NHL 10 to 12 only in the gen7 range; PS4 carries an arena id in its own table. |
| Draft fields | NHL 10 carries draft fields the neighbours lack. |
| Main team pointer (`ubPc.dXSB`) | 0-indexed on NHL 09 and 10, 1-indexed with 0 meaning absent on NHL 11 to Legacy and (at 8, 9 or 10 bits) on PS4; NHL 07 uses offset 0. |

## Between platforms of one title

- **Xbox 360 and PlayStation 3** main databases are byte-identical for every title from NHL 08 to NHL Legacy. Everything measured on one applies to the other. Dictionaries, archives and saves differ.
- **PS4 NHL 15** is not the Xbox 360 or PS3 NHL 15 database: different schema, different byte order, and the ratings regime changes (see [Field Encodings](Field-Encodings)).
- **NHL Legacy and the Legacy Recompilation** are one database.
- **NHL 19 and NHL 20 on PS4** share a roster generation; the roster-table capacities are identical. Earlier project material measured "NHL 20" on a file that was in fact the NHL 19 base database; the facts on this wiki come from the genuine NHL 20 release unless marked.

## Tables that are placeholders

The gen7 main database carries 33 to 43 tables with capacity zero. They are the mode tables (franchise, season, Be a Pro state) that only exist populated inside the corresponding save files. Mode saves hold a single TDB of these tables and no roster.

## Settings catalogue

Only the PS4 NHL 15 main database carries a mode-setup settings catalogue (a `stocksettings` table with per-context defaults). NHL 16 to 21 keep the same ids in `setting_csv/*.csv` files inside `inxp_00.big`; gen7 titles keep a table inside the executable; NHL 07 to 09 show none in the inspected files. Four fields of the NHL 15 catalogue are stored offset-binary rather than two's complement.

Last reviewed: 2026-10-20

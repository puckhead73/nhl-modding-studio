# Leagues, Competitions and Schedules

**Evidence:** measured on the main databases of every title and on the Xbox 360 league-code tables (PS3 by reference, since its main databases are byte-identical); 46 season and franchise save specimens; the NHL 15 PS4 settings tables and the NHL 16 to 21 CSV catalogues. One in-game observation exists (an NHL 21 league rename). Everything about which copy or rule the game applies at runtime is open.

## Where league identity lives

| Generation | League table | Team-side field | Display name |
|---|---|---|---|
| NHL 07 | none | `stockteamtable.league` (3 bits); the thin team table is only an identity map | not in the database; the three `CHTM_LEAGUE_*` keys are absent from NHL 07's dictionary, so the label source is open |
| NHL 08 to Legacy | none | `exhibitionteams.league` (3 bits on 08, 4 from 09) | a dictionary key bound to the code inside the game program (measured on Xbox 360 09 to Legacy; literal order only on 07 and 08) |
| PS4 NHL 15 to 21 | `stockleagues` | `exhibitionteams.league` → `stockleagues.leagueid` | `stockleagues.name` is a **dictionary key** (25-byte inline string), not display text |

PS4 `stockleagues` also carries `artid` (the league logo index, see [Logos and League Imagery](Logos-and-League-Imagery)), `renderid` (open), `orderid` (from NHL 17) and two colours (from NHL 18). Rows equal capacity on every PS4 title (20 on NHL 15 to 31 on NHL 20 and 21), so no spare league rows exist. NHL 21's 31 rows run from NHL, AHL, Elitserien, SM-liiga, DEL, Czech Extraliga, Swiss, EBEL, Allsvenskan, Champions League, International, OHL, QMJHL, WHL, Prospects, ECHL and Custom through EASHL, Mascot, 3-on-3, Spengler, NHL Alumni, All-Time, Pro-Am, CHL, USA, Russia, Rest of World and HUT; codes 16 and 24 to 30 have no team rows.

Dictionary text differs by title (`elitserien` reads "SHL" from NHL 14; `TXT_LEAGUE_EBEL` reads "Erste Bank Eishockey Liga" on NHL 18 and 20 but "ICE Hockey League" on NHL 21), so **never use a sibling title's dictionary**. Keys absent from the PS4 dictionaries (`AHL`, `CUSTOM_TEAMS`, `HUT`, mascot, 3-on-3, Pro-Am) presumably fall back to literal text (open).

**In-game observation (NHL 21 PS4):** re-pointing the Allsvenskan row's name key at the existing `TXT_ROW_LEAGUE` key and installing the database inside the archive showed "REST OF WORLD" in that league's slot across two cold boots; restoring the archive restored the text. One league, one placement, one emulator build, no screenshots.

## Xbox 360 league codes

| Title | Code → key |
|---|---|
| NHL 07 (inferred, literal order) | NHL, NATIONAL, Swedish, Finnish, German, Custom; team codes 0 NHL (32), 1 National (21), 2 Elitserien (12), 3 SM-liiga (13), 4 DEL (14), 5 Davos |
| NHL 08 (inferred) | 0 NHL, 1 AHL, 2 Elitserien, 3 SM-liiga, 4 National, 5 Custom |
| NHL 09 | 0 NHL, 1 AHL, 2 Elitserien, 3 SM-liiga, **4 `DEL`** (not `TXT_LEAGUE_DEL`), 5 Czech, 6 Superliga, 7 National, 8 Historic, 9 Custom |
| NHL 10 | 0 to 5 as above, 6 Swiss, 7 National, 8 Historic, 9 Prospects, 10 Custom |
| NHL 11 | 0 to 7 as NHL 10, 8 OHL, 9 QMJHL, 10 WHL, 11 Prospects, 12 Custom, 13 CHL |
| NHL 12 to 15 (one identical 20-key table) | 0 to 11 as NHL 11, 12 Legends, 13 Custom, 14 NHL (Winter Classic rows), 15 CHL, 16 United States, 17 Russia, 18 Rest of World, 19 HUT; codes 15 to 19 are unused by any stock team and their acceptance is open |
| Legacy | 0 to 6 as above, **7 Norway**, 8 National, 9 OHL, 10 QMJHL, 11 WHL, 12 Prospects, 13 Custom, 14 NHL, 15 EASHL, 16 CHL, 17 United States, 18 Russia, 19 Rest of World, 20 HUT, 21 Legends |

Gen7 labels are candidates from member-team abbreviations except where a PS4 `stockleagues` on an identical layout names them (NHL 12 to 15 match NHL 15 PS4; Legacy matches NHL 16 PS4).

## Conferences and divisions

NHL 08 to 11 keep a local 1-bit `conference` and 3-bit `division` per team; the binding of those numbers to names is open. From NHL 12 to Legacy each team carries three **global** 6-bit group ids (`leaguegroup`, `conferencegroup`, `divisiongroup`) indexing a group table of (dictionary key, league art id) pairs compiled into the game: 52 groups on NHL 12 and 13, 50 on 14 and 15, 49 on Legacy, where Norway's single group 19 shifts every later id. NHL teams are league group 0 with conferences 1 and 2; the AHL is group 9 on 12 and 13 and 7 from 14; single-group leagues repeat one id in all three fields; national teams point at a tournament block; All-Star rows put their conference id in the division slot. **Ids are title-specific and never transfer.** PS4 uses 7-bit ids with the NHL as group 1 (NHL 21: conferences 6 and 7, divisions 8 to 11; AHL 12 to 18); the PS4 group table is in the game program, with NHL 17 ids 43 to 60 and NHL 18 ids 74 to 76 inferred and NHL 19 to 21 ids 77 and 78 unknown. Division and conference names are dictionary keys; `WesternConference` and `EasternConference` are shared by the NHL, AHL, OHL and WHL, so renaming one renames all.

NHL 07 keeps league, conference and division on its 93-row `stockteamtable`: NHL rows 0 to 31 (All-Star rows 30 and 31), national 32 to 52, Elitserien 53 to 64, SM-liiga 65 to 77, DEL 78 to 91, Davos 92.

## Schedule tables

One table per league in the main database (`nhlschedule`, `ahlschedule`, the three CHL leagues, the European leagues, and later `echl`, `austrian`, `swedish2nd`, `champions`, `spengler`, `wc`, `worldchamps`, `worldjunior`; NHL 07 uses `intl<league>schedule`), bound to its league **by name stem only**.

| Family | Record | Fields (bit offset / width) |
|---|---|---|
| gen7 NHL, AHL, CHL; PS4 NHL 15 | 8 bytes | `round` 0/3, `status` 3/2, `day` 5/5, `month` 10/4, `index` 14/11, `home` 25/5, `away` 30/5 |
| gen7 European; PS4 15 European | 4 bytes | as above with `index` 14/9, `home` 23/4, `away` 27/4 (at most 16 teams) |
| NHL 07 | 8 bytes | 3-bit status, day 6/5, month 11/4, index 15/11, home 26/5, away 31/5 |
| PS4 16, 17 | 8 or 4 bytes | no `status`: `round` 0/3, `day` 3/5, `month` 8/4, `index` 12/11, `home` 23/5, `away` 28/5 |
| PS4 18 to 21 | 8 or 4 bytes | as 16/17 with 6-bit `home` and `away` (up to 64 teams); European tables keep 4 bits |

Month and day are 0-indexed (the Legacy opener month 9 day 6 is 7 October 2015); `index` runs 0 to N−1 in row order and equals position on all 144 shipped tables; `round` and `status` are 0 in every stock row and their meaning is open.

**The slot rule:** `home` and `away` are **the team's 0-based rank among its league's members sorted by the team table's global `index`**, not the index value and not record order. Changing a team's `index` or `league` silently relabels the whole schedule. An editor sorts members by `index`, maps slot to rank, and never writes `index` into a slot.

Row counts: `nhlschedule` 1 231 on NHL 08 to 17 (1 230 on NHL 10, which has no All-Star row), **1 271** on PS4 18 to 21 (31 × 82) beside `nhl32schedule` (1 312) and `ahl31/32schedule`; the AHL 1 116 to 1 200; the European leagues 300 to 450. Gen7 tables are packed, so more games need table growth, and whether the engine honours a shorter table is open; PS4 tables carry about 60 spare rows. On 30-team gen7 titles the All-Star squads are slots 30 and 31 with one game; PS4 18 to 21 carry 35 NHL rows, four of them zero-game All-Star squads, so validators must allow members with no games.

**NHL formats by era:** NHL 07 and 08 (6 divisions of 5): 8 × 4 rivals, 4 × 10, 1 × 10 of 15; NHL 09 to 13: 6 × 4, 4 × 10, 1 × 12 and 2 × 3; NHL 14 to 17 (four divisions of 7 or 8): 4 or 5 in division, 3 across, 2 × 15; NHL 18 to 21 (31 teams): the real alignment. The `nhl32schedule` uses an even 2/3 mix that is not the real NHL format. European shapes on the latest titles: SHL, Allsvenskan, DEL and Czech 14 clubs × 52 (every pair × 4); Swiss 12 × 50 in three groups of four; Finnish 15 × 60 in five groups of three; Austrian 11 clubs with a two-phase table whose second phase is open; the junior and ECHL tables are shipped irregular schedules with no rule established. Tournament tables (`wcschedule`, `championsschedule`, `spenglerschedule`) hold phase templates whose slot-to-team binding is open.

**Mode saves:** 44 of 46 day-one saves hold a regular-season copy equal to the source table under the mode's team numbering (three exceptions: a swapped All-Star row on NHL 13 to 15, NHL 08's All-Star row numbered after the AHL, one NHL 18 save matching the 31-team AHL table); franchise copies add 60 to 105 generated preseason rows; whether an **edited** source table reaches a new mode is open.

## The game settings catalogue

NHL 15 PS4 stores a mode-setup catalogue in five packed database tables (`stocksettings` 223 rows with string id, legacy id, default, size, setting identity, min and max; per-screen `stocksettingdefaults`; contexts; groups; dependencies). Four fields are stored **offset-binary** (`value = raw + rangelow`): `stocksettings.min` and `.id`, `stocksettingcontext.subgroupparent` and `.subgrouporder`, witnessed by `size == max − min + 1` holding on 213 of 223 rows under that reading and none under two's complement. Screens (names inferred from later titles) include Play Now (107 defaults), franchise creation, online GM, tournament, Be A Pro, Playoff and Season. Stored defaults include season tie-break 2 (five minutes then shootout), overtime loss 1 point, playoff series best of 7, salary cap off globally but on for franchise and online GM, franchise length "25". The 01.06 patch re-pointed `NumberOfTeams` to a four-choice list and added six tournament defaults, showing the catalogue is maintained as data without showing the game reads it.

NHL 16 to 21 keep the same ids in `nhl/data/common/gamemodes/setting_csv/*.csv` inside `inxp_00.big` (ASCII, CRLF, no quotes; large files chunkzip-wrapped with a 0x20000 window and zlib level 9 reproducing EA's bytes). NHL 16 stores the global default in one cell; NHL 17 in the `_DEFAULT_` context row; NHL 18 to 21 in **two equal cells** treated as one paired edit. Choice codes shift from NHL 17 (the tie-break list gains "OT3 then shootout" at code 2) and stored defaults are choice values, not list indexes. Xbox 360 NHL 10 to Legacy keep the definitions only inside the game program, and NHL 07 to 09 show none. Three questions are open everywhere: whether any screen shows a stored default, which default wins, and where the player's chosen value is stored.

Last reviewed: 2026-10-20

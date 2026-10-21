# Commentary and Speech Data

**Evidence:** measured by parsing the shipped speech tables of every gen7 title (PS3 equals Xbox 360 record for record) and the seven PS4 titles, with every reference resolved and counted. Which line the game chooses among the candidates at runtime is open.

Commentary is driven by data: events raised by the game, contexts that decide what to say when an event fires, sentences made of phrases, and sample groups that hold the recordings. NHL 09 and 10 ship these tables as EA's source XML; NHL 11 onwards ship compiled binaries of the same model.

## The gen7 speech model

- **Sentences** are named after their trigger event and carry a priority (1 to 4) and a list of **phrases**. Each phrase names exactly one fixed sample group plus event-reference tags whose value is an event parameter, and a repetition pool.
- **Graffiti tables** map `<tag>_<value>` to sample ids.
- **Event subscriptions** declare parameters, bit-flag enums and events with a 15-bit `interfaceCrc` that is stable per name and signature (the hash is not identified).
- Compiled forms: `SPC0` (v4.2 on NHL 11, 4.4 on NHL 12, 5.3 on NHL 13 to Legacy), `GRF0` (2.1, then 3.3 from NHL 13), `EVT0` parameter tables. A sentence's slot is FNV-1 of its name modulo the table size, verified on every record.

| Title | Phrases | Graffiti tables | Bank samples referenced |
|---|---|---|---|
| NHL 09 | 10 490 | 9 842 | 45 589 of 45 589 |
| NHL 10 | 13 331 | 11 133 | 53 366 references against 53 276 bank samples |
| NHL 11 | 14 060 | 12 624 | 61 572 of 61 576 |
| NHL 12 | 15 309 | 15 515 | 73 194 of 73 194 |
| NHL 13 | 14 696 | 17 392 | 83 453 of 83 453 |
| NHL 14 | 15 745 | 18 525 | 89 734 of 89 734 |
| NHL 15 | 9 829 | 8 368 | 45 442 of 45 442 |
| Legacy | 11 465 | 10 819 | 59 938 of 59 940 |

The keyword index (`KWD0`) is a separate mention index, not derivable from the graffiti tables.

## Name calls (gen7)

| Speaker | Database field | Tag | Coverage |
|---|---|---|---|
| play-by-play player names | `playernames.audioid` | `player_ID` | NHL 09 1 900 of 1 902; 14 3 862 of 3 875; 15 1 169 of 1 172; Legacy 1 681 of 1 683; 10 to 13 about 85% |
| rink-announcer player names | `exhibitionplayerbiotable.audioid` | announcer `playerid` | 70 to 92% of the roster per title |
| team names | `exhibitionteams.audioid` | `team_ID` | 207 of 211 keys on NHL 15 |

## Contexts: which line is spoken (`CTX0`)

NHL 13 to Legacy ship version 4.5 (`CTX0PS30` and `CTX0X360`): every record is a trigger plus conditions plus one or two items. Conditions point into a script blob table; an item is a **speech item** (speak this event) or a **context item** (raise another trigger), with one parameter per declared event parameter, either immediate or a call block. Counts: NHL 13 3 371 records, 14 3 583, 15 3 866, Legacy 4 183, with 0 refusals; the same parser reads the crowd contexts file (1 164 records). NHL 11 and 12 ship version 3.1 with inline names and one item per record (2 833 and 3 339 records); NHL 09 and 10 have no context file.

Worked example from NHL 15: context `Cup_End_Cup_Presentation_NHL` fires on `trigger_CUP_ENDINGS` under the conditions `cup_type_Stanley_Cup`, `+_Context_RUN_ONCE` and `+Group_Cup_Sequence_5_run_once`, and speaks `CUP_END_CUP_NHL`. Header words read as priority-like (100 to 800 for ordinary lines, 2 000 for cup presentations, 5 001 for injury stories, 9 999 at the top) and window-like values; the labels are candidates. The condition bytecode and runtime selection among triggered contexts are open.

## PS4 commentary (NHL 15 to 21)

- Contexts are a little-endian `CTX0` with 64-bit pointers (`0XTCTTEK`), version 3 on NHL 15 to 19 and 4 on 20 and 21; every trigger, item, condition and script index resolves on all seven titles (records: 3 864, 4 396, 5 178, 2 676, 2 998, 1 418, 2 153). A per-record 16-byte block table is undecoded.
- Events (`0TVETTEK`) use the player parameter `player_ID` on NHL 15 to 19 and `surname_ID` on 20 and 21; NHL 21 adds gender enums.
- NHL 21's sentence chain: 2 137 sentence slots lead to 4 953 phrases, each phrase's group key resolving to a DSET in one of the 16 play-by-play banks (4 953 of 4 953) and a history key to one of 27 history entries. For example, `INTRO_MATCHUP_LEAGUE_AHL` resolves to three sample ranges.
- The NHL 20 install ships a two-keyword stub keyword file, so keyword-driven name calls cannot work from it.
- **PS4 player-name binding has no established binding.** Five direct joins between database audio ids and the banks were excluded by density checks; partial, conditional or code-resident use is untested, and "not consumed" is not claimed.

## Display-name keys are not speech

The PS4 Creation Zone gear and mask names (`$bauer4500` and the like) resolve through the title's own dictionary by a case-insensitive key hash; exact spelling misses 19 to 64 cells per title, and the only unresolved cells are placeholder keys. This is a general dictionary-lookup rule, not commentary.

Last reviewed: 2026-10-20

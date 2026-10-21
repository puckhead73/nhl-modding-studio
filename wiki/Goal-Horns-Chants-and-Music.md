# Goal Horns, Chants and Music

**Evidence:** measured joins between database fields, loader tables and shipped bank files on every gen7 tree (16 Xbox 360 and PS3 trees) and the seven PS4 titles; selection rules that happen at runtime are labelled open. Nothing is game-verified.

## Gen7 goal and period horns (Xbox 360 and PS3, NHL 09 to Legacy)

The chain is a file binding, not a playback observation: the arena's `goalhorn` field (9 bits) names a loader asset in `audio/na_en_loadfile.xml`, which names `audio/sfx/{id:03}_goal.abk`; `periodhorn` follows the same chain with `_period.abk`. The loader table lists ids 0 to 29 and 100 (31 ids on NHL 09 to 13) plus 194 (32 ids from NHL 14), matching the shipped files exactly in both directions on all 16 trees. The 30 NHL arenas all match; ids 0, 100 and 194 are shipped but named by no row.

Residual ids appear in no loader table and no inspected source:

| Titles | `goalhorn` residuals |
|---|---|
| NHL 09 | 35, 40, 45, 61, 136, 146 |
| NHL 10, 11 | 30 to 39, 41, 45, 61, 136 |
| NHL 12, 13 | 30 to 38, 41, 45, 61, 136 |
| NHL 14, 15, Legacy | 30 to 36, 38, 41, 45, 61, 136 |

What plays for an unlisted id is a runtime question: silence, bank 0, or the segment-built horn handle (`h_goalHorn`, with eight sustain and gap parameters) are the candidates.

## PS4 horns

| Title | `goalhorn` id space |
|---|---|
| NHL 15 | identical to Xbox 360 NHL 15 per arena: 42 ids in 0 to 136 (NHL 0 to 29, DEL 30 to 41, SHL 45, Liiga 61, AHL and international 136) |
| NHL 18 | renumbered: NHL clubs 1 to 31 (including Vegas); 41 is the shared default on 253 of 303 arenas; `periodhorn` keeps the old numbering |
| NHL 21 | as NHL 18 plus custom-horn fields (`goalhorniscustom`, three 10-bit `goalhornelement` fields, `goalhornflair`, `goalhornblastsnum`, blast and pause lengths); `periodhorn` becomes a three-value enum (Generic, Siren, Buzzer) |

The `goalhorns.csv` catalogue (NHL 17 to 21: 90, 116, 278, 280, 280 rows) joins by rule: values 1 to 31 are premade club horns whose catalogue `audioid` is the value minus one (matching the arena's own club on 30 of 30 per title); generic values match `audioid` directly (41 is `AirHorn1` on 18 to 21, 31 on 17); 690 to 693 are sentinels with no row; the Rangers row is absent on every title. From NHL 19 the catalogue adds 63 horn elements (`audioid` 500 to 686 in steps of 3) and 97 flairs. NHL 15 and 16 ship no catalogue.

**Catalogue to sound is open.** No bank carries the team or generic ids as sample ids; the horn bank holds only the element and flair range plus unrelated ids. The consumer is a compiled sound patch (`goalhornpatch.ptc` in `sfx_ptc.big`) that names the teams and elements and is not decoded. The NHL 21 horn bank's sound-id space (56 plus a delta) is not the database id space.

## Gen7 chants

- **73 team-chant banks** (`<id:03>_chant.abk`, ids 0 to 79 with gaps at 30, 32, 33, 35, 47, 50).
- **45 goalie-chant banks** (`<id:04>_chant.abk`) keyed by the goalie's announcer audio id (`exhibitionplayerbiotable.audioid`): 44 of 45 match on NHL 10, every match a goalie; the set is an NHL 10-era list that ages (26 of 45 on Xbox 360 NHL 15).
- The arena's `chantaudioid` → `<id>_chant.abk` is a **partial candidate**: 23 to 25 values per title (30 to 33, 89, 90, 100 to 103, 105 to 117, 136) have no bank, and their tenants are DEL, national, Czech and create-a-team rows; 66 to 80 banks are named by no row.

**Club chants 055 to 066** are twelve banks carrying `<club>SamplePicker` names for Swedish clubs (Brynäs, Djurgården, Färjestad, Frölunda, HV71, Mora, Linköping, Luleå, Malmö, MoDo, Södertälje, Timrå; 3 to 5 records each), byte-frozen per platform from NHL 10 to Legacy. The names also appear in the crowd module and the control bank as empty message endpoints. Every SHL club's arena carries the generic chant id 45. The one database link: the Finnish arena carries 61 in `chantaudioid`, `goalhorn` and `periodhorn`, and 61 is the Linköping bank number, while no horn bank 61 exists. No consumer that selects a club bank has been traced; three candidates (a loader keyed by `chantaudioid`, a coincidental unshipped set number, a selector derived elsewhere) are not separable offline. Listing only; no edit route.

The arena `cityaudioid` and `arenaaudioid` fields have no established binding; their numerical overlaps with chant and horn ids are reported as small-integer coincidence.

## Music

**Gen7 music box.** `music_musicboxdata.bin` (`MBX0`) holds 36-byte records `sample id | category | kind | pointer → (volume type, context name)`, exactly one per music-bank id (NHL 11: 195, 14: 231, 15: 263, Legacy: 239). The chain is context → sound id → `SBKR` index → `.sbs` range → EALayer3 v1 (EA-XMA on Xbox 360 NHL 09 to 12). Contexts include `MenuTunes` (22 to 42 tracks by title), `IntroNationalAnthem` (21), `TeamIntroduction`, `HomeGoal` and `AwayGoal`, `EndOfPeriod` (16), `OffPlayGeneric` and `OffPlayOrgan`, penalty, win and lose, and on Legacy `WinterClassic` and `ShootoutIntro`. The `kind` values and category meaning are open; track titles are not located.

**PS4 music.** NHL 21 ships `musicbank.sbr` (423 records), `music_inxp.sbr` (6 210 records, 285 ids), an embargoed bank and a Nordic bank. Context-to-track selection lives in a compiled music module, so reassignment is not possible statically. The `songs.csv` catalogue (NHL 17 to 21: 191, 245, 637, 356, 422 rows) lists identifier, title key, composer, genre, art id, `audioid` and goal, power-play and win flags; its `audioid` is used as a sample id by no bank, and the database's six non-zero `songid` cells match neither catalogue column. It is a listing only.

Last reviewed: 2026-10-20

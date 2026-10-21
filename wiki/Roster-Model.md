# Roster Model

**Evidence:** measured on the NHL 11 to Legacy and PS4 reference databases: the bridge chain is declared in the metadata, the block layout and pool mechanism were decoded from every row of the Legacy database, and the capacity ceilings were read from every roster save and main database. The NHL 07 and NHL 08 variants are measured separately and noted.

A "player on a team" is not one row. It is a chain of rows across four tables, and the roster table is laid out in contiguous per-team blocks that the game reads as a unit. Most corruption left behind by older tools comes from treating a roster slot as a free-standing record.

## The bridge chain

```
kOtt  exhibitionteams            team; key vfEq (8-bit)
  ▲   eGlu.dXSB → kOtt.vfEq      the slot's team
eGlu  exhibitionrostertable      roster slot + lines; key toWX (14-bit)
  │   eGlu.XSWT → ZBac.vfEq      declared foreign key in the metadata
  ▼
ZBac  exhibitionplayers          player index; key vfEq (14-bit)
  │   ZBac.ykFq → ubPc.wBIz      the index row's bio
  ▼
ubPc  exhibitionplayerbiotable   player bio; key wBIz (14-bit)
```

A **player** is the bio row plus companion rows that share its key `wBIz`:

| Companion | Metadata name | Holds | For |
|---|---|---|---|
| `dSvy` | exhibitionskaterai | skater ratings and AI | skaters |
| `xmja` | exhibitionskaterequipment | skater equipment ids | skaters |
| `mHuy` | exhibitiongoalieai | goalie ratings | goalies |
| `fMVl` | exhibitiongoalieequipment | goalie equipment ids | goalies |

A usable skater needs a bio, a `dSvy` row and an `xmja` row; a goalie a bio, `mHuy` and `fMVl`. The companion pairs are allocated in lock-step (the skater pair always shares a row count, as does the goalie pair). NHL 07 has no equipment companions, and NHL 09 carries four bio rows with no companion and no index row at all.

## Roster slots and the block layout

The roster table is **partitioned into contiguous per-team blocks that tile the whole table** with no gaps and no overlaps: on Legacy, 222 teams with slots each own one record range, 20 to 35 slots long (most 23 to 26), beginning exactly where the previous team's ends. The earlier community assumption of a fixed 40-slot block is wrong in size but right in structure.

The stable identity of a slot is therefore *team T's k-th slot*. What varies is its **occupant** (`XSWT`, the player index it points to), the jersey number (`sVRt`), captaincy (`mCcl`), a copy of the player style (`QgFs`), the dressed flag (`hSZj`) and the line flags. The unique key `toWX` is globally increasing with gaps between blocks; its per-team contiguity is a creation-order artifact, not an invariant.

Consequences for any editor:

- **Moving a player** must relocate the occupant into a free slot of the destination block, never retag the slot's team field. Retagging breaks the block the game reads the roster from.
- **Trading** when both teams are full swaps the occupants and per-slot fields of one slot in each block.
- **Signing** into a full block requires growing the block, which inserts a record and shifts every later block.
- **Releasing** clears the slot's occupant, leaving an empty slot in the block.
- **Deleting** a slot slides the following records up; swap-with-last breaks the tiling.

The Studio enforces the tiling as a validation predicate and repairs out-of-order blocks on load ([Core Concepts](Core-Concepts)).

## Free agents and hidden players

Pool membership is **structural**: a player is off-roster if and only if their index row (`ZBac`) is referenced by no roster slot. There is no sentinel "free agent" team and no unindexed-bio pool. On Legacy, 5 531 index rows are placed and 556 are not; every bio has at least one index row, and some bios have several (a player carried in more than one pool).

`ZBac.RREB` (metadata name "table") is the **player type**, 0 built-in, 1 user-created, 2 special, 3 developer, correlating one-to-one with the bio's `active` flag. It is not roster status; an earlier reading that used it as dressed/scratched/minors was wrong. The real dressed flag is the 1-bit `eGlu.hSZj`. "In the minors" is not stored on the slot at all; it is a presentation concept for players whose main team differs from the team holding their rights.

## Line slots

Besides the occupant fields, each roster row carries one-bit **line flags** saying which lineup slots this player occupies: 59 flags on NHL 11 and 12, 71 from NHL 13 on (twelve 4-on-4 overtime slots were added). The 71 are: four centres, four left wings, four right wings, three left and three right defence, two goalies; two five-skater power-play units, two four-skater power-play units, two four-skater and two three-skater penalty-kill units, five shootout slots, two extra-attacker slots and three overtime units of four. The slot-to-field mapping is a fixed ordered list in the Studio's roster audit.

## NHL 08 differences

NHL 08 has an explicit free-agent list, keyed roster blocks with no empty-slot convention, and index key 0 is a real shipped player. A membership leaves by removing its roster row and arrives by inserting a row at the end of the team's block with a new key, which is exactly the delete-and-insert the NHL 08 roster save format declares for that table. NHL 09 to 14 have no key-0 row; from NHL 15 key-0 rows belong to players with another key.

## Capacity ceilings

The gen7 main database is packed: `max_records` equals `current_records` for the seven roster-bearing tables, so it cannot tell you how many players the game can hold. The vanilla **roster save** carries the same population with pre-allocated slack, and that `max_records` is the ceiling. On PS4 the main database carries the slack itself. The ceilings agree across platforms for the same roster year (PS3 NHL 15 roster save equals PS4 NHL 15 main database; PS3 Legacy equals PS4 NHL 16).

| Title | `ZBac` | `ubPc` | `dSvy` / `xmja` | `mHuy` / `fMVl` | `eGlu` |
|---|---|---|---|---|---|
| NHL 10 | 5 688 | 4 533 | 3 999 | 534 | 6 219 |
| NHL 11 | 7 093 | 5 910 | 5 260 | 650 | 7 728 |
| NHL 12 | 8 283 | 6 898 | 6 324 | 874 | 7 853 |
| NHL 13 | 8 907 | 6 946 | 6 407 | 839 | 10 457 |
| NHL 14 | 8 803 | 6 873 | 6 350 | 823 | 10 353 |
| NHL 15 (X360, PS3, PS4) | 8 948 | 7 034 | 6 479 | 855 | 10 619 |
| NHL Legacy, PS4 NHL 16 | 9 955 | 7 988 | 7 331 | 957 | 10 879 |
| PS4 NHL 17 | 10 935 | 9 428 | 8 236 | 1 192 | 11 714 |
| PS4 NHL 18 | 11 825 | 10 371 | 9 143 | 1 228 | 13 502 |
| PS4 NHL 19, 20 | 14 913 | 12 714 | 11 078 | 1 636 | 15 567 |
| PS4 NHL 21 | 15 615 | 12 439 | 10 841 | 1 598 | 16 576 |

Shipped populations are roughly half the ceilings (Legacy: 5 532 of 10 879 roster slots). Community rosters do spend the headroom; one NHL 19 custom roster holds 9 801 index rows against a vanilla 8 581. NHL 07, 08 and 09 ceilings are not established: their roster saves decode to a TDB without the roster core.

## Matching players across sources

Name plus date of birth is a viable primary key: 99.4% or more of bios on every anchor carry a plausible birth date. It is not unique: 0.1% to 6.1% of players collide even with the date, because the same real player is carried in more than one pool. Any importer must return candidates and let the user choose.

Last reviewed: 2026-10-20

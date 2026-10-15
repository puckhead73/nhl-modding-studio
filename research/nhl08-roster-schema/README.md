# NHL 08 Xbox 360 roster save vs `nhlng.db` — schema and save-model diagnosis

**Status:** research only, 2026-10-05. Nothing outside `research/nhl08-roster-schema/` was
modified. No application code, anchor, manifest, baseline approval, game install or save was
touched; Studio was not run. Every number below is reproducible from the committed anchors with
`python research/nhl08-roster-schema/scripts/run_all.py` (section 14).

## 0. Diagnosis and confidence

The NHL 08 Xbox 360 roster save is a **difflist (change-list) database generated from the game's
own schema**, not a snapshot of `nhlng.db`. Each of the 17 game tables in the `rosters` save group
owns up to three companion tables with their own four-character ids — *insert* (full records),
*delete* (key only) and *update* (key + every field marked `update="True"`). All 30 tables in the
save are **declared by name in `nhlng-meta.xml`** through `insertshortname`, `deleteshortname` and
`updateshortname`; none had to be inferred. The shipped save holds **zero records** in all 30
tables: a 14,780-byte schema followed by 1,223,672 zero bytes, and that tail equals, to the byte,
the sum of the declared difflist capacities times each table's record length.

Studio refuses the save because `analyze_bytes` requires the roster's inner TDB to share canonical
table ids with `nhlng.db` and to carry equal `current_records` per shared table. Difflist
companions never share ids and an empty change list has no rows, so the refusal is structural,
not a format mismatch. The "different database format" wording quoted in the brief is wrong in
substance: same container, same TDB v8 big-endian codec, different save model. While this
research ran, a concurrent session placed an untracked `roster_difflist.rs` in `studio-domain`
that recognises exactly this model (qualified on NHL 07 only) and changed the refusal text; the
working-tree state is described as found in section 7 and was neither modified nor executed.

| Claim | Confidence | Basis |
|---|---|---|
| Container structure, checksums, boundaries, padding | **high** | all CRCs recomputed and matched; zero tail verified; allocation arithmetic exact on roster *and* dynasty |
| Save model = difflist of the `rosters` group | **high** | 30/30 tables declared in the meta; descriptors verified field-by-field against the game DB |
| Table → parent mapping, roles, keys, capacities | **high** | declared, unique, descriptor-verified; capacity sum reproduces the file size |
| Semantics of a populated *insert* table | **high** | 27 populated insert tables observed in the NHL 08 dynasty save (`max == cur`, full records, `data_alloc_type = 6`) |
| Semantics of populated *update* / *delete* tables, key matching at load | **medium** | implied by the key-only / key+update layouts; **no populated roster update or delete table exists in the repository** |
| Runtime acceptance of a Studio-written difflist, create/delete flow | **unknown** | needs the in-game experiments in section 13 |

## 1. Inputs, hashes and provenance

`output/input_hashes.txt` (SHA-256, size):

| Item | Path | Size | SHA-256 |
|---|---|---|---|
| Game DB | `anchors/xbox360/nhl08/databases/nhlng.db` | 1,556,560 | `587204c3…dc858b9` |
| Game meta | `anchors/xbox360/nhl08/databases/nhlng-meta.xml` | 243,270 | `d2b6feb2…b8656da1` |
| Roster anchor | `anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658` | 1,238,480 | `b7a37835…0fe035755` |
| Dynasty anchor | `anchors/xbox360/nhl08/savefiles/DYNASTY1 20260718064632/DYNASTY1 20260718064632` | 2,461,712 | `e0c93382…e79a25` |
| Manifest | `anchors/xbox360/nhl08/game-files.json` | 6,965 | `3c4d2b19…013c2b4` |

Provenance, from the manifest: the DB and meta hashes match the manifest's `db/nhlng.db` and
`db/nhlng-meta.xml` entries of the *NHL 08 Xbox 360 USA original ISO* (`image_sha256`
`4cb8f03f…c62624b`), i.e. disc provenance is established for those two. Both save anchors have
`disc_matches: []` — they are user captures with no disc provenance, as the brief states.

Two provenance observations the brief did not anticipate:

- `anchors/ps3/nhl08/databases/nhlng.db` and `nhlng-meta.xml` are **byte-identical** to the Xbox
  360 anchors (same hashes), as already recorded for other PS3 titles.
- `anchors/ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA` is **byte-identical to the
  Xbox 360 roster anchor** (same SHA-256 `b7a37835…`, same 1,238,480 bytes). The two "platform"
  roster anchors are therefore one capture, and it is not established which console produced it.
  The PS3 dynasty `SYS-DATA` (`09b844e6…`) is a different capture from the Xbox 360 dynasty
  (different `checksum_a`, inner `db_size` 525,408 vs 525,416), so the dynasty anchors are
  independent. Nothing in this report depends on which console wrote the roster save: the save
  is an empty change list whose content is a pure function of the schema.

## 2. Container verification

`output/container_checks_roster.txt`, `output/container_checks_dynasty.txt`,
`output/container_checks_gamedb.txt`.

### 2.1 Roster anchor

| Offset | Value | Meaning | Verified |
|---|---|---|---|
| `0x00` | `Rosters\0…` (16 B) | gen6 roster magic; discovery → kind `roster` | yes |
| `0x10` | `70380061` | `checksum_a` = CRC-32/IEEE over `[0x1C..EOF]` | recomputed, match |
| `0x14` | `3` | container version word → `ContainerVersion::RawV3`, header length `0x1C` | yes |
| `0x18` | `0` | unknown word | — |
| `0x1C` | `DB 00 08 01 00 00 00 00 00 00 39 BC …` | inner TDB: version 8, big-endian, `db_size` 14,780, 30 tables, header CRC `9256FC10` | header CRC recomputed (CRC-32/MPEG-2 over the first 0x14 bytes), match |
| `0x34`–`0x124` | directory | 30 × 8-byte `(raw id, offset)` | parsed |
| `0x124`–`0x39D4` | table blocks | 0x28-byte headers + 0x10-byte descriptors, **no record block** (`max_records = current_records = 0` in every table) | parsed; blocks contiguous in directory order |
| `0x39D4` | `B2F2BE2C` | EOF CRC over the last table's body | recomputed, match |
| `0x39D8`–`0x12E5D0` | `00 …` | 1,223,672 zero bytes | all-zero confirmed |

Inner CRC scopes, recomputed exactly as `tdb-core::recompute_crcs` does: file-header CRC ok;
`prior_crc` ok 30/30; per-table `header_crc` ok 30/30; EOF CRC ok. Every table has
`data_alloc_type = 2`, word@+0x10 = 0, word@+0x18 = 0xFFFF, `index_count = 0`, and
`record_len_bits = record_len_bytes × 8 − 1`. No second `DB` header exists anywhere in the file.

### 2.2 Capacity model (why the file is exactly 1,238,480 bytes)

For each save table, capacity = `maxinsert` / `maxdelete` / `maxupdate` of its role, where `-1`
means the parent's `max_records` in `nhlng.db`. Σ(capacity × record bytes) over the 30 tables =
**1,223,672 = the zero tail**, and 14,780 + 1,223,672 = 1,238,452 = file − 0x1C. The save is a
fixed pre-allocation of the maximum change list; a populated save never has to grow the file.

### 2.3 Dynasty anchor (container only; content in section 9)

| Region | Size | Content |
|---|---|---|
| `0x00`–`0x1C` | 28 | magic `Dynasties`, `checksum_a` `47530247` (CRC-32/IEEE `[0x1C..EOF]`, match), version word 3, word@0x18 = 0 |
| `0x1C`–`0x8D9BC` | 580,000 | **non-TDB preamble**: 50,717 non-zero bytes; starts `00 00 08 34`, then `FF FF` every 14 bytes for the first 256 bytes; rating-like byte strings at an 80-byte stride near offset 106,482. Not decoded (dynasty runtime state, out of scope) |
| `0x8D9BC`–`0x10DE24` | 525,416 | inner TDB, 111 tables, all four CRC scopes ok |
| `0x10DE24`–`0x259010` | 1,356,268 | zero tail |

The same allocation rule holds: schema-only bytes (`db_size` − record data = 525,416 − 495,484 =
29,932) + Σ(capacity × record bytes) (1,851,752) = **1,881,684 = file − TDB base**, difference 0.
So the dynasty save is the same fixed-size pre-allocation with the inner TDB positioned after a
title-specific preamble.

### 2.4 Game DB

79 tables, 40,866 records, every CRC scope ok, `data_alloc_type = 2` everywhere (also on
populated tables — so `6`, seen only on the dynasty's populated difflist tables, is not simply
"has records"; see section 9).

## 3. The save model from NHL 08's own metadata

`output/update_semantics.txt`, `output/difflist_map_roster.txt`.

- Every `<table>` in `nhlng-meta.xml` carries `difflistproperty="ENABLED"`; 65 carry
  `savegroups`; 56 declare `insertshortname`/`maxinsert`, 50 `deleteshortname`/`maxdelete`, 55
  `updateshortname`/`maxupdate`. The save groups are `rosters` (17 tables), `currentrosters` (8),
  `defaultrosters` (3), `rosterupdate` (12), `rosterupdatenogoalieequipment` (11),
  `defaultbiosattributes` (5), `settings` (5), `league` (29), `dynasty` (40), `allloaded` (65).
- The roster save contains **exactly** the declared companions of the 17 `rosters`-group tables:
  14 insert/delete tables (11 insert + 3 delete) and 16 update tables, no table unaccounted for and
  no `rosters` parent without its declared companions. The brief's preliminary "14 declared + 16
  proposed update" count is confirmed, except that the 16 are *declared*, not proposed.
- Companion layouts (section 5): insert = parent descriptors verbatim; delete = the parent's key
  field alone at bit 0 in a 4-byte record; update = `{update="True"} ∪ {key}` in parent descriptor
  order, dense-packed (which equals the parent layout whenever every non-key field is marked).
- The save's directory order is the meta's **document order** of the parents, with the roles
  adjacent per parent in the fixed sequence insert → delete → update
  (`output/save_directory_order.txt`). The game DB's own directory is not in that order, so the
  save is produced by a writer that walks the schema definition, exactly as on NHL 07.

## 4. Complete mapping and capacity table

30 save tables, all declared. Capacity source "-1" = parent `max_records`. Key = the parent field
marked `key="True"`; it is the only field never marked `update="True"`.
Confidence "declared+verified" = declared in the meta **and** field set / descriptors verified
against the game DB; the field-set cross-check column records how many *other* meta tables share
the same field-id set (a measure of how ambiguous a pure field-set inference would have been).

| Save | Role | Parent | Parent name | Key (bits) | Fields | Rec B | Capacity | Bytes | Cross-check |
|---|---|---|---|---|---|---|---|---|---|
| `gKeN` | insert | `lVMf` | exhibitiongoalieequipment | `zIBw` game_id (13) | 75/75 | 68 | 60 | 4,080 | unique |
| `JJDo` | update | `lVMf` | exhibitiongoalieequipment | game_id | 75/75 | 68 | 315 | 21,420 | unique |
| `hTDj` | update | `NDEo` | customteamequipmenttable | `qEfv` index (10) | 32/32 | 32 | 600 | 19,200 | +1 (`LwGR` league twin) |
| `BlkH` | insert | `ajmx` | exhibitionskaterequipment | game_id | 17/17 | 12 | 300 | 3,600 | unique |
| `MSZu` | update | `ajmx` | exhibitionskaterequipment | game_id | 17/17 | 12 | 2,647 | 31,764 | unique |
| `mASv` | update | `oYeE` | exhibitionteamstrategies | index (8, FK exhibitionteams) | 11/11 | 8 | 138 | 1,104 | +1 (`QfUJ`) |
| `KjDh` | update | `Roup` | custompalettetable | index (5) | 5/5 | 4 | 20 | 80 | +1 (`SsSL`) |
| `sAud` | insert | `nAUy` | stockgoalieequipment | `XWot` key (12) | 79/79 | 148 | 60 | 8,880 | unique |
| `aGgS` | delete | `nAUy` | stockgoalieequipment | key | 1/79 | 4 | 236 | 944 | +6 (any `XWot`-keyed table) |
| `VDLc` | update | `nAUy` | stockgoalieequipment | key | 79/79 | 148 | 236 | 34,928 | unique |
| `sAMA` | insert | `QEoV` | exhibitionfreeagents | `TWSX` playerindex (13, FK exhibitionplayers) | 1/1 | 4 | 600 | 2,400 | unique |
| `mGOv` | delete | `QEoV` | exhibitionfreeagents | playerindex | 1/1 | 4 | 156 | 624 | +1 (`kGzd`) |
| `SREo` | update | `ttOk` | exhibitionteams | index (8) | 24/31 | 200 | 138 | 27,600 | unique |
| `PPVt` | update | `Vzjq` | customfacetable | `qFky` playerid (13) | 13/13 | 8 | 180 | 1,440 | unique |
| `LZbD` | insert | `caBZ` | exhibitionplayers | index (13) | 3/3 | 4 | 1,000 | 4,000 | unique |
| `Frps` | delete | `caBZ` | exhibitionplayers | index | 1/3 | 4 | 3,257 | 13,028 | +34 (any `qEfv`-keyed table) |
| `qWeu` | update | `caBZ` | exhibitionplayers | index | 3/3 | 4 | 3,257 | 13,028 | unique |
| `ibGM` | insert | `ulGe` | exhibitionrostertable | `XWot` key (13) | 71/71 | 16 | 1,500 | 24,000 | +1 (`rgQY` league twin) |
| `AtUj` | delete | `ulGe` | exhibitionrostertable | key | 1/71 | 4 | 2,525 | 10,100 | +6 |
| `vozd` | update | `ulGe` | exhibitionrostertable | key | 71/71 | 16 | 2,525 | 40,400 | +1 (`rgQY`) |
| `UHgc` | update | `vbHh` | stockteamequipmenttable | index (13) | 33/33 | 32 | 8,000 | 256,000 | unique |
| `PsvH` | insert | `cPbu` | exhibitionplayerbiotable | game_id (13) | 32/32 | 184 | 360 | 66,240 | unique |
| `CvSu` | update | `cPbu` | exhibitionplayerbiotable | game_id | 32/32 | 184 | 2,962 | 545,008 | unique |
| `TUaR` | update | `OEtS` | exhibitionarena | index (8) | 33/33 | 104 | 138 | 14,352 | +1 (`PSFv`) |
| `kbtS` | insert | `yvSd` | exhibitionskaterai | game_id | 30/30 | 24 | 300 | 7,200 | unique |
| `RosT` | update | `yvSd` | exhibitionskaterai | game_id | 30/30 | 24 | 2,647 | 63,528 | unique |
| `PxTk` | insert | `yuHm` | exhibitiongoalieai | game_id | 26/26 | 20 | 60 | 1,200 | unique |
| `uyGF` | update | `yuHm` | exhibitiongoalieai | game_id | 26/26 | 20 | 315 | 6,300 | unique |
| `yJYN` | insert | `sozv` | stockteamcolorstable | index (9) | 6/6 | 8 | 10 | 80 | unique |
| `XGls` | update | `sozv` | stockteamcolorstable | index | 4/6 | 8 | 143 | 1,144 | unique |

Σ bytes = 1,223,672. Reconciled counts: 30 tables = 11 insert + 3 delete + 16 update, over 17
parents: 3 with all three roles (`nAUy`, `caBZ`, `ulGe`), 1 with insert + delete (`QEoV`, whose
only field is its key, so no update table exists), 6 with insert + update (`lVMf`, `ajmx`, `cPbu`,
`yvSd`, `yuHm`, `sozv`) and 7 with update only (`NDEo`, `oYeE`, `Roup`, `ttOk`, `Vzjq`, `vbHh`,
`OEtS`); 3×3 + 1×2 + 6×2 + 7×1 = 30.

Capacity consequences for a writer:

- Bio rows (`cPbu`) can be inserted (360) and updated but **never deleted**; player removal must go
  through `exhibitionplayers` delete (`Frps`) and roster delete (`AtUj`).
- AI and equipment rows have insert and update companions, no delete.
- `exhibitionteams`, `exhibitionarena`, `exhibitionteamstrategies`, the custom face/palette/team
  equipment tables and `stockteamequipmenttable` are update-only: teams and arenas cannot be
  created or removed through a roster save; the 30 pre-allocated "Custom Team 27…" rows
  (`table=1`, `active=0`) are the only way to add teams.
- `stockgoalieequipment` (`nAUy`) is a separately keyed preset table (`key` 0..235 plus the
  goalie's `game_id` and name) with all three roles; it is **not** a twin of
  `exhibitiongoalieequipment` (75 vs 79 fields, 68 vs 148 B).

## 5. Descriptor comparison (types, widths, offsets, encoding)

`output/difflist_map_roster.txt` (section "descriptor comparison"), `output/roster_group_meta.txt`
(every field of every `rosters` parent with its bit offset in the parent and in each companion).

- **Field ids**: every field in the save exists in its parent with the same `kind`
  (0 = string, 3 = integer in the binary descriptor) and the same `width`; no exceptions.
- **Insert tables**: field list, order and bit offsets identical to the parent; same record length.
- **Delete tables**: one field, the parent's key, at bit 0 in a 4-byte record — "re-packed"
  relative to the parent, where the key sits at an arbitrary offset (`cPbu.game_id` at bit 1432,
  `ulGe.key` at bit 54, `nAUy.key` at bit 1134, `caBZ.index` at bit 2).
- **Update tables**: 14 of 16 contain every parent field (all non-key fields marked
  `update="True"`) and therefore keep the parent's exact offsets and record length. The two
  reduced ones are dense re-packs in parent descriptor order:
  - `SREo` (exhibitionteams, 24 of 31 fields, 200 vs 204 B) drops `table`, `smoke_r/g/b`,
    `ahlaffiliate`, `original`, `itechvisorcolor`; e.g. `shortname` moves from bit 336 to 328,
    `index` from 1208 to 1184.
  - `XGls` (stockteamcolorstable, 4 of 6 fields) drops `team` and `colorid`; `index` moves from bit
    34 to 24.
- **Encoding**: integers are MSB-first bit fields and strings inline NUL-padded bytes, identical in
  parent and companion; no companion introduces a different depth or type. The meta `depth` equals
  the binary `width` for every `rosters` field. The binary descriptor order differs from the meta
  document order in every table (a known property of this format), so a companion's layout must be
  derived from the **binary** parent descriptors, never from the XML field order.

## 6. Player → slot → roster / free agent → team: the key graph

`output/key_graph.txt`, `output/record_trace.txt`, `output/record_trace_player.txt`.

Relationships verified by record contents of `nhlng.db`:

| Edge | Verified |
|---|---|
| `exhibitionplayers` (`caBZ`, 3,257 rows, 4 B): `index` 0..5179 unique, ascending, 1,923 gaps; `playerid` → `exhibitionplayerbiotable.game_id` | every `playerid` resolves; every bio is slotted at least once |
| `table` flag: 0 = stock (3,077 slots, playerids 0..4164), 1 = custom (180 slots, playerids 5000..5179, `index == playerid`) | yes |
| 259 bios occupy more than one slot (international duplicates: e.g. Crosby below) | yes |
| `exhibitionrostertable` (`ulGe`, 2,525 rows): surrogate `key` 0..2699 unique (≠ record index except 23 rows, ≠ `playerindex` except 1 row); `playerindex` unique → `caBZ.index`; `team` 0..107 → `exhibitionteams.index` | all resolve |
| `exhibitionfreeagents` (`QEoV`, 156 rows): `playerindex` → `caBZ.index`, disjoint from the roster | yes |
| roster ∪ free agents = 2,681 of 3,257 slots; 576 unassigned (180 custom placeholders + 396 stock slots) | yes |
| `exhibitionteams` (`ttOk`, 138 rows, full team table): `index` == record index; leagues {0: 32 NHL, 1: 29, 2: 12, 3: 14, 4: 21 international, 5: 30 custom}; `table=1`/`active=0` on the 30 custom teams; `exhibitionarena.index` and `exhibitionteamstrategies.index` cover the same 138 | yes |
| skater AI (`yvSd`, 2,647) ∪ goalie AI (`yuHm`, 315) = exactly the 2,962 bio `game_id`s, disjoint; goalies are `position = 4` (not 0 as on later titles' vocabularies — verify per title) | yes |
| skater/goalie equipment (`ajmx`/`lVMf`) carry exactly the AI id sets and are record-order aligned with the AI tables | yes |
| 150 custom skater + 30 custom goalie AI/equipment rows are pre-allocated for the 180 custom bios | yes |
| `stockgoalieequipment.game_id` ⊂ goalie AI ids (236 of 315) | yes |
| bio `team` vs roster `team` disagree on 249 rows — all on international teams (89, 90, 92, 104, 107); bio `jersey` vs roster `jersey` disagree on 489 | observation: the roster row, not the bio, carries the per-team assignment |

Representative trace (stable keys, with offsets):

| Record | Offset | Content |
|---|---|---|
| `cPbu[1977]` | `0x7FD18` | `game_id 3109` Sidney Crosby, `team 23`, `jersey 87`, `position 0`, `headid 1082`, `contractdollars 280` |
| `caBZ[2245..2247]` | `0xAE47C` | three slots `index 2947/2948/2949`, all `playerid 3109`, `table 0` |
| `ulGe[531]` | `0xB1A14` | `key 593`, `playerindex 2947`, `team 23` Pittsburgh Penguins, `rosterstatus 1`, `captain 1`, `l1c 1`, `pp1c 1` |
| `ulGe[687]` | `0xB23D4` | `key 769`, `playerindex 2948`, `team 30` Eastern All-Stars, `captain 2` |
| `ulGe[2073]` | `0xB7A74` | `key 2248`, `playerindex 2949`, `team 89` Canada |
| `yvSd[1779]` / `ajmx[1779]` | `0xC3FCC` / `0xCE5C8` | `game_id 3109`: speed 42, passing 49, potential 45; stick 7, stick flex 1 |
| goalie `cPbu[129]` | `0x2CCD8` | `game_id 150` Michael Wall, `position 4` → `yuHm[0]` (`0x1FC90`), `lVMf[0]` (`0x21A04`), `nAUy[0]` (`0x130C8C`, preset `key 0`) |
| custom `cPbu[2782]` | `0xA3FB0` | `game_id 5000`, empty names, `position 4`, `yeardrafted 127` sentinel → `caBZ` slot 5000 (`table 1`), `Vzjq` face row `playerid 5000` |
| free agent | `QEoV[1]` `0x1F85C` | `playerindex 152` → `caBZ[131]` (`index 152`, `playerid 147`) |

What this means for a difflist writer: a scalar edit on a stock player is a `CvSu` row keyed by
`game_id`; a team move is a `vozd` row keyed by the roster row's surrogate `key` (not by
`playerindex`), or `AtUj` + `ibGM` for a slot that changes identity; free-agent membership is
`sAMA`/`mGOv` keyed by `playerindex`; lines/captaincy live in the same `ulGe` row. New roster rows
need a fresh `key` — the allocation rule (max+1 = 2700? reuse of the 175 gaps?) is unobserved.

## 7. Where Studio's snapshot assumptions reject or misinterpret this save

Line numbers are from the working tree of branch `codex/overnight-game-qa` on 2026-10-05
(`crates/` is heavily modified and uncommitted; the `tdb-defs` change below is part of that).

1. **The refusal** — `crates/studio-domain/src/workspace.rs:2945-2976` (`analyze_bytes`):
   intersects canonical table ids of `nhlng.db` with the roster's, requires equal
   `current_records` per shared table, and errors with "This roster save shares no database
   tables … Its save model cannot use record-index mirroring" when the intersection is empty
   (`:2969-2973`; the older "different database format" wording quoted in the brief has already
   been replaced in the working tree). For NHL 08 the intersection *is* empty (30 companion ids vs
   79 game ids). Even a non-empty intersection would be mis-handled: the equal-row rule encodes
   the snapshot model, and a difflist has 0 rows.
   **Concurrent work, found while this research ran:** `workspace.rs:2933-2941` (`analyze`) now
   first calls `crate::roster_difflist::recognize` and, when it succeeds, refuses with "This
   roster stores changes to the game database. Roster-save editing for this save model is not
   yet enabled". The module `crates/studio-domain/src/roster_difflist.rs` (untracked, 441 lines)
   implements exactly the declaration-only classification of section 11 step 1 (`recognize`,
   `:80-173`: every save table must be a declared `rosters`-group companion, one declared key,
   field set = all / key / update ∪ key, parent kind and width equal, declared capacity honoured),
   a key-addressed overlay (`effective_roster`, `:177-290`, delete → insert → update order, update
   combined with insert/delete of the same key refused) and an **NHL 07-only** experimental jersey
   writer (`experimental_nhl07_jersey`, `:296-441`, gated on 31 tables, a 508-byte `cPbu` and the
   1,598,888-byte file, so it cannot touch NHL 08). Its tests
   (`crates/studio-domain/tests/nhl07_difflist.rs`) run only on the NHL 07 anchor. By static
   reading, `recognize` accepts the NHL 08 anchor: 30 declared `rosters` companions = 30 save
   tables, one key per parent, field sets and widths as in sections 4–5; it does not check bit
   offsets or record length, which sections 5 adds as evidence. This was not executed here.
2. **The model assumption** — `workspace.rs:15-19` ("verified record-index-aligned subset of
   nhlng.db … verified for nhl14 PS3 only"), `:301-316` (`with_roster` computes
   `(shared_tables, aligned)`), `:1055-1059` (`roster_aligned`). Every write fans out only under
   `m.aligned && m.shared_tables.contains(table)` (`:1243`, `:1312`, `:1352`, `:1423`, `:2176`,
   `:2305`, `:2374`, `:2422`): a `(table, record, field)` mirror, meaningless for a key-addressed
   overlay.
3. **Registry wiring** — `crates/studio-api/src/registry.rs:341-350`: "PS3/Xbox/Recomp mirror:
   there nhlng.db *is* the roster (record-index aligned)"; the Xbox roster slot is always offered
   as a mirror, so NHL 08 fails at open instead of opening roster-only.
4. **Hard-coded exclusion** — `crates/studio-api/src/bin/baseline_inventory.rs:101-107`
   (`incompatible_roster`: "shares no database tables … cannot mirror its edits") applied to NHL 08
   at `:127`; the NHL 07 entry at `:124` carries the sibling text. The test
   `crates/studio-api/tests/nhl07_source_qa.rs:218-232` pins the "shares no database tables"
   wording for NHL 07; there is no NHL 08 counterpart.
5. **Discovery** — `crates/studio-domain/src/discovery.rs:982-999` (`kind_from_magic`) classifies
   by the type token in the magic: `Rosters` → `roster`, `Dynasties` → `beagm`. Correct, but
   it then presents the roster as a writable mirror candidate.
6. **Extraction** — `crates/tdb-savedata/src/header.rs:111-143` resolves version word 3 to
   `RawV3` (payload at `0x1C`); `crates/tdb-savedata/src/extract.rs:22-37` (`inner_db_len`)
   requires `payload[0..2] == "DB"` and trims to `db_size`. For the **roster** this is right (the
   TDB starts at `0x1C`; the zero tail is kept as the "trailer" that `pack` re-attaches). For the
   **dynasty** it is wrong: the payload starts with the 580,000-byte preamble, so `extract_db`
   returns `SaveError::NotTdb` although a valid 111-table TDB sits at `0x8D9BC`. Any future
   dynasty support needs a scan-for-`DB`-header step (`tdblib.scan`) rather than a fixed offset.
7. **Metadata** — `crates/tdb-defs/src/lib.rs:342-352, 366-380` now parses
   `insertshortname`/`deleteshortname`/`updateshortname`, `max*`, `savegroups`, the exact
   `update="True"` field list and the keys into `SaveTableMeta` (working-tree change, not at
   HEAD); `lib.rs:440-442` exposes `save_tables()`, consumed only by `roster_difflist.rs:92`. The
   NHL 07 report's statement that these attributes are ignored is stale for the working tree.
   Independently, `lib.rs:402` still sets `updatable = attribute != "False"`: the NHL 08 meta never
   writes `update="False"`, marks 1,127 fields `True` and leaves 451 unmarked, so Studio currently
   treats all 1,578 fields as updatable (section 10).
8. **Documentation** — `docs/formats/metadata-xml.md:17,31` lists the attributes without
   semantics; `docs/plans/project-completion-roadmap.md:353` records that NHL 07's and NHL 08's
   roster saves "decode to a TDB without the roster core so their capacities are unknown". The
   capacities are in fact declared (section 4) and the 30-table TDB *is* the roster core, as a
   change list.

## 8. NHL 07 vs NHL 08: shared and title-specific

`output/crosstitle_compare.txt`, `output/difflist_map_nhl07_roster_declared.txt`.

**Correction to the NHL 07 report.** The NHL 07 meta declares `updateshortname` on 41 tables.
All 12 update tables that report resolved by field-set identity are declared there with the same
parents — including the two "resolved by adjacency" pairs `DZfz→dDrd` and `YvhR→Raym`. Its
hypothesis H2 is closed by declaration, no runtime experiment is needed for it, and no mapping on
either title rests on adjacency or cross-title id stability. (Its table-count sentence should read
19 insert/delete + 12 update = 31.)

Shared between the two titles (safe to implement once):

- Container: `Rosters` magic, version word 3 (`RawV3`), `checksum_a` CRC-32/IEEE over
  `[0x1C..EOF]`, inner TDB v8 big-endian at `0x1C`, four inner CRC scopes.
- Difflist grammar: three roles with the layouts of section 5; directory in meta document order
  with insert → delete → update per parent; empty save = schema + zero tail; file size =
  schema-only + Σ(capacity × record bytes); `-1` = parent `max_records`.
- Meta vocabulary: `difflistproperty`, `savegroups`, `insert/delete/updateshortname`, `max*`,
  `update="True"` as the exact update set, `key="True"`.
- 33 parents keep identical `(insert, delete, update)` id triples across the two metas; 11 shared
  save ids appear in both roster saves (`AtUj CvSu Frps LZbD RosT ibGM mASv mGOv sAMA uyGF vozd`),
  but **five of them have different layouts** (`CvSu` 43→32 fields, `RosT` 27→30, `uyGF` 23→26,
  `ibGM`/`vozd` 70→71). Ids are stable; layouts are not. A classifier must map by the *target
  title's* meta and verify descriptors against the *target title's* game DB, never reuse a mapping.

Title-specific (must stay per title):

| Aspect | NHL 07 | NHL 08 |
|---|---|---|
| `rosters` parents / save tables | 16 / 31 | 17 / 30 (9 parents in common) |
| Player bio model | `exhibitionplayerbiotable` (stock, 46 fields, 508 B, byte-identical twin `stockplayerbiotable`) + `customplayerbiotable` (8-bit id, insert/delete) + `exhibitiondownloadplayerbio` (200 placeholders) | one `exhibitionplayerbiotable` (32 fields, 184 B, no twin); custom players are 180 **pre-allocated** rows `game_id 5000..5179` reached by `exhibitionplayers.table = 1`; no download bios |
| Strings | 1280-bit names | 320-bit names |
| Equipment | inside the bio (`stick`, `blocker`, …) | separate `exhibitionskaterequipment` / `exhibitiongoalieequipment` + `stockgoalieequipment` presets + team equipment / colour / palette / face tables |
| Teams | thin `exhibitionteams (index, teamid, table, division)` selector onto `stockteamtable`; insert/delete companions | full 31-field `exhibitionteams`, update-only (`SREo`), 30 custom placeholder teams |
| Arena | 3-field selector, insert companion | full 33-field table, update-only |
| Roster row key | `playerindex` (FK) | surrogate `key` (`XWot`), `playerindex` is a plain FK and *is* update-marked |
| AI tables | skater 27 fields / goalie 23 (several 2-bit); stock/custom/download triplets | skater 30 / goalie 26 (all 6-bit); single exhibition pair + rookie/league variants outside the roster group |
| `rosterversiontable` | present, 1 row | absent |
| Goalie position code | (not re-checked here) | `position = 4` |
| Capacities | e.g. bio update 2,250, roster insert 3,200 | bio insert 360 / update 2,962, roster insert 1,500 / update 2,525 |

## 9. Dynasty anchor: populated difflist evidence (dynasty observations only)

`output/dynasty_evidence.txt`. These are observations about the **dynasty** save group; they do
not prove roster behaviour.

- The inner TDB holds the 111 declared companions of all 40 `dynasty`-group parents (every
  parent's insert and delete table, plus the 31 declared update tables), in meta document order.
- **27 insert tables are populated**, every one with `max_records == current_records`,
  `data_alloc_type = 6`, inline record data after the descriptors, and db_size grown accordingly.
  **All 40 delete tables and all 31 update tables are empty** (`alloc = 2`). The dynasty creates a
  fresh league, so it is written entirely as inserts; it gives no example of an update or delete
  row anywhere in the repository.
- Insert rows are full parent records with meaningful content: `fcdR` (leagueplayerbiotable,
  1,504 rows, `index` 0..1503), `Pwkd` (leaguerostertable, 1,314 rows), `ioPQ`
  (leaguefreeagents, 311 rows, `playerindex` 673..1503), `ogsK`/`vmSc` (league skater/goalie AI,
  1,328/176 rows), `DnxP` (leagueteams, 63 rows, "ANAHEIM DUCKS" …), `ULcv` (schedule, 2,391 rows),
  `eiHK` (leaguearena, 63 rows, "Honda Center" …). Key columns are unique and ascending in every
  populated table.
- Dynasty update tables show the reduced-set layout clearly: `azBc` (leaguerostertable update)
  is 12 B vs the 16 B insert, `Krok` (leagueplayerbiotable update) 12 B vs 188 B, `SoUW`
  (leagueteams update) 4 B vs 208 B — the league tables mark few fields `update="True"`, and the
  companions shrink accordingly. This confirms the update-table construction rule on a second,
  independently authored group.
- Six league tables declare `maxupdate="0"` (`leaguetransactiontable`, `leaguearena`,
  `leaguecustomfacetable`, `leaguerookiegoalieequipment`, `leaguenews`,
  `leaguecustomteamequipmenttable`): an update table exists in the schema but has no capacity.
  Capacity 0 must be honoured by a writer.
- The 580,000-byte preamble before the TDB is dynasty runtime state outside the difflist.

Hypothesis for the roster, supported by this evidence but **not observed**: when the game writes a
non-empty roster, each touched companion gets `max = cur = n`, `data_alloc_type = 6`, inline
records, a grown `db_size`, recomputed inner CRCs, the file stays 1,238,480 bytes, and
`checksum_a` is recomputed over the whole tail.

## 10. Metadata update semantics vs Studio's editability policy

`output/update_semantics.txt`.

- `update` takes exactly two states in the NHL 08 meta: `"True"` (1,127 fields) or absent (451).
  `update="False"` never occurs. Key fields (79) are never marked `update`.
- In the `rosters` group the unmarked fields are exactly: the key of every table (17), seven
  `exhibitionteams` fields (`original`, `smoke_r/g/b`, `itechvisorcolor`, `ahlaffiliate`,
  `table`) and two `stockteamcolorstable` fields (`team`, `colorid`). `exhibitionplayers` marks
  both `playerid` and `table` (NHL 07 marks neither), and `exhibitionfreeagents` has only its key.
  The update companions contain **exactly** `{update="True"} ∪ {key}` for all 16 (verified), so
  for this title *absent means not written by an update row*, and the key rides along as the
  address, not as an updatable value.
- No table marks fields `update="True"` without declaring an `updateshortname`; conversely every
  `updateshortname` table has marked fields except where `maxupdate="0"`.
- Studio's `FieldMeta.updatable` (`tdb-defs/src/lib.rs:401`, "absent ⇒ updatable") is a
  **different policy** from the game's write path. For direct `nhlng.db` editing it is a
  deliberate editability decision and may stay. For a difflist writer it is simply wrong: a value
  placed in a field that is not in the update companion has nowhere to go. The two notions must be
  kept apart: `SaveTableMeta.update_fields` (exact declaration) governs what a difflist row can
  carry; `FieldMeta.updatable` governs what the editor offers. A field editable in Studio but
  outside the update set (`exhibitionteams.ahlaffiliate`, `stockteamcolorstable.team`, …) must be
  shown as "not savable to a roster save" rather than silently dropped.

## 11. Narrowly scoped implementation plan (not implemented)

Ordered so each step is independently reviewable and nothing writes a save until the runtime
experiments of section 13 exist. The untracked `roster_difflist.rs` (section 7, item 1) already
contains an NHL 07-qualified form of steps 1 and 2; what follows describes the NHL 08 scope those
steps must reach, so that the remaining work is "extend and qualify", not "build". Nothing in
that module was changed or run by this research.

**Step 1 — save-model classification (read-only, replaces the refusal).** In `studio-domain`
beside `analyze_bytes`, given the roster's inner TDB, the game DB and `FieldMetaIndex`:
- `Snapshot` when table ids intersect (current behaviour, unchanged for NHL 09–Legacy and PS3).
- `Difflist { parents }` when **every** roster table id is declared by some meta table's
  `insert/delete/updateshortname` (via `SaveTableMeta`, which already exists), the parent is in
  the game DB, the parent's `savegroups` contains `rosters`, and the companion's descriptors verify
  against the parent per section 5 (insert = all fields and offsets; delete = key at bit 0; update
  = update ∪ key, dense-packed in parent order). Record per table: role, parent, key field, declared
  capacity, verified layout.
- `Unsupported { reason }` otherwise, naming the first offending table id; this replaces the
  "different database format" text and the `baseline_inventory` reason strings for NHL 07 and NHL
  08 with "difflist roster save; overlay editing not yet implemented".
  No adjacency or cross-title heuristics: anything undeclared is unsupported.

**Step 2 — read-only effective roster.** A pure function `overlay(parent_table, companions) →
records`: start from the parent's records; remove rows whose key appears in the delete table;
replace the update-set fields of rows whose key appears in the update table (unknown key →
diagnostic, row ignored); append insert rows (key collision with base → diagnostic). For the
shipped save this returns the game DB unchanged, so every existing NHL 08 view keeps its data.
Expose it through `Workspace` as a display-only roster session with **no mirror** and every
edit refused with a precise message; `roster_aligned()` stays `false`. Precedence is fixed and
one-directional — the save overlays the base; the base is never rewritten from the save and the
save is never derived from base edits — until a design decision says otherwise.

**Step 3 — writer (after section 13 experiments 1–4 are recorded).** Edit capture into companion
rows keyed as in section 6; serialisation by growing each touched companion from `max = 0`
(`tdb-edit` already has `grow_table_max` / `append_record`), `data_alloc_type` per the observed
value, CRCs via `tdb-core::recompute_crcs`, file length fixed at 1,238,480 bytes, `checksum_a` via
`checksum::recompute_a_only`; refuse at `maxinsert`/`maxdelete`/`maxupdate` (including 0).
Routed through the single pending-changes review as one target of the coordinated transaction.

For NHL 08 specifically, the delta over the existing NHL 07-qualified module is: (a) a test that
`recognize` on the NHL 08 anchor yields the 30 mappings of section 4 with the capacities listed
there; (b) bit-offset and record-length verification of companions (section 5), so a re-packed
update table such as `SREo` is checked against the parent layout and not only by field set;
(c) `effective_roster` identity on the NHL 08 anchor over all 17 parents, including the
surrogate-keyed `ulGe` and the preset-keyed `nAUy`; (d) the refusal/notice text and the
`baseline_inventory` reason for NHL 08 updated to the change-list wording; (e) **no** NHL 08
writer until section 13 experiments E0–E4 are recorded, because the NHL 07 writer's observed
"delete + insert pair" pattern has not been seen on NHL 08.

Out of scope for all three steps: NHL 07 PS3, the dynasty save, `rosterupdate` /
`currentrosters` / `defaultrosters` groups, and any base ↔ save synchronisation.

## 12. Validation plan (automated, then runtime)

Automated (synthetic or anchor-based, all read-only on anchors):
1. Classification: NHL 07 and NHL 08 Xbox 360 anchors → `Difflist` with the exact 31 / 30 mappings
   of the NHL 07 report and section 4; NHL 09 Xbox 360 anchor (19 tables, 19 shared, 31,245
   records) → `Snapshot`; a save with one undeclared table → `Unsupported` naming it; a declared
   companion with a tampered descriptor width → `Unsupported`.
2. Capacity arithmetic: Σ(capacity × record bytes) + schema-only == file − TDB base for the roster
   anchor and the dynasty anchor (two independent fixtures of the same rule).
3. Overlay identity: empty companions ⇒ effective roster == game DB for all 17 parents.
4. Overlay semantics on synthetic companions: delete by key, update by key with reduced set
   (`SREo` layout), insert append, unknown-key diagnostics, key collision diagnostics.
5. Writer round trip (step 3 only): empty anchor → one `CvSu` row → re-read → classification still
   `Difflist`, overlay shows the edit, file length unchanged, `checksum_a` and all inner CRCs
   valid; refusal at capacity + 1 and at `maxupdate = 0`.
6. `tdb-savedata`: a regression that `extract_db` on the NHL 08 dynasty anchor currently returns
   `NotTdb`, converted to a scan-based extraction when dynasty support is scheduled.

Runtime (Xenia or console, NHL 08 USA, section 13), recorded per the evidence rules. Nothing above
promotes NHL 08 beyond *implemented* / *automated-tested* until those runs exist; the existing
`baseline_inventory` exclusion stays until step 2 is accepted.

## 13. Unresolved questions and controlled in-game experiment matrix

Unresolved:
1. Populated roster companion layout (`data_alloc_type`, `max == cur`, `db_size` growth, file
   length, `checksum_a` scope) — inferred from the dynasty, unobserved for the roster.
2. Whether update rows carry the full update set with untouched fields at their stock values, or
   whether the game tolerates partial sets; whether rows are matched by key or by position.
3. How player creation is serialised: `PsvH` insert of a new bio (capacity 360) vs `CvSu` update of
   a pre-allocated `game_id 5000..5179` row (the NHL 08 schema makes the second at least as likely).
4. Roster `key` allocation for new roster rows, and whether `AtUj` + `ibGM` or a single `vozd` row
   expresses a team transfer.
5. Whether returning a field to its stock value removes the update row or leaves a no-op row.
6. Whether the game accepts a Studio-written difflist at all (ordering, CRCs, alloc type, trailer).
7. Which console produced the shared roster anchor (section 1); irrelevant to the schema, relevant
   to provenance records.
8. PS3 NHL 08: the PS3 roster `SYS-DATA` is the same file; whether the PS3 game writes the same
   container is unobserved.

One further input to the predictions: the comments in the untracked `roster_difflist.rs` state
that a game-written **NHL 07** roster replaced an edited roster row with a *delete + insert pair*
(`AtUj` + `ibGM`), switched those companions to `data_alloc_type = 6` and wrote them key-sorted —
i.e. NHL 07 did **not** use the `vozd` update table for a jersey edit. The populated NHL 07 save
behind that statement is not in the repository and was not examined here; it is quoted as a
claim, and it makes "update tables may go unused for roster rows" a live possibility for NHL 08.

Experiment matrix. Each experiment is a **separate save** from the unmodified roster state, copied
out immediately after saving; "expected" rows are predictions, to be replaced by observations.
Success criterion for every experiment: the file remains 1,238,480 bytes, `checksum_a`
recomputes, all inner CRCs recompute, and only the listed companions gain rows.

| # | Action in game | Establishes | Expected companions (prediction) | Expected unchanged |
|---|---|---|---|---|
| E0 | Open roster, save again without editing | whether an untouched save is byte-identical to the anchor or the game writes version/options rows; the baseline for diffing E1–E7 | none (byte-identical), or rows the game writes unconditionally | all 30 tables |
| E1 | Change one stock player's jersey number (or another bio field) | populated **update** layout (Q1, Q2); key-vs-position; whether all 31 update fields are written; whether NHL 08 uses update rows at all or the NHL 07-style delete + insert pair | `CvSu` 1 row (`game_id` of the player) and, for the roster row's `jersey`, either `vozd` 1 row **or** `AtUj` 1 + `ibGM` 1 (same `key` or a new one) | `PsvH`, all AI/equipment tables |
| E2 | Change one stock **skater** attribute | AI update companion; confirms `RosT` ↔ `yvSd` | `RosT` 1 row (`game_id`) | `kbtS`, `uyGF`, bio tables |
| E3 | Change one stock **goalie** attribute | goalie AI companion; confirms `uyGF` ↔ `yuHm` and the `position = 4` goalie set | `uyGF` 1 row | `PxTk`, `RosT` |
| E4a | Move one player from team A to team B | roster-row addressing (Q4): `vozd` by surrogate `key` vs delete + insert | `vozd` 1 row (same `key`, new `team`, line flags cleared) **or** `AtUj` 1 + `ibGM` 1 (new `key`) | `CvSu` unless the game also rewrites `bio.team` |
| E4b | Release one player to free agency, then sign a different free agent | free-agent companions | `mGOv` 1 (signed player's `playerindex`), `sAMA` 1 (released), `AtUj` 1 / `ibGM` 1 or `vozd` 2 | `LZbD`, `Frps` |
| E5a | Create one player (skater, custom team or FA) | create model (Q3) and custom-slot usage | either `PsvH` 1 + `LZbD` 1 (`table 1`) + `kbtS` 1 + `BlkH` 1 + `PPVt`/`sAMA`/`ibGM`, **or** `CvSu` 1 (`game_id 5000`) + `qWeu` 1 + `RosT`/`MSZu`/`PPVt` updates + `sAMA`/`ibGM` | goalie tables |
| E5b | Delete the player created in E5a | delete model; whether inserts are retracted or deletes appended | `Frps` 1 and/or `AtUj`/`mGOv` 1; insert rows from E5a possibly removed; **no** bio delete exists, so the bio row should persist as `CvSu`/`PsvH` | `aGgS` |
| E6 | Edit a field (as in E1), then set it back to its original value, then save | no-op handling (Q5) | `CvSu` 0 rows (retracted) **or** 1 row equal to stock | everything else |
| E7 | Load a Studio-written E1-equivalent file (only after E0–E6 are recorded) | runtime acceptance (Q6) | game shows the edit; a subsequent in-game save reproduces the Studio file or a canonical re-serialisation of it | — |

Diffing protocol for each result: `tdb_scan.py <save> 0x1c` for table counts and `alloc`;
`record_dump.py <save> 0x1c <meta> <companion> --as <parent>` to decode rows with parent field
names; `container_checks.py` for CRCs and the allocation arithmetic (the sum must now be computed
with schema-only = `db_size` − record data, as for the dynasty).

## 14. Reproduction, scripts and temporary files

All scripts are read-only Python 3, standard library only, in `scripts/`; `run_all.py` regenerates
every file in `output/` from the committed anchors and removes `scripts/__pycache__`.

```sh
python research/nhl08-roster-schema/scripts/run_all.py
python research/nhl08-roster-schema/scripts/tdb_scan.py "anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658" 0x1c --fields
python research/nhl08-roster-schema/scripts/difflist_map.py "<roster>" anchors/xbox360/nhl08/databases/nhlng.db anchors/xbox360/nhl08/databases/nhlng-meta.xml
python research/nhl08-roster-schema/scripts/record_dump.py anchors/xbox360/nhl08/databases/nhlng.db 0 anchors/xbox360/nhl08/databases/nhlng-meta.xml cPbu 0 3 --where lastname=Crosby
```

| Script | Purpose |
|---|---|
| `tdblib.py` | shared parser (file/table/descriptor layout per `tdb-core`), CRC-32/MPEG-2 and IEEE, four-scope CRC verifier, bit-field decoder, meta loader, role/capacity helpers |
| `tdb_scan.py` | `--scan` finds every embedded `DB` header; lists tables/fields of one TDB |
| `container_checks.py` | container words, `checksum_a`, inner CRC scopes, alloc/unknown words, zero tail, contiguity, capacity model |
| `difflist_map.py` | declared mapping, independent field-set cross-check, per-field descriptor comparison with the parent |
| `roster_group_meta.py` | every `rosters` field: type, depth, range, key, update flag, FK, bit offset in parent / insert / update / delete |
| `update_semantics.py` | `update`/`key` attribute census, update-table set identity, capacity-0 and role-coverage facts |
| `key_graph.py` | key uniqueness and FK coverage of the player/slot/roster/FA/team/AI/equipment graph |
| `exhibition_vs_stock.py` | exhibition/stock/league/custom table families: schema identity and byte identity |
| `save_directory_order.py` | save directory order vs meta document order and role adjacency |
| `crosstitle_compare.py` | NHL 07 vs NHL 08 containers, meta vocabulary, declared ids, `rosters` schema drift, shared save ids; re-checks the NHL 07 report's inferences against declarations |
| `dynasty_evidence.py` | dynasty container, preamble statistics, companion population, alloc types, allocation arithmetic, decoded samples |
| `record_dump.py` | decodes records with meta names; `--as <parent>` for companions; `--where field=value` |
| `run_all.py` | regenerates `output/` and writes `input_hashes.txt` |

Outputs (`output/`): `input_hashes.txt`, `scan_and_tables.txt`, `save_fields.txt`,
`container_checks_{roster,dynasty,gamedb}.txt`, `difflist_map_roster.txt`,
`difflist_map_nhl07_roster_declared.txt`, `roster_group_meta.txt`, `update_semantics.txt`,
`key_graph.txt`, `exhibition_vs_stock.txt`, `save_directory_order.txt`, `crosstitle_compare.txt`,
`dynasty_evidence.txt`, `record_trace.txt`, `record_trace_player.txt`.

Temporary files: none outside this folder. Nothing was written to the scratchpad, anchors, crates,
docs or any game/emulator directory; `scripts/__pycache__` is deleted by `run_all.py`. The
`research/` tree is untracked in git and was not staged.

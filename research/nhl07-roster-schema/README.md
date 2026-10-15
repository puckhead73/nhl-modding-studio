# NHL 07 Xbox 360 roster save vs `NHL07NG.db` — schema-mismatch diagnosis

**Status:** research only, 2026-10-05. Nothing outside `research/nhl07-roster-schema/` was modified.
No application code, anchor, manifest, game install or save was changed; roster mirroring stays
disabled and the baseline approvals are untouched.

## 0. Plain-language summary

The NHL 07 roster save is **not an incompatible or foreign database**. It is a **difflist (change
list) database** generated from the game's own schema: for every game table in the `rosters` save
group it carries up to three companion tables — an *insert* table (full records), a *delete* table
(key only) and an *update* table (key + the fields flagged `update="True"`). Those companion tables
have their **own 4-character ids**, which is why Studio sees no shared table ids. The field ids are
exactly the game's: all 179 field ids in the save exist in `NHL07NG.db`.

The fresh save the user produced (byte-identical to the anchor) contains **zero records** in all 31
tables: it is an *empty* change list followed by 1,588,384 zero bytes. That tail is exactly the sum
of the declared difflist capacities (`maxinsert` / `maxdelete` / `maxupdate` from
`NHL07NG-meta.xml`, with `-1` meaning the parent's `max_records`) times each table's record length.
The file is therefore a fixed-size pre-allocation of the maximum change list.

Studio refuses the save because `workspace.rs::analyze_bytes` requires the roster's inner TDB to
share canonical **table ids** with the game database and to have **equal `current_records`** per
shared table (a record-index-aligned snapshot model, verified on NHL 14 PS3). The NHL 08 Xbox 360
roster save uses the same difflist model; NHL 09 is the first Xbox 360 title whose roster save is a
snapshot with the game's own table ids. The warning text ("belongs to a different database format")
is wrong in substance: the format is the same TDB v8 big-endian container, the save model differs.

Safe roster editing for NHL 07 needs a difflist overlay reader/writer, not table-id alignment. The
mapping save-table → parent-table is established for all 31 tables (20 by the meta's own
`insertshortname`/`deleteshortname` declarations, 12 by exact `update`+key field-set identity, with
the two schema-identical AI pairs resolved by cross-title id stability plus directory adjacency).

## 1. Inputs and integrity

| Item | Path | Size | SHA-256 |
|---|---|---|---|
| Fresh Xenia save | `H:\Emulators\Xenia Manager\Emulators\Xenia Canary\content\E03000001470E6EC\454107F3\00000001\ROSTER1\ROSTER1` | 1,598,888 | `40a06d53…dd2223d` |
| Bundled anchor | `anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1` | 1,598,888 | `40a06d53…dd2223d` (identical) |
| Game DB | `anchors/xbox360/nhl07/databases/NHL07NG.db` | 4,189,776 | `67a00bea…fcdb8ad3` |
| Game meta | `anchors/xbox360/nhl07/databases/NHL07NG-meta.xml` | 269,813 | `c9c833dc…33d98520` |

The PS3 `anchors/ps3/nhl07/databases/` copies of the DB and meta are byte-identical to the Xbox 360
ones (same hashes). The Xenia content folder also holds `USER PROFILE1` (30,128 bytes) and STFS
`.header` files; these were only listed, not read.

Container checks (`output/container_checks_nhl07.txt`):

| Offset | Value | Meaning | Verified |
|---|---|---|---|
| `0x00` | `Rosters\0…` | gen6 roster magic → `SaveType::Roster` | yes |
| `0x10` | `2F65C1BE` | `checksum_a` = CRC-32/IEEE over `[0x1C..EOF]` | recomputed, matches |
| `0x14` | `3` | container version word → `ContainerVersion::RawV3` | yes |
| `0x18` | `0` | unknown word (also 0 on NHL 08/09 rosters) | — |
| `0x1C` | `DB 00 08 01 …` | inner TDB, version 8, big-endian, `db_size=0x28EC` (10,476), 31 tables | header CRC `4054DEA1` recomputed, matches |
| `0x34` | directory | 31 × 8-byte `(id, offset)` entries | parsed |
| `0x12C`–`0x2908` | table blocks | 40-byte headers + 16-byte descriptors, **no record data** (`max_records = current_records = 0` everywhere) | parsed |
| `0x2908`–`0x186428` | `00 …` | 1,588,384 zero bytes | all-zero confirmed |

The game DB header CRC also recomputes (`D9A12BDB`).

## 2. What Studio does today (code references)

- **Extraction is correct.** `crates/tdb-savedata/src/header.rs:120-130` resolves version word 3 to
  `RawV3` (header 0x1C); `crates/tdb-savedata/src/extract.rs:22-37` trims the payload to the inner
  `db_size`, which correctly yields the 10,476-byte schema and drops the zero tail. No second
  database section exists in the file (`output/scan_and_tables.txt`, `--scan` finds one `DB` header).
- **The refusal.** `crates/studio-domain/src/workspace.rs:2939-2970` (`analyze_bytes`) builds the set
  of canonical table ids of `nhlng.db`, intersects it with the roster's, requires equal
  `current_records` per shared table, and returns `DomainError::Validation("This roster save shares
  no database tables …different database format…")` when the intersection is empty. The comment
  at `workspace.rs:2958-2960` records the NHL 07 fixture as the trigger.
- **The model assumption.** `workspace.rs:15-19` documents the roster save as "a verified
  record-index-aligned subset of nhlng.db … verified for nhl14 PS3 only";
  `crates/studio-api/src/registry.rs:341-343` states "PS3/Xbox/Recomp mirror: there nhlng.db *is*
  the roster (record-index aligned)". The mirror gate at `workspace.rs:1241-1244` only fans a
  `set_field` out when `aligned && shared_tables.contains(studio_table)`.
- **The hard-coded exclusion.** `crates/studio-api/src/bin/baseline_inventory.rs:124` (NHL 07) and
  `:101-107` + `:127` (`incompatible_roster`, NHL 08) record the saves as incompatible; the test
  `crates/studio-api/tests/nhl07_source_qa.rs:218-232` pins the refusal text.
- **Discovery** (`crates/studio-domain/src/discovery.rs:955-975`) classifies the save as kind
  `roster`, writable (RawV3 is writeable), so the registry *does* try to attach it as a mirror.
- **Metadata ingestion** (`crates/tdb-defs/src/lib.rs:342-390`) reads `name`, `type`, `depth`,
  range, `key`, `update`, `fkparenttable` and ignores `insertshortname`, `deleteshortname`,
  `maxinsert`, `maxdelete`, `maxupdate`, `savegroups`. `docs/formats/metadata-xml.md:17,31` mentions
  those attributes exist but not their semantics.

## 3. Findings

### 3.1 Confirmed facts (reproducible from the anchors)

1. **Same binary format, same endianness.** Both files are TDB version 8, big-endian, 40-byte table
   headers, 16-byte descriptors; `tdb-core`'s layout parses both without error. No endianness or
   id-canonicalisation mistake is involved: the ids genuinely differ.
2. **Zero shared table ids, 179/179 shared field ids.** `output/fieldset_match_save_vs_game.txt`.
3. **20 of 31 save tables are declared in the game's own meta** as `insertshortname` /
   `deleteshortname` of a table whose `savegroups` contains `rosters`
   (`output/difflist_map_nhl07.txt`). Delete tables carry exactly the parent's key field; insert
   tables carry the parent's full field list with identical descriptor layout.
4. **The other 12 are update tables**: their field set equals, exactly, `{fields with
   update="True"} ∪ {key}` of a `rosters`-group parent (`output/update_table_match_nhl07.txt`).
   Parents without any `update="True"` field (exhibitionfreeagents, exhibitionteams,
   exhibitionplayers, exhibitionarena) have no update table — matching the save. Three player-bio
   update tables have *different* field counts (44/43/42) that match the three bio parents'
   update-sets one-to-one (`HHLk`→customplayerbiotable 43+1, `CvSu`→exhibitionplayerbiotable
   42+1, `MVOx`→exhibitiondownloadplayerbio 41+1).
5. **Capacity model.** `output/container_checks_nhl07.txt`: Σ(capacity × record_length) over the 31
   tables = **1,588,384** = the zero tail, and 10,476 + 1,588,384 = 1,598,860 = file − 0x1C. The
   file is a fixed pre-allocation of the maximum change list; a populated save never has to grow.
6. **Save directory order = meta document order of the parents**, with (insert, delete, update)
   adjacent per parent (`output/save_directory_order_nhl07.txt`). The game DB's own directory is
   *not* in meta order, so the save was produced by a writer that walks the schema definition.
7. **NHL 08 Xbox 360 uses the same model** (30 tables, 14,780-byte empty schema, 1,223,672 zero
   bytes, 14 declared + 16 update tables all resolved uniquely — `output/difflist_map_nhl08.txt`,
   `output/update_table_match_nhl08.txt`). **NHL 09** switches to a 1.57 MB snapshot whose 19 tables
   use the game's ids with records populated (`output/scan_and_tables.txt`).
8. **The NHL 07 dynasty save is a populated difflist of the `dynasty` group** (77 tables, 690 KB,
   e.g. `fcdR` = leagueplayerbiotable insert with 941 full records: "FRANCOIS BEAUCHEMIN" game_id
   324 …; `ioPQ` = leaguefreeagents insert rows `TWSX=867…`). Populated difflist tables carry
   `data_alloc_type = 6` and `max_records == current_records`
   (`output/dynasty_difflist_evidence_nhl07.txt`).
9. **Cross-title id stability.** For the 37 parents that declare insert/delete ids in both the NHL 07
   and NHL 08 metas, all 37 pairs are identical, and the update tables `CvSu/RosT/uyGF/vozd/mASv`
   resolve to the same parents in both titles (`output/crosstitle_ids.txt`). The difflist ids are a
   stable function of the parent id across EA's gen6/early-gen7 tooling.
10. **Game-DB roster graph** (`output/key_graph_nhl07.txt`, `output/record_trace_nhl07.txt`):
    - `exhibitionplayerbiotable` (`cPbu`, 2,250 rows, 508 B, records at `0x89A70`) is
      **byte-identical** to `stockplayerbiotable` (`aUhv`); likewise exhibition↔stock/default for
      the roster table, skater/goalie AI, free agents, download bios and roster version
      (`output/exhibition_vs_stock_nhl07.txt`). The `exhibition*` tables are the working copies the
      difflist overlays.
    - `exhibitionplayers` (`caBZ`, 2,790 rows, records at `0x1A0BA0`): `(index, table, playerid)`
      is the player-slot registry. `index` is unique (0..3340, sparse); `playerid` references
      `game_id` in the bio tables: rows 0–199 point at the 200 `exhibitiondownloadplayerbio`
      placeholders ("Roster PlaceHolder1…"), the rest at stock bios. 277 `playerid`s are
      duplicated: a second slot for the same bio on an international team (Canada 31, USA 30,
      Sweden 27, …). `table` is 0 on every shipped row; the meta documents it as the stock/custom
      discriminator (custom bios live in `customplayerbiotable`, `MqdK`, 0 rows shipped,
      `game_id` 8-bit).
    - `exhibitionrostertable` (`ulGe`, 2,516 rows, 16 B, records at `0x1A3BC0`):
      `playerindex` → `caBZ.index` (all resolve, unique), `team` → `exhibitionteams.index`
      (0..92), plus roster status, captaincy and 60-odd 1-bit line flags. `exhibitionfreeagents`
      (`QEoV`, 74 rows) holds `playerindex` values disjoint from the roster; roster ∪ FA covers
      2,590 of the 2,790 slots (the 200 placeholders are unassigned).
    - `exhibitionteams` (`ttOk`, 93 rows) is a thin `(index, teamid, table, division)` selector onto
      `stockteamtable` (`UzqH`, 93 rows, 596 B; e.g. index 89 = "Adler Mannheim", 92 = "HC Davos").
      `exhibitionarena` (`OEtS`) and `exhibitionteamstrategies` (`oYeE`) hang off `exhibitionteams`.
    - Trace: `cPbu[0]` = Vitalij Aab, `game_id 200`, team 89, jersey 79 ↔ `caBZ[200]` = (index 200,
      playerid 200) ↔ `ulGe[0]` = playerindex 200, team 89 (Adler Mannheim), rosterstatus 1.
    - `rosterversiontable` (`bZvB`) holds one row, `version='05Aug05'`.

### 3.2 Full table mapping

Roles from the meta (`insert`/`delete`) or from the update-field-set identity (`update`). Capacity:
`maxinsert`, `maxdelete`, `maxupdate`; `-1` = parent `max_records`.

| Save table | Role | Parent (raw id) | Parent name | Rec B | Capacity | Evidence |
|---|---|---|---|---|---|---|
| `DZfz` | update | `dDrd` | exhibitiondownloadgoalieai | 16 | 50 | field set (3 candidates) + §3.3 |
| `YvhR` | update | `Raym` | exhibitiondownloadskaterai | 20 | 150 | field set (3 candidates) + §3.3 |
| `znPC` | insert | `oYeE` | exhibitionteamstrategies | 8 | 30 | meta |
| `JhBt` | delete | `oYeE` | exhibitionteamstrategies | 4 | 93 | meta |
| `mASv` | update | `oYeE` | exhibitionteamstrategies | 8 | 93 | field set (unique) |
| `eUOR` | insert | `MqdK` | customplayerbiotable | 508 | 180 | meta |
| `oCMe` | delete | `MqdK` | customplayerbiotable | 4 | 0 (parent empty) | meta |
| `HHLk` | update | `MqdK` | customplayerbiotable | 504 | 180 | field set (unique) |
| `MVOx` | update | `EhOn` | exhibitiondownloadplayerbio | 504 | 200 | field set (unique) |
| `KboR` | insert | `YzeG` | customgoalieai | 16 | 30 | meta |
| `Exae` | delete | `YzeG` | customgoalieai | 4 | 0 | meta |
| `rcpY` | update | `YzeG` | customgoalieai | 16 | 30 | field set (3 candidates) + §3.3 |
| `sAMA` | insert | `QEoV` | exhibitionfreeagents | 4 | 200 | meta |
| `mGOv` | delete | `QEoV` | exhibitionfreeagents | 4 | 74 | meta |
| `fyFF` | insert | `ttOk` | exhibitionteams | 4 | 30 | meta |
| `dULq` | delete | `ttOk` | exhibitionteams | 4 | 93 | meta |
| `LZbD` | insert | `caBZ` | exhibitionplayers | 4 | 180 | meta |
| `Frps` | delete | `caBZ` | exhibitionplayers | 4 | 2790 | meta |
| `ibGM` | insert | `ulGe` | exhibitionrostertable | 16 | 3200 | meta |
| `AtUj` | delete | `ulGe` | exhibitionrostertable | 4 | 2516 | meta |
| `vozd` | update | `ulGe` | exhibitionrostertable | 16 | 2516 | field set (unique) |
| `CvSu` | update | `cPbu` | exhibitionplayerbiotable | 504 | 2250 | field set (unique) |
| `tMYP` | insert | `bZvB` | rosterversiontable | 68 | 1 | meta |
| `nGCg` | delete | `bZvB` | rosterversiontable | 4 | 1 | meta |
| `Iblm` | update | `bZvB` | rosterversiontable | 68 | 1 | field set (unique) |
| `CTnc` | insert | `OEtS` | exhibitionarena | 4 | 30 | meta (no delete declared) |
| `RosT` | update | `yvSd` | exhibitionskaterai | 20 | 2015 | field set + NHL 08 same id→`yvSd` |
| `uyGF` | update | `yuHm` | exhibitiongoalieai | 16 | 235 | field set + NHL 08 same id→`yuHm` |
| `KERl` | insert | `oDHy` | customplayerai | 20 | 150 | meta |
| `wKzK` | delete | `oDHy` | customplayerai | 4 | 0 | meta |
| `zrsU` | update | `oDHy` | customplayerai | 20 | 150 | field set (3 candidates) + §3.3 |

### 3.3 Hypotheses (well supported, not yet game-verified)

- **H1 — difflist semantics.** Insert rows are whole new records; delete rows name a parent key;
  update rows carry a key plus new values for the updatable fields. Supported by the meta
  attribute names, the key-only delete layout, the exact update-set identity, and the populated
  dynasty save (insert tables hold full named records). *Not yet observed*: a populated NHL 07
  **roster** update or delete table, and whether the game applies update rows by `game_id` /
  `index` key (as the layout implies) rather than by position.
- **H2 — disambiguation of the schema-identical AI tables.** Goalie AI: `{DZfz, rcpY, uyGF}` ↔
  `{dDrd, YzeG, yuHm}`; skater AI: `{YvhR, RosT, zrsU}` ↔ `{Raym, oDHy, yvSd}`. `uyGF→yuHm` and
  `RosT→yvSd` are fixed by the NHL 08 save (same ids, unique there). `rcpY` and `zrsU` sit
  directly after their parents' declared insert/delete pair, the pattern every one of the other
  four resolvable triples follows; that leaves `DZfz→dDrd` and `YvhR→Raym`. Directory order also
  places parents in meta document order under this assignment. A runtime experiment (§6) settles it.
- **H3 — populated layout.** When the game writes a non-empty roster difflist it reserialises the
  inner TDB with `max_records = current_records = n`, `data_alloc_type = 6`, record data inline
  after each table's descriptors, and `db_size` grown accordingly, within the fixed 1,598,888-byte
  allocation; the container `checksum_a` is recomputed over `[0x1C..EOF]` including the zero tail.
  Supported by the dynasty save's populated tables and the capacity arithmetic; unverified for the
  roster slot because no populated NHL 07 roster save exists in the repository.
- **H4 — `rosterupdate` / `currentrosters` groups** are EA's downloadable-roster and
  current-roster save contexts; `exhibitiondownload*` + the 200 "Roster PlaceHolder" bios are the
  roster-update slots. Not needed for editing; noted for completeness.

### 3.4 Side observation (out of scope, reported only)

`crates/tdb-defs/src/lib.rs:376` treats a field as updatable unless `update="False"`. No committed
meta (NHL 07, 08, 14 PS3, 19 PS4) ever writes `update="False"`; the NHL 07 meta marks 954 fields
`update="True"` and leaves 859 unmarked, and the game's update difflists include **only** the
marked fields. For NHL 07/08 "absent" therefore means non-updatable, and the current reading
over-approximates editability. Any difflist writer must use the exact marked set.

## 4. Answers to the questions

1. **Correct payload?** Yes. The container holds exactly one TDB at `0x1C`; `extract_db` returns
   the whole 10,476-byte schema; the remainder is zero padding sized by the capacity model. No
   other database section or embedded structure exists.
2. **Endianness / ids / canonicalisation?** All interpreted correctly. Both files are big-endian
   TDB v8; the save's table ids are genuinely different because they name difflist companions.
3. **Different schemas?** The save's schema is *derived from* the game's: 31 companion tables of 16
   parents in the `rosters` save group, mapped above by the meta's declarations and by exact
   field-set identity (never by name similarity).
4. **Structures and stable ids.** Players: `exhibitionplayerbiotable.game_id` (13-bit, stock
   200..3353) / `customplayerbiotable.game_id` (8-bit); slot registry `exhibitionplayers.index` →
   `(table, playerid)`; roster assignment `exhibitionrostertable.(playerindex, team, rosterstatus,
   lines)`; free agents `exhibitionfreeagents.playerindex`; teams `exhibitionteams.index/teamid` →
   `stockteamtable`; editable attributes = the `update="True"` fields of each parent (bio: 42 of
   46; roster row: 69 of 70; skater AI 26 of 27; goalie AI 22 of 23; strategies 10 of 11).
5. **Why zero shared tables?** `analyze_bytes` matches **table ids**, assumes a snapshot model and
   requires equal row counts. Difflist companions never share ids, and the empty save has no rows.
   The assumption is title-specific (true for NHL 09–Legacy and NHL 14 PS3, false for NHL 07/08).
6. **Implementation work.** See §5.

## 5. Proposed implementation and validation plan (not implemented)

### 5.1 Read side — recognise and overlay a difflist roster

1. **Metadata.** Extend the `tdb-defs` meta reader to retain, per table: `insertshortname`,
   `deleteshortname`, `maxinsert`, `maxdelete`, `maxupdate`, `savegroups`, and per field the exact
   `update="True"` flag (see §3.4). No behaviour change for titles that lack these attributes.
2. **Save-model classification** (studio-domain, beside `analyze_bytes`): given the roster's inner
   TDB and the game DB, classify as
   - `Snapshot` — shares table ids (current behaviour), or
   - `Difflist` — zero shared ids **and** every save table resolves to a `rosters`-group parent by
     (a) declared insert/delete id, or (b) exact update+key field-set identity, with ties broken by
     directory adjacency (§3.3 H2) and recorded as "resolved by adjacency" in the diagnostics.
   Unresolved tables → refuse with a precise message naming the table. Replace the "different
   database format" wording.
3. **Effective roster view** = parent tables of `NHL07NG.db` (the `exhibition*` working copies)
   with the difflist applied: delete keys removed, update rows merged by key, insert rows appended.
   On the shipped save this is the game DB unchanged, so existing NHL 07 views keep working.
4. **Baseline inventory / approvals.** Keep the NHL 07 and NHL 08 roster saves excluded until the
   writer below is validated; change the recorded reason to "difflist save model, overlay editing
   not yet implemented".

### 5.2 Write side — safe roster editing (Standard mode, target-native)

1. **Edit capture.** For a scalar edit on a stock row: append/merge a row in the parent's *update*
   table keyed by the parent key (`game_id`, `index`, `playerindex`), carrying all update fields
   (current values for untouched ones). For membership/line changes: update rows in `vozd`
   (`exhibitionrostertable`), plus insert/delete in `sAMA`/`mGOv` (free agents) and `ibGM`/`AtUj`.
   For a created player: insert rows in `eUOR` (custom bio, 8-bit `game_id`), `LZbD`
   (`exhibitionplayers` with `table=1`), `KERl`/`KboR` (AI), `ibGM` or `sAMA`.
2. **Serialisation.** Grow each touched table from `max=0` (`tdb-edit` already has
   `grow_table_max`/`append_record`), write `data_alloc_type` per H3, recompute table and file CRCs
   (`tdb-core::recompute_crcs`), keep the file at 1,598,888 bytes (schema + capacity never exceeds
   it), and recompute `checksum_a` with `checksum::recompute_a_only` (RawV3 path already exists in
   `tdb-savedata::pack`). Respect `maxinsert`/`maxupdate`/`maxdelete` as hard capacity limits.
3. **Transaction.** Route through the existing single pending-changes review; a difflist roster is
   a second write target of one coordinated transaction, same recovery model.

### 5.3 Game-database ↔ save mirroring

Record-index mirroring is **not applicable**. The relationship is base + overlay, keyed by stable
ids, and both directions are expressible:

- *Save-only editing* (the common Standard-mode path): the game DB is read-only base; every edit
  becomes difflist rows. Nothing to mirror.
- *Game-DB editing* (ISO/loose-file modding): edit the `exhibition*` tables and keep their
  byte-identical `stock*`/`default*` twins in sync (§3.1 fact 10). If a roster save is also
  attached, project the same change as an update/insert/delete row so the overlay stays consistent;
  conversely a difflist row can be folded into the base. Both projections key on `game_id` /
  `index`, never on record position.

### 5.4 Validation (automated, then game-verified)

- Automated: (a) classification test on the three Xbox 360 fixtures (07/08 → Difflist, 09 →
  Snapshot); (b) overlay identity test (empty difflist ⇒ effective roster == game DB); (c) writer
  round-trip: empty save → one update row → re-read → effective roster shows the edit; byte length
  unchanged; `checksum_a` and inner CRCs valid; (d) capacity refusal at `maxinsert+1`.
- Game-verified (Xenia, NHL 07 USA): the experiments in §6, recorded per the evidence rules;
  nothing above promotes the target beyond *implemented* until those runs exist.

## 6. Unresolved questions and runtime experiments

1. **Populated roster difflist layout (H3).** Needed: in-game, change one stock player's jersey
   number, save the roster, copy the file. Expect one row in `CvSu` (key `game_id` + 42 fields),
   `max=cur=1`, `data_alloc_type` observed, `db_size` grown, file still 1,598,888 bytes. Also
   answers whether untouched update fields carry the stock values.
2. **AI-table disambiguation (H2).** Edit one stock goalie's attributes and one stock skater's
   attributes; expect rows in `uyGF` and `RosT`. Then (if the game exposes it) edit a
   downloaded/placeholder player to see `DZfz`/`YvhR`; otherwise `DZfz→dDrd`, `YvhR→Raym` stays an
   inference.
3. **Create / delete semantics.** Create a player in-game: expect `eUOR`, `LZbD` (`table=1`,
   `playerid` = new 8-bit custom `game_id`), AI inserts, and `ibGM` or `sAMA`. Delete that player:
   expect `oCMe`/`Frps`/`AtUj` key rows, or removal of the insert rows — unknown which.
4. **Does the game accept a Studio-written difflist?** Load a file produced by the §5.2 writer.
5. **Version row.** Whether the game writes `tMYP`/`Iblm` (rosterversiontable) on every save and
   what string it uses.
6. **PS3 NHL 07/08.** Not examined (no PS3 roster save in `anchors/`; PS3 NHL 07 is research-only).
7. **NHL 08 Xbox 360** is covered by the same model (fact 7) but its own runtime experiments are
   separate work; nothing here changes its approval.

## 7. Reproduction

All scripts are read-only Python 3 (standard library only). From the repository root:

```sh
python research/nhl07-roster-schema/scripts/run_all.py        # regenerates every file in output/
python research/nhl07-roster-schema/scripts/tdb_scan.py anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1 --scan
python research/nhl07-roster-schema/scripts/tdb_scan.py anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1 0x1c --fields
python research/nhl07-roster-schema/scripts/difflist_map.py anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1 0x1c anchors/xbox360/nhl07/databases/NHL07NG.db anchors/xbox360/nhl07/databases/NHL07NG-meta.xml
python research/nhl07-roster-schema/scripts/record_dump.py anchors/xbox360/nhl07/databases/NHL07NG.db 0 anchors/xbox360/nhl07/databases/NHL07NG-meta.xml cPbu 0 4 --only game_id,firstname,lastname,team
```

| Script | Purpose |
|---|---|
| `tdb_scan.py` | TDB walker: `--scan` finds every embedded `DB` header; lists tables/fields of one |
| `container_checks.py` | container checksum, inner CRC, zero tail, capacity model |
| `schema_compare.py` | pairs tables by descriptor signature, ignoring ids |
| `fieldset_match.py` | pairs tables by exact field-id set, with meta names |
| `difflist_map.py` | maps save tables via `insertshortname`/`deleteshortname`; lists save groups |
| `update_table_match.py` | tests the update+key field-set identity for undeclared tables |
| `roster_group_meta.py` | field metadata (key / update / FK / offsets) of all `rosters`-group tables |
| `key_graph.py` | verifies the player → slot → roster/free-agent → team graph by record contents |
| `record_dump.py` | decodes records with meta names (`--as <parent>` for difflist tables) |
| `crosstitle_ids.py` | NHL 07 vs NHL 08 difflist-id stability |
| `save_directory_order.py` | save directory order vs meta document order |
| `exhibition_vs_stock.py` | byte-identity of `exhibition*` vs `stock*`/`default*` tables |

Outputs live in `output/` (one text file per script run; `scan_and_tables.txt` holds the raw table
listings of all five files examined).

## 8. Temporary files

None outside this folder. `scripts/__pycache__` is removed after each run. Nothing was written to
the scratchpad, anchors, Xenia content or any crate.

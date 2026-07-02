# Anchor DB Capacity & Editable-Table Inventory

> **Scope of this audit.** (1) *Capacity scope* — the record-count headroom in the anchor TDB
> databases, with the **PS3 family** as the focus ("S3 capacity"), and (2) a per-game inventory of
> every editable table across all anchor `.db` files, with human-readable naming and an assessment of
> expansion potential. Generated from the committed anchors under `anchors/` (see method below);
> exhaustive per-table data is in [`data/anchor-tables-full.csv`](data/anchor-tables-full.csv)
> (3,173 rows — every table in every anchor DB).
>
> Companion audits: [anchors-manifest.md](anchors-manifest.md) (file inventory),
> [roster-pools.md](roster-pools.md) (pool model), [nhlng-schema.md](nhlng-schema.md),
> [roster-relationships.md](roster-relationships.md). This doc adds the **capacity/expansion** lens.

## 1. Method

Each anchor `.db` is a **TDB v6** container. For every table the 40-byte header stores
`max_records` (u16 @0x14) and `current_records` (u16 @0x16); table/field names come from the
sibling `*-meta.xml` (joined on the on-disk 4-char id — the community/"canonical" id shown here is the
byte-reversed form on big-endian PS3/Xbox360, as-is on little-endian PS4). Parsed with a standalone
reader mirroring `tdb-core` (`TdbFile::parse`); 100 % of metadata-backed tables joined to their meta.
Where an anchor ships no meta (PS3 NHL 10; all auxiliary `.db`s), names are recovered from the
**shared-id map** — every auxiliary table's id matches a named table in the main DB.

Two structural facts make the report tractable:

- **PS3 and Xbox360 anchors are byte-identical** per game — same table set, same record counts, same
  record lengths (verified for all of NHL 10–15 + Legacy). They diverge only in the *save-file*
  container, never in the DB. GEN7 tables below are therefore labelled **"PS3 = Xbox360."**
- **Every auxiliary `.db` is a subset projection** of the main game DB (identical ids/schema, a
  different row population) — the same relationship the roster save has to `nhlng.db`.

## 2. The capacity model — what "capacity" actually means here

There are **three** distinct ceilings; conflating them is the usual mistake.

| Ceiling | Where it lives | Value | Meaning |
|---|---|---|---|
| **Physical slot alloc** | table header `max_records` | **GEN7: == `current_records`** (packed); **PS4: ≥ `current_records`** (pre-allocated slack) | Slots physically reserved for the block right now. |
| **u16 record wall** | `max_records` is a u16 | **65,535** | Hard cap on rows per table; `tdb-core` returns `CapacityExhausted` here. |
| **Key-field id space** | meta `depth`/`rangehigh` of the key | e.g. player index **14-bit → 16,383** (GEN7), **16-bit → 65,535** (PS4) | The real practical cap — you run out of *ids* long before the u16 wall. |

Two consequences that drive everything below:

1. **The two platform generations allocate slots differently:**
   - **GEN7 (PS3 = Xbox360) anchors are packed — `max_records == current_records` for 100 % of
     tables, free slots = 0.** Any insert requires **growing the block** (reallocation + shifting every
     following table + CRC/size fix-ups), never just filling a spare slot. This is exactly the "blocks
     are full → growth needed" condition the [towx-blocks](towx-blocks.md) / roster-pool work flagged
     (Spike S3 / Phase 4.4 capacity normalization).
   - **PS4 (GEN8) anchors pre-allocate slack — `max_records > current_records` for the majority of
     tables** (NHL 21: 83 of 115). Empty `league*` runtime tables reserve their *entire* insert budget
     as physical slots (e.g. `leaguetradeblock` `max_records`=30,000 with 0 rows); populated
     `exhibition*` tables carry spare rows (e.g. NHL 21 `exhibitionrostertable` 9,090 rows in 16,576
     slots — 7,486 free). PS4 inserts fill slack **without** block growth until the reservation is
     exhausted.
2. **`maxinsert` is a per-transaction diff budget, not a hard table-size cap.** Several *populated*
   tables already hold more rows than their `maxinsert` (GEN7 `exhibitionplayers` `maxinsert`=3000 but
   5,593–6,087 rows; PS4 =4,800 but 9,281). Read it as "max rows one save-diff may add." On PS4 it
   *also* doubles as the pre-reserved slot count for the empty runtime tables (consequence 1). Absolute
   size is always bounded by the u16 wall and the key id space above.

## 3. PS3 capacity scope (the focus)

### 3.1 DB-level totals — PS3 `nhlng.db` (= Xbox360)

| Game | tables | records | Σmax_records | free slots | packed? |
|---|--:|--:|--:|--:|:--:|
| NHL 10 | 86 | 51,971 | 51,971 | 0 | yes |
| NHL 11 | 131 | 74,479 | 74,479 | 0 | yes |
| NHL 12 | 133 | 77,411 | 77,411 | 0 | yes |
| NHL 13 | 136 | 79,628 | 79,628 | 0 | yes |
| NHL 14 | 134 | 85,401 | 85,401 | 0 | yes |
| NHL 15 | 134 | 88,571 | 88,571 | 0 | yes |
| NHL Legacy | 134 | 88,525 | 88,525 | 0 | yes |

Every DB is fully packed (free slots = 0): expansion is always block-growth, never slot-fill.

### 3.2 Roster occupancy vs. the id ceiling (the number that matters for expansion)

| Game | players (exhibitionplayers) | roster slots (exhibitionrostertable) | teams (exhibitionteams) | goalies (exhibitiongoalieequipment) |
|---|--:|--:|--:|--:|
| NHL 10 | 4188 | 3719 | 182 | 434 |
| NHL 11 | 5593 | 5228 | 242 | 550 |
| NHL 12 | 5779 | 5349 | 243 | 574 |
| NHL 13 | 5874 | 5446 | 248 | 537 |
| NHL 14 | 5827 | 5353 | 246 | 523 |
| NHL 15 | 5942 | 5511 | 246 | 556 |
| NHL Legacy | 6087 | 5532 | 252 | 566 |

- **Players** are keyed by a **14-bit index (`ZBac`) → hard ceiling 16,383.** Legacy ships 6,087, so
  ~**10,000 additional player ids** are available before the id space is exhausted (well under the
  65,535 u16 wall). Parallel per-player tables (`ubPc` bio, `xmja`/`dSvy` skater equip/AI) share that
  14-bit index and must grow in lock-step.
- **Roster slots** (`eGlu`) and **skater-equipment ids** are also 14-bit (16,383). **Free-agent index**
  (`VoEQ`) 14-bit.
- **Teams** (`kOtt`) are keyed by an **8-bit index → ceiling 255** (Legacy ships 252 — **only 3 free
  team ids**). The dynasty `leagueteams` key is **7-bit → 127**. Team expansion is the tightest
  constraint in the GEN7 DBs.
- **Player heads / appearance** (`stockplayerheads` `headid`) are **16-bit → 65,535** — effectively
  unconstrained.

### 3.3 Where the editable bytes are — the `exhibition*` roster complex

The live, user-editable roster is the **`exhibition*` family** (populated in every anchor). A "player"
is an entity joined across parallel tables by its 14-bit index: `exhibitionplayers` (`ZBac`, core),
`exhibitionplayerbiotable` (`ubPc`, bio/appearance/ratings), `exhibitionskaterequipment` (`xmja`),
`exhibitionskaterai` (`dSvy`); goalies add `exhibitiongoalieequipment`/`ai`; `exhibitionrostertable`
(`eGlu`) holds team→player slots; `exhibitionteams` (`kOtt`) + `exhibitionteamstrategies` the clubs;
`exhibitionfreeagents` (`VoEQ`) the unplaced pool. The `stock*` family is **shipped reference data**
(appearance: heads, helmet placement, jerseys, colors, retired numbers; plus read-only career/stat
baselines). The `league*` family is **dynasty / Be-A-GM runtime state — empty (0 rows) in every
anchor**, filled only when a mode is started (which is why it carries the giant `maxinsert` budgets up
to 65,534). Full per-game tables in §6.

### 3.4 PS4 companion (GEN8) — for contrast

DB-level totals, PS4 `nhl_game.db`:

| Game | tables | records | Σmax_records | free slots | packed? |
|---|--:|--:|--:|--:|:--:|
| NHL 15 | 148 | 90,227 | 469,364 | 379137 | NO |
| NHL 16 | 135 | 94,209 | 463,089 | 368880 | NO |
| NHL 17 | 118 | 101,281 | 480,035 | 378754 | NO |
| NHL 18 | 117 | 100,238 | 567,677 | 467439 | NO |
| NHL 19 | 111 | 111,145 | 263,626 | 152481 | NO |
| NHL 20 | 111 | 111,145 | 263,626 | 152481 | NO |
| NHL 21 | 115 | 129,056 | 279,860 | 150804 | NO |

Roster occupancy vs. the wider PS4 id space:

| Game | players | roster slots | teams | goalies |
|---|--:|--:|--:|--:|
| NHL 15 | 5984 | 5523 | 246 | 559 |
| NHL 16 | 6087 | 5532 | 252 | 566 |
| NHL 17 | 6701 | 6430 | 256 | 601 |
| NHL 18 | 7735 | 7373 | 296 | 714 |
| NHL 19 | 8581 | 8328 | 419 | 747 |
| NHL 20 | 8581 | 8328 | 419 | 747 |
| NHL 21 | 9281 | 9090 | 433 | 749 |

## 4. File families & roles

| DB file | Platforms | Role | Editable? |
|---|---|---|---|
| `nhlng.db` | PS3/Xbox360 | main game + roster DB | **yes** (roster surface) |
| `nhl_game.db` | PS4 | main game + roster DB (LE analogue of `nhlng.db`) | **yes** |
| `draft_roster.db` | PS3/Xbox360 NHL 11+ | draft-class roster (10-table `exhibition*` projection) | yes |
| `dynamic_ratings.db` | PS3/Xbox360 NHL 14+ | live rating overlay — `dynamicskaterai`/`dynamicgoalieai`, **0 rows in anchor** | yes (runtime) |
| `entry_draft.db` / `expansion_draft.db` | PS4 NHL 18+ | draft / expansion roster pools (10-table `exhibition*` projection) | yes |
| `threes_campaign.db` | PS4 NHL 18+ | Threes mode roster projection | yes |
| `proam.db` | PS4 NHL 20 | Pro-Am roster projection | yes |
| `initial_experience.db` | PS4 all | onboarding default full-game snapshot (35–43 tables) | yes |
| `nhl_eng_us.db` / `eng_us.db` | all | localization — single `LanguageStrings` (`vCJG`) hash-dictionary | text-only |

> Note: PS4 `eng_us.db` is **absent for NHL 19 and NHL 21** (meta present, DB not shipped in the
> anchor set) — flagged for anchor completeness.

## 5. Editable-table taxonomy (naming legend)

| Prefix | Human meaning | State in anchor | Edit relevance |
|---|---|---|---|
| `exhibition*` | Active roster (players, bios, equipment, AI, teams, slots, free agents, arenas) | **populated** | primary edit surface |
| `custom*` | Creation-Zone / custom content (custom players, faces, palettes, team equip) | populated | user content |
| `stock*` | Shipped reference: appearance (heads/jerseys/colors) + read-only career/stat baselines | populated | appearance editable; stats read-only |
| `league*` | Dynasty / Be-A-GM runtime (rosters, schedules, stats, contracts, trades, awards…) | **empty (0 rows)** | filled at play; huge insert budgets |
| `*schedule` | League calendars (NHL/AHL/WHL/OHL/QMJHL/Euro leagues; 32-team variants on PS4) | populated | editable |
| `interview*`/`invitations*` | Story-mode Q&A text (Legacy/PS4) | populated | text |
| `dynamic*` | Live rating overlays | empty | runtime |
| text/name | `playernames`, `*texts` string pools | populated | text |

**Edit column legend:** `I`=insertable, `U`=updatable, `D`=deletable (from meta insert/update/delete
caps); `ro`=read-only (no diff caps). `maxinsert(per-txn)` is the per-diff insert budget (§2), blank =
not insertable.

## 6. Per-game inventory (sorted by game)

Each game lists its main DB's family breakdown, the full **editable roster surface**
(`exhibition*`+`custom*`+`stock*`), every auxiliary DB (as a projection), and its localization DB.
`league*`/stat/text tables are summarized in the family table and fully enumerated in the CSV
appendix.
<!-- GEN7 -->

### NHL 10 — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **86 tables**, **51,971 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 16 | 20,542 | 16 |
| custom* (Creation Zone content) | 3 | 860 | 3 |
| stock* (shipped reference/appearance) | 13 | 23,929 | 3 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 45 | 0 | 45 |
| *schedule (league calendars) | 7 | 4,210 | 7 |
| text/name pools | 1 | 2,405 | 0 |
| other / misc | 1 | 25 | 0 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 8696 | 18 | ro |  |
| stockteamequipmenttable | `hHbv` | 8000 | 33 | ·U· |  |
| exhibitionplayers | `ZBac` | 4188 | 3 | IUD | 1500 |
| exhibitionplayerbiotable | `ubPc` | 3933 | 34 | IUD | 600 |
| stockcareerplayoffplayerstats | `OAqO` | 3739 | 15 | ro |  |
| exhibitionrostertable | `eGlu` | 3719 | 71 | IUD | 2500 |
| exhibitionskaterai | `dSvy` | 3499 | 34 | IUD | 500 |
| exhibitionskaterequipment | `xmja` | 3499 | 15 | IUD | 500 |
| stockintltourneyplayerstats | `xVwb` | 1222 | 8 | ro |  |
| stockcareerseasongoaliestats | `USfF` | 947 | 20 | ro |  |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| exhibitiongoalieai | `mHuy` | 434 | 30 | IUD | 100 |
| exhibitiongoalieequipment | `fMVl` | 434 | 77 | IUD | 100 |
| stockgoalieequipment | `yUAn` | 388 | 81 | IUD | 100 |
| stockcareerplayoffgoaliestats | `nCDp` | 297 | 17 | ro |  |
| exhibitionfreeagents | `VoEQ` | 289 | 1 | I·D | 800 |
| exhibitionarena | `StEO` | 182 | 13 | ·U· |  |
| exhibitionteams | `kOtt` | 182 | 46 | ·U· |  |
| exhibitionteamstrategies | `EeYo` | 182 | 11 | ·U· |  |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| stockawardstable | `mkrU` | 178 | 10 | ro |  |
| stockteamcolorstable | `vzos` | 143 | 6 | IU· | 60 |
| stockintltourneygoaliestats | `UrBg` | 137 | 10 | ro |  |
| stockretirednumbertable | `wcSL` | 120 | 3 | ro |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |
| exhibitioncontroller | `plCE` | 0 | 4 | IUD | 1 |
| exhibitionoptions | `fWLJ` | 0 | 2 | IUD | 1 |
| exhibitionrules | `fiKF` | 0 | 2 | IUD | 2 |
| exhibitiontweaks | `RCwP` | 0 | 2 | IUD | 2 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 12,505 strings.


### NHL 11 — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **131 tables**, **74,479 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 13 | 28,922 | 13 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 15 | 28,285 | 5 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 44 | 0 | 44 |
| *schedule (league calendars) | 12 | 6,696 | 10 |
| text/name pools | 1 | 2,405 | 0 |
| other / misc | 42 | 7,251 | 1 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockteamequipmenttable | `hHbv` | 10000 | 33 | ·U· |  |
| stockcareerseasonplayerstats | `tsSy` | 9297 | 18 | ro |  |
| exhibitionplayers | `ZBac` | 5593 | 3 | IUD | 1500 |
| exhibitionplayerbiotable | `ubPc` | 5310 | 44 | IUD | 600 |
| exhibitionrostertable | `eGlu` | 5228 | 66 | IUD | 2500 |
| exhibitionskaterai | `dSvy` | 4760 | 36 | IUD | 500 |
| exhibitionskaterequipment | `xmja` | 4760 | 16 | IUD | 500 |
| stockcareerplayoffplayerstats | `OAqO` | 3865 | 15 | ro |  |
| exhibitiondraftpick | `qHav` | 1260 | 7 | ·U· |  |
| stockintltourneyplayerstats | `xVwb` | 1027 | 8 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 1008 | 12 | IUD | 100 |
| stockcareerseasongoaliestats | `USfF` | 983 | 20 | ro |  |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| stockteamjerseys | `xjgw` | 627 | 11 | IUD | 10 |
| exhibitiongoalieai | `mHuy` | 550 | 29 | IUD | 100 |
| exhibitiongoalieequipment | `fMVl` | 550 | 104 | IUD | 100 |
| stockgoalieequipment | `yUAn` | 538 | 106 | IUD | 100 |
| stockcareerplayoffgoaliestats | `nCDp` | 311 | 17 | ro |  |
| exhibitionarena | `StEO` | 242 | 14 | ·U· |  |
| exhibitionteams | `kOtt` | 242 | 46 | ·U· |  |
| exhibitionteamstrategies | `EeYo` | 242 | 11 | ·U· |  |
| stockawardstable | `mkrU` | 198 | 10 | ro |  |
| exhibitionfreeagents | `VoEQ` | 184 | 1 | I·D | 800 |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| stockteamcolorstable | `vzos` | 140 | 6 | IU· | 60 |
| stockretirednumbertable | `wcSL` | 120 | 3 | ro |  |
| stockintltourneygoaliestats | `UrBg` | 109 | 10 | ro |  |
| customteamjerseys | `PuTc` | 60 | 11 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |

**`draft_roster.db`** — 9 tables, 28,200 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 5598 | 3 |
| exhibitionplayerbiotable | `ubPc` | 5308 | 44 |
| exhibitionrostertable | `eGlu` | 5272 | 66 |
| exhibitionskaterai | `dSvy` | 4758 | 36 |
| exhibitionskaterequipment | `xmja` | 4758 | 16 |
| exhibitiondraftpick | `qHav` | 1260 | 7 |
| exhibitiongoalieai | `mHuy` | 550 | 29 |
| exhibitiongoalieequipment | `fMVl` | 550 | 104 |
| exhibitionfreeagents | `VoEQ` | 146 | 1 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 14,920 strings.


### NHL 12 — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **133 tables**, **77,411 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 14 | 29,308 | 14 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 16 | 28,653 | 7 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 39 | 0 | 39 |
| *schedule (league calendars) | 15 | 6,900 | 13 |
| text/name pools | 1 | 4,200 | 0 |
| other / misc | 44 | 7,430 | 3 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 10377 | 19 | ro |  |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayers | `ZBac` | 5779 | 3 | IUD | 2500 |
| exhibitionplayerbiotable | `ubPc` | 5395 | 42 | IUD | 1500 |
| exhibitionrostertable | `eGlu` | 5349 | 66 | IUD | 2500 |
| exhibitionskaterai | `dSvy` | 4821 | 68 | IUD | 1500 |
| exhibitionskaterequipment | `xmja` | 4821 | 16 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4264 | 16 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 2420 | 12 | IUD | 1000 |
| stockcareerseasongoaliestats | `USfF` | 1131 | 20 | ro |  |
| exhibitiondraftpick | `qHav` | 1050 | 7 | ·U· |  |
| stockplayerheads | `uquv` | 1032 | 4 | IUD | 100 |
| stockintltourneyplayerstats | `xVwb` | 978 | 8 | ro |  |
| stockteamjerseys | `xjgw` | 676 | 11 | IUD | 60 |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| exhibitiongoalieai | `mHuy` | 574 | 61 | IUD | 300 |
| exhibitiongoalieequipment | `fMVl` | 574 | 104 | IUD | 300 |
| stockgoalieequipment | `yUAn` | 562 | 106 | IUD | 300 |
| stockcareerplayoffgoaliestats | `nCDp` | 356 | 17 | ro |  |
| stockretirednumbertable | `wcSL` | 336 | 2 | I·D | 60 |
| exhibitionarena | `StEO` | 243 | 14 | ·U· |  |
| exhibitionteams | `kOtt` | 243 | 50 | ·U· |  |
| exhibitionteamstrategies | `EeYo` | 243 | 11 | ·U· |  |
| stockawardstable | `mkrU` | 218 | 10 | ro |  |
| exhibitionfreeagents | `VoEQ` | 214 | 1 | I·D | 1500 |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| stockteamcolorstable | `vzos` | 140 | 6 | IU· | 60 |
| stockintltourneygoaliestats | `UrBg` | 101 | 10 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |
| exhibitiondlcnew | `GFxF` | 1 | 2 | ·U· |  |

**`draft_roster.db`** — 9 tables, 28,598 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 5783 | 3 |
| exhibitionplayerbiotable | `ubPc` | 5398 | 42 |
| exhibitionrostertable | `eGlu` | 5353 | 66 |
| exhibitionskaterai | `dSvy` | 4824 | 68 |
| exhibitionskaterequipment | `xmja` | 4824 | 16 |
| exhibitiondraftpick | `qHav` | 1050 | 7 |
| exhibitiongoalieai | `mHuy` | 574 | 61 |
| exhibitiongoalieequipment | `fMVl` | 574 | 104 |
| exhibitionfreeagents | `VoEQ` | 218 | 1 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 16,787 strings.


### NHL 13 — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **136 tables**, **79,628 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 14 | 29,671 | 14 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 16 | 29,376 | 7 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 50 | 0 | 50 |
| *schedule (league calendars) | 16 | 8,171 | 14 |
| text/name pools | 1 | 4,516 | 0 |
| other / misc | 35 | 6,974 | 2 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 10873 | 19 | ro |  |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayers | `ZBac` | 5874 | 3 | IUD | 3000 |
| exhibitionrostertable | `eGlu` | 5446 | 78 | IUD | 5000 |
| exhibitionplayerbiotable | `ubPc` | 5412 | 52 | IUD | 1500 |
| exhibitionskaterai | `dSvy` | 4875 | 70 | IUD | 1500 |
| exhibitionskaterequipment | `xmja` | 4875 | 16 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4432 | 16 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 2541 | 12 | IUD | 600 |
| exhibitiondraftpick | `qHav` | 1260 | 7 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1215 | 20 | ro |  |
| stockplayerheads | `uquv` | 1082 | 4 | IUD | 100 |
| stockintltourneyplayerstats | `xVwb` | 838 | 8 | ro |  |
| stockteamjerseys | `xjgw` | 684 | 13 | IUD | 60 |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| stockgoalieequipment | `yUAn` | 550 | 106 | IUD | 300 |
| exhibitiongoalieai | `mHuy` | 537 | 63 | IUD | 300 |
| exhibitiongoalieequipment | `fMVl` | 537 | 104 | IUD | 300 |
| stockcareerplayoffgoaliestats | `nCDp` | 377 | 17 | ro |  |
| stockretirednumbertable | `wcSL` | 325 | 2 | I·D | 60 |
| exhibitionteams | `kOtt` | 248 | 51 | ·U· |  |
| exhibitionteamstrategies | `EeYo` | 248 | 61 | ·U· |  |
| exhibitionfreeagents | `VoEQ` | 183 | 1 | I·D | 1500 |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| exhibitionarena | `StEO` | 174 | 18 | IU· | 5 |
| stockteamcolorstable | `vzos` | 140 | 6 | IU· | 60 |
| stockawardstable | `mkrU` | 138 | 10 | ro |  |
| stockintltourneygoaliestats | `UrBg` | 119 | 10 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |
| exhibitiondlcnew | `GFxF` | 1 | 2 | ·U· |  |

**`draft_roster.db`** — 9 tables, 28,856 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 5842 | 3 |
| exhibitionrostertable | `eGlu` | 5455 | 78 |
| exhibitionplayerbiotable | `ubPc` | 5381 | 52 |
| exhibitionskaterai | `dSvy` | 4847 | 70 |
| exhibitionskaterequipment | `xmja` | 4847 | 16 |
| exhibitiondraftpick | `qHav` | 1260 | 7 |
| exhibitiongoalieai | `mHuy` | 534 | 63 |
| exhibitiongoalieequipment | `fMVl` | 534 | 104 |
| exhibitionfreeagents | `VoEQ` | 156 | 1 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 18,826 strings.


### NHL 14 — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **134 tables**, **85,401 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 14 | 29,641 | 14 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 19 | 31,504 | 9 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 52 | 0 | 52 |
| *schedule (league calendars) | 17 | 8,171 | 12 |
| text/name pools | 4 | 5,339 | 0 |
| dynamic* (live rating overlay) | 2 | 0 | 2 |
| interview* / invitations* (story-mode text) | 6 | 8,386 | 1 |
| invitations* | 6 | 1,182 | 0 |
| other / misc | 10 | 258 | 2 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 11114 | 19 | ro |  |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayers | `ZBac` | 5827 | 3 | IUD | 3000 |
| exhibitionplayerbiotable | `ubPc` | 5374 | 55 | IUD | 1500 |
| exhibitionrostertable | `eGlu` | 5353 | 78 | IUD | 5000 |
| exhibitionskaterequipment | `xmja` | 4851 | 16 | IUD | 1500 |
| exhibitionskaterai | `dSvy` | 4851 | 70 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4559 | 16 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 2603 | 12 | IUD | 600 |
| stockplayervisorexempttable | `zkcJ` | 1806 | 1 | IUD | 300 |
| exhibitiondraftpick | `qHav` | 1470 | 7 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1142 | 20 | ro |  |
| stockplayerheads | `uquv` | 761 | 3 | IUD | 100 |
| stockintltourneyplayerstats | `xVwb` | 748 | 8 | ro |  |
| stockteamjerseys | `xjgw` | 704 | 13 | IUD | 60 |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| stockgoalieequipment | `yUAn` | 543 | 106 | IUD | 300 |
| exhibitiongoalieequipment | `fMVl` | 523 | 104 | IUD | 300 |
| exhibitiongoalieai | `mHuy` | 523 | 63 | IUD | 300 |
| stockawardstable | `mkrU` | 425 | 10 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 356 | 17 | ro |  |
| stockretirednumbertable | `wcSL` | 321 | 2 | I·D | 60 |
| exhibitionteamstrategies | `EeYo` | 246 | 61 | ·U· |  |
| exhibitionteams | `kOtt` | 246 | 51 | ·U· |  |
| exhibitionfreeagents | `VoEQ` | 198 | 1 | IUD | 1500 |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| exhibitionarena | `StEO` | 177 | 18 | IU· | 5 |
| stockteamcolorstable | `vzos` | 134 | 6 | IU· | 60 |
| stocktweets | `MSRy` | 111 | 7 | ro |  |
| stockintltourneygoaliestats | `UrBg` | 85 | 10 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockteamsalaryextras | `Teix` | 30 | 76 | IUD | 30 |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlcnew | `GFxF` | 1 | 2 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |

**`draft_roster.db`** — 10 tables, 29,000 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 5827 | 3 |
| exhibitionplayerbiotable | `ubPc` | 5374 | 55 |
| exhibitionrostertable | `eGlu` | 5353 | 78 |
| exhibitionskaterai | `dSvy` | 4851 | 70 |
| exhibitionskaterequipment | `xmja` | 4851 | 16 |
| exhibitiondraftpick | `qHav` | 1470 | 7 |
| exhibitiongoalieai | `mHuy` | 523 | 63 |
| exhibitiongoalieequipment | `fMVl` | 523 | 104 |
| exhibitionfreeagents | `VoEQ` | 198 | 1 |
| stockteamsalaryextras | `Teix` | 30 | 76 |

**`dynamic_ratings.db`** — 2 tables, 0 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| dynamicskaterai | `mDFM` | 0 | 28 |
| dynamicgoalieai | `dOoo` | 0 | 23 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 24,736 strings.


### NHL 15 — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **134 tables**, **88,571 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 14 | 30,448 | 14 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 19 | 32,352 | 9 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 52 | 0 | 52 |
| *schedule (league calendars) | 17 | 9,402 | 12 |
| text/name pools | 4 | 5,623 | 0 |
| dynamic* (live rating overlay) | 2 | 0 | 2 |
| interview* / invitations* (story-mode text) | 6 | 8,386 | 1 |
| invitations* | 6 | 1,182 | 0 |
| other / misc | 10 | 258 | 2 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 11343 | 19 | ro |  |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayers | `ZBac` | 5942 | 3 | IUD | 3000 |
| exhibitionplayerbiotable | `ubPc` | 5534 | 55 | IUD | 1500 |
| exhibitionrostertable | `eGlu` | 5511 | 78 | IUD | 5000 |
| exhibitionskaterequipment | `xmja` | 4978 | 16 | IUD | 1500 |
| exhibitionskaterai | `dSvy` | 4978 | 70 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4580 | 16 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 2660 | 12 | IUD | 600 |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 | IUD | 300 |
| stockplayerheads | `uquv` | 1521 | 3 | IUD | 100 |
| exhibitiondraftpick | `qHav` | 1470 | 7 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1242 | 20 | ro |  |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| stockteamjerseys | `xjgw` | 659 | 13 | IUD | 60 |
| exhibitiongoalieequipment | `fMVl` | 556 | 104 | IUD | 300 |
| exhibitiongoalieai | `mHuy` | 556 | 63 | IUD | 300 |
| stockgoalieequipment | `yUAn` | 555 | 106 | IUD | 300 |
| stockintltourneyplayerstats | `xVwb` | 492 | 8 | ro |  |
| stockawardstable | `mkrU` | 433 | 10 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 379 | 17 | ro |  |
| stockretirednumbertable | `wcSL` | 315 | 2 | I·D | 60 |
| exhibitionfreeagents | `VoEQ` | 251 | 1 | IUD | 1500 |
| exhibitionteamstrategies | `EeYo` | 246 | 61 | ·U· |  |
| exhibitionteams | `kOtt` | 246 | 51 | ·U· |  |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| exhibitionarena | `StEO` | 178 | 18 | IU· | 5 |
| stockteamcolorstable | `vzos` | 115 | 6 | IU· | 60 |
| stocktweets | `MSRy` | 111 | 7 | ro |  |
| stockintltourneygoaliestats | `UrBg` | 76 | 10 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockteamsalaryextras | `Teix` | 30 | 76 | IUD | 30 |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlcnew | `GFxF` | 1 | 2 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |

**`draft_roster.db`** — 10 tables, 29,813 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 5947 | 3 |
| exhibitionrostertable | `eGlu` | 5618 | 78 |
| exhibitionplayerbiotable | `ubPc` | 5533 | 55 |
| exhibitionskaterai | `dSvy` | 4978 | 70 |
| exhibitionskaterequipment | `xmja` | 4978 | 16 |
| exhibitiondraftpick | `qHav` | 1470 | 7 |
| exhibitiongoalieai | `mHuy` | 555 | 63 |
| exhibitiongoalieequipment | `fMVl` | 555 | 104 |
| exhibitionfreeagents | `VoEQ` | 149 | 1 |
| stockteamsalaryextras | `Teix` | 30 | 76 |

**`dynamic_ratings.db`** — 2 tables, 0 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| dynamicskaterai | `mDFM` | 0 | 28 |
| dynamicgoalieai | `dOoo` | 0 | 23 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 25,169 strings.


### NHL Legacy — PS3 = Xbox360 (byte-identical DB)

`nhlng.db` — **134 tables**, **88,525 records**, endian=big, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 14 | 30,808 | 14 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 19 | 32,378 | 9 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 52 | 0 | 52 |
| *schedule (league calendars) | 17 | 9,455 | 12 |
| text/name pools | 4 | 5,138 | 0 |
| dynamic* (live rating overlay) | 2 | 0 | 2 |
| interview* / invitations* (story-mode text) | 6 | 8,386 | 1 |
| invitations* | 6 | 1,182 | 0 |
| other / misc | 10 | 258 | 2 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 11716 | 19 | ro |  |
| exhibitionplayers | `ZBac` | 6087 | 3 | IUD | 3000 |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayerbiotable | `ubPc` | 5632 | 53 | IUD | 1500 |
| exhibitionrostertable | `eGlu` | 5532 | 78 | IUD | 5000 |
| exhibitionskaterequipment | `xmja` | 5066 | 16 | IUD | 1500 |
| exhibitionskaterai | `dSvy` | 5066 | 70 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4708 | 16 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 2173 | 12 | IUD | 600 |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 | IUD | 100 |
| stockplayerheads | `uquv` | 1645 | 3 | IUD | 50 |
| exhibitiondraftpick | `qHav` | 1260 | 7 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1161 | 20 | ro |  |
| stockteamjerseys | `xjgw` | 703 | 13 | IUD | 60 |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| exhibitiongoalieequipment | `fMVl` | 566 | 104 | IUD | 300 |
| exhibitiongoalieai | `mHuy` | 566 | 63 | IUD | 300 |
| stockgoalieequipment | `yUAn` | 552 | 106 | IUD | 300 |
| stockawardstable | `mkrU` | 432 | 10 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 365 | 17 | ro |  |
| stockintltourneyplayerstats | `xVwb` | 361 | 8 | ro |  |
| exhibitionfreeagents | `VoEQ` | 340 | 1 | IUD | 1500 |
| stockretirednumbertable | `wcSL` | 322 | 2 | I·D | 60 |
| exhibitionteamstrategies | `EeYo` | 252 | 61 | ·U· |  |
| exhibitionteams | `kOtt` | 252 | 51 | ·U· |  |
| stockteamcolorstable | `vzos` | 205 | 6 | IU· | 30 |
| exhibitionarena | `StEO` | 187 | 18 | IU· | 5 |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| stocktweets | `MSRy` | 126 | 7 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockintltourneygoaliestats | `UrBg` | 38 | 10 | ro |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockteamsalaryextras | `Teix` | 30 | 76 | ·UD |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlcnew | `GFxF` | 1 | 2 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |

**`draft_roster.db`** — 10 tables, 30,205 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 6093 | 3 |
| exhibitionrostertable | `eGlu` | 5701 | 78 |
| exhibitionplayerbiotable | `ubPc` | 5643 | 53 |
| exhibitionskaterai | `dSvy` | 5073 | 70 |
| exhibitionskaterequipment | `xmja` | 5073 | 16 |
| exhibitiondraftpick | `qHav` | 1260 | 7 |
| exhibitiongoalieai | `mHuy` | 570 | 63 |
| exhibitiongoalieequipment | `fMVl` | 570 | 104 |
| exhibitionfreeagents | `VoEQ` | 192 | 1 |
| stockteamsalaryextras | `Teix` | 30 | 76 |

**`dynamic_ratings.db`** — 2 tables, 0 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| dynamicskaterai | `mDFM` | 0 | 28 |
| dynamicgoalieai | `dOoo` | 0 | 23 |

**`nhl_eng_us.db`** — localization: LanguageStrings `vCJG`, 25,782 strings.


<!-- GEN8 -->

### NHL 15 — PS4

`nhl_game.db` — **148 tables**, **90,227 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 14 | 30,571 | 14 |
| custom* (Creation Zone content) | 4 | 920 | 4 |
| stock* (shipped reference/appearance) | 25 | 33,775 | 9 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 59 | 0 | 59 |
| *schedule (league calendars) | 17 | 9,402 | 12 |
| text/name pools | 4 | 5,623 | 0 |
| dynamic* (live rating overlay) | 2 | 0 | 2 |
| interview* / invitations* (story-mode text) | 6 | 8,386 | 1 |
| invitations* | 6 | 1,182 | 0 |
| other / misc | 11 | 368 | 2 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 11277 | 19 | ro |  |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayers | `ZBac` | 5984 | 3 | IUD | 3000 |
| exhibitionplayerbiotable | `ubPc` | 5555 | 56 | IUD | 1500 |
| exhibitionrostertable | `eGlu` | 5523 | 78 | IUD | 5000 |
| exhibitionskaterequipment | `xmja` | 4996 | 16 | IUD | 1500 |
| exhibitionskaterai | `dSvy` | 4996 | 77 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4552 | 16 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 2660 | 12 | IUD | 600 |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 | IUD | 300 |
| stockplayerheads | `uquv` | 1521 | 3 | IUD | 100 |
| exhibitiondraftpick | `qHav` | 1470 | 7 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1227 | 20 | ro |  |
| stocksettinggroups | `Oreg` | 702 | 3 | ro |  |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| stockteamjerseys | `xjgw` | 659 | 13 | IUD | 60 |
| exhibitiongoalieequipment | `fMVl` | 559 | 104 | IUD | 300 |
| exhibitiongoalieai | `mHuy` | 559 | 67 | IUD | 300 |
| stockgoalieequipment | `yUAn` | 554 | 106 | IUD | 300 |
| stockintltourneyplayerstats | `xVwb` | 492 | 8 | ro |  |
| stockawardstable | `mkrU` | 433 | 10 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 376 | 17 | ro |  |
| stocksettingdefaults | `gZtX` | 373 | 3 | ro |  |
| stockretirednumbertable | `wcSL` | 315 | 2 | I·D | 60 |
| exhibitionfreeagents | `VoEQ` | 257 | 1 | IUD | 1500 |
| exhibitionteamstrategies | `EeYo` | 246 | 61 | ·U· |  |
| exhibitionteams | `kOtt` | 246 | 55 | ·U· |  |
| stocksettings | `fRvu` | 223 | 8 | ro |  |
| stocksettingcontext | `HwRX` | 197 | 5 | ro |  |
| customfacetable | `qjzV` | 180 | 12 | ·U· |  |
| exhibitionarena | `StEO` | 178 | 18 | IU· | 5 |
| stockteamcolorstable | `vzos` | 115 | 6 | IU· | 60 |
| stocktweets | `MSRy` | 111 | 7 | ro |  |
| stockintltourneygoaliestats | `UrBg` | 72 | 10 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| stockteamsalaryextras | `Teix` | 30 | 76 | IUD | 30 |
| stocksettingdependencies | `FzWx` | 25 | 3 | ro |  |
| stockleagues | `fsLK` | 20 | 3 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |
| exhibitiondlcnew | `GFxF` | 1 | 2 | ·U· |  |
| exhibitiondlc | `SadW` | 1 | 2 | ·U· |  |

**`initial_experience.db`** — 35 tables, 52,926 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 6101 | 3 |
| stockteamequipmenttable | `hHbv` | 6000 | 33 |
| exhibitionplayerbiotable | `ubPc` | 5534 | 56 |
| exhibitionrostertable | `eGlu` | 5503 | 78 |
| exhibitionskaterai | `dSvy` | 4978 | 77 |
| exhibitionskaterequipment | `xmja` | 4978 | 16 |
| stockplayerhelmetplacement | `DzSF` | 2659 | 12 |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 |
| stockplayerheads | `uquv` | 1521 | 3 |
| exhibitiondraftpick | `qHav` | 1470 | 7 |
| favoriteteamschedule | `qiwI` | 1231 | 7 |
| nhlschedule | `Smhi` | 1231 | 7 |
| nhlfutureschedule | `DEyb` | 1231 | 7 |
| ahlschedule | `NKJA` | 1140 | 7 |
| whlschedule | `DZTk` | 792 | 7 |
| ohlschedule | `jEiu` | 680 | 7 |
| stockteamjerseys | `xjgw` | 664 | 13 |
| qmjhlschedule | `cfSy` | 612 | 7 |
| exhibitiongoalieai | `mHuy` | 556 | 67 |
| exhibitiongoalieequipment | `fMVl` | 556 | 104 |
| stockgoalieequipment | `yUAn` | 555 | 106 |
| finnishschedule | `QQBR` | 406 | 7 |
| germanschedule | `hxjN` | 364 | 7 |
| czechschedule | `WQzR` | 364 | 7 |
| swedishschedule | `TemL` | 330 | 7 |
| stockretirednumbertable | `wcSL` | 315 | 2 |
| swissschedule | `vlni` | 300 | 7 |
| exhibitionfreeagents | `VoEQ` | 251 | 1 |
| exhibitionteams | `kOtt` | 246 | 55 |
| exhibitionteamstrategies | `EeYo` | 246 | 61 |
| exhibitionarena | `StEO` | 178 | 18 |
| stockteamcolorstable | `vzos` | 115 | 6 |
| stockteamsalaryextras | `Teix` | 30 | 76 |
| wcgoalieequipment | `kJlD` | 9 | 104 |
| exhibitiondlcnew | `GFxF` | 1 | 2 |

**`eng_us.db`** — localization: LanguageStrings `GJCv`, 24,251 strings.


### NHL 16 — PS4

`nhl_game.db` — **135 tables**, **94,209 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 12 | 30,806 | 12 |
| custom* (Creation Zone content) | 3 | 740 | 3 |
| stock* (shipped reference/appearance) | 18 | 38,277 | 9 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 54 | 0 | 54 |
| *schedule (league calendars) | 17 | 9,455 | 12 |
| text/name pools | 4 | 5,138 | 0 |
| dynamic* (live rating overlay) | 2 | 0 | 2 |
| interview* / invitations* (story-mode text) | 6 | 8,386 | 1 |
| invitations* | 6 | 1,182 | 0 |
| other / misc | 13 | 225 | 5 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 11716 | 19 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 8449 | 12 | IUD | 1000 |
| exhibitionplayers | `ZBac` | 6087 | 3 | IUD | 3000 |
| stockteamequipmenttable | `hHbv` | 6000 | 33 | ·U· |  |
| exhibitionplayerbiotable | `ubPc` | 5632 | 88 | IUD | 1500 |
| exhibitionrostertable | `eGlu` | 5532 | 87 | IUD | 5000 |
| exhibitionskaterequipment | `xmja` | 5066 | 43 | IUD | 1500 |
| exhibitionskaterai | `dSvy` | 5066 | 78 | IUD | 1500 |
| stockcareerplayoffplayerstats | `OAqO` | 4708 | 16 | ro |  |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 | IUD | 100 |
| stockplayerheads | `uquv` | 1645 | 4 | IUD | 50 |
| exhibitiondraftpick | `qHav` | 1260 | 5 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1161 | 20 | ro |  |
| stockteamjerseys | `xjgw` | 703 | 13 | IUD | 60 |
| customteamequipmenttable | `oEDN` | 660 | 32 | ·U· |  |
| exhibitiongoalieequipment | `fMVl` | 566 | 104 | IUD | 300 |
| exhibitiongoalieai | `mHuy` | 566 | 69 | IUD | 300 |
| stockgoalieequipment | `yUAn` | 552 | 106 | IUD | 300 |
| stockawardstable | `mkrU` | 432 | 10 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 365 | 17 | ro |  |
| exhibitionfreeagents | `VoEQ` | 340 | 1 | IUD | 1500 |
| stockretirednumbertable | `wcSL` | 322 | 2 | I·D | 60 |
| exhibitionteams | `kOtt` | 252 | 104 | ·U· |  |
| exhibitionteamstrategies | `EeYo` | 252 | 61 | ·U· |  |
| stockteamcolorstable | `vzos` | 205 | 6 | IU· | 30 |
| exhibitionarena | `StEO` | 187 | 18 | IU· | 5 |
| stocktweets | `MSRy` | 126 | 7 | ro |  |
| customteamjerseys | `PuTc` | 60 | 12 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockteamsalaryextras | `Teix` | 30 | 76 | ·UD |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| stockleagues | `fsLK` | 22 | 4 | ro |  |
| custompalettetable | `puoR` | 20 | 5 | ·U· |  |

**`initial_experience.db`** — 33 tables, 59,353 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| stockplayerhelmetplacement | `DzSF` | 8447 | 12 |
| exhibitionplayers | `ZBac` | 6105 | 3 |
| stockteamequipmenttable | `hHbv` | 6000 | 33 |
| exhibitionrostertable | `eGlu` | 5709 | 87 |
| exhibitionplayerbiotable | `ubPc` | 5653 | 88 |
| exhibitionskaterequipment | `xmja` | 5081 | 43 |
| exhibitionskaterai | `dSvy` | 5081 | 78 |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 |
| stockplayerheads | `uquv` | 1645 | 4 |
| exhibitiondraftpick | `qHav` | 1260 | 5 |
| nhlschedule | `Smhi` | 1231 | 6 |
| nhlfutureschedule | `DEyb` | 1231 | 6 |
| favoriteteamschedule | `qiwI` | 1231 | 6 |
| ahlschedule | `NKJA` | 1120 | 6 |
| whlschedule | `DZTk` | 792 | 6 |
| stockteamjerseys | `xjgw` | 703 | 13 |
| ohlschedule | `jEiu` | 680 | 6 |
| qmjhlschedule | `cfSy` | 612 | 6 |
| exhibitiongoalieequipment | `fMVl` | 572 | 104 |
| exhibitiongoalieai | `mHuy` | 572 | 69 |
| stockgoalieequipment | `yUAn` | 570 | 106 |
| finnishschedule | `QQBR` | 450 | 6 |
| germanschedule | `hxjN` | 364 | 6 |
| czechschedule | `WQzR` | 364 | 6 |
| swedishschedule | `TemL` | 364 | 6 |
| stockretirednumbertable | `wcSL` | 322 | 2 |
| swissschedule | `vlni` | 300 | 6 |
| exhibitionteams | `kOtt` | 252 | 104 |
| exhibitionteamstrategies | `EeYo` | 252 | 61 |
| stockteamcolorstable | `vzos` | 202 | 6 |
| exhibitionfreeagents | `VoEQ` | 192 | 1 |
| exhibitionarena | `StEO` | 187 | 18 |
| stockteamsalaryextras | `Teix` | 30 | 76 |

**`eng_us.db`** — localization: LanguageStrings `GJCv`, 27,468 strings.


### NHL 17 — PS4

`nhl_game.db` — **118 tables**, **101,281 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 12 | 33,768 | 12 |
| custom* (Creation Zone content) | 13 | 7,082 | 13 |
| stock* (shipped reference/appearance) | 19 | 44,470 | 9 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 49 | 0 | 49 |
| *schedule (league calendars) | 20 | 10,602 | 14 |
| text/name pools | 2 | 5,307 | 0 |
| other / misc | 3 | 52 | 0 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 12804 | 19 | ro |  |
| stockplayerhelmetplacement | `DzSF` | 9081 | 12 | IUD | 1000 |
| exhibitionplayers | `ZBac` | 6701 | 3 | IUD | 3500 |
| exhibitionrostertable | `eGlu` | 6430 | 87 | IUD | 5000 |
| exhibitionplayerbiotable | `ubPc` | 6128 | 96 | IUD | 2500 |
| stockteamequipmenttable | `hHbv` | 6000 | 21 | ·U· |  |
| exhibitionskaterequipment | `xmja` | 5527 | 29 | IUD | 2000 |
| exhibitionskaterai | `dSvy` | 5527 | 81 | IUD | 2000 |
| stockcareerplayoffplayerstats | `OAqO` | 5120 | 16 | ro |  |
| stockcareermiscstats | `NPzU` | 3589 | 7 | ro |  |
| customteamequipmenttable | `oEDN` | 1950 | 21 | ·U· |  |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 | IUD | 100 |
| stockplayerheads | `uquv` | 1662 | 4 | IUD | 50 |
| stockcareerseasongoaliestats | `USfF` | 1352 | 20 | ro |  |
| exhibitiondraftpick | `qHav` | 1260 | 5 | ·U· |  |
| customplayerbiotable | `KdqM` | 1208 | 96 | IUD | 2000 |
| customplayers | `Myas` | 1208 | 3 | IUD | 2000 |
| customskaterequipment | `kVlL` | 1006 | 29 | IUD | 1750 |
| customskaterai | `NXqo` | 1006 | 81 | IUD | 1750 |
| stockteamjerseys | `xjgw` | 813 | 28 | IUD | 60 |
| stockgoalieequipment | `yUAn` | 654 | 68 | IUD | 500 |
| exhibitiongoalieequipment | `fMVl` | 601 | 65 | IUD | 500 |
| exhibitiongoalieai | `mHuy` | 601 | 69 | IUD | 500 |
| stockawardstable | `mkrU` | 430 | 10 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 423 | 17 | ro |  |
| stockretirednumbertable | `wcSL` | 316 | 2 | I·D | 60 |
| exhibitionarena | `StEO` | 256 | 124 | IU· | 5 |
| exhibitionteams | `kOtt` | 256 | 97 | ·U· |  |
| exhibitionteamstrategies | `EeYo` | 256 | 61 | ·U· |  |
| exhibitionfreeagents | `VoEQ` | 225 | 1 | IUD | 5000 |
| stockteamcolorstable | `vzos` | 205 | 4 | IU· | 30 |
| customgoalieequipment | `yrDg` | 202 | 65 | IUD | 250 |
| customgoalieai | `GezY` | 202 | 69 | IUD | 250 |
| customteamjerseys | `PuTc` | 150 | 28 | ·U· |  |
| stocktweets | `MSRy` | 126 | 7 | ro |  |
| customarena | `Usqf` | 50 | 124 | ·U· |  |
| customteamstrategies | `IEZE` | 50 | 61 | ·U· |  |
| customteams | `SvDE` | 50 | 97 | ·U· |  |
| stockplayoffteamstats | `cyZi` | 32 | 14 | ro |  |
| stockteamsalaryextras | `Teix` | 30 | 76 | ·UD |  |
| stockseasonteamstats | `czBw` | 30 | 14 | ro |  |
| stockleagues | `fsLK` | 24 | 5 | ro |  |
| customrostertable | `eGsL` | 0 | 87 | IUD | 2000 |
| customfreeagents | `BeQk` | 0 | 2 | IUD | 3200 |

**`initial_experience.db`** — 35 tables, 67,484 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| stockplayerhelmetplacement | `DzSF` | 9081 | 12 |
| exhibitionplayers | `ZBac` | 7435 | 3 |
| exhibitionplayerbiotable | `ubPc` | 6928 | 96 |
| exhibitionrostertable | `eGlu` | 6714 | 87 |
| exhibitionskaterequipment | `xmja` | 6236 | 29 |
| exhibitionskaterai | `dSvy` | 6236 | 81 |
| stockteamequipmenttable | `hHbv` | 6000 | 21 |
| stockplayervisorexempttable | `zkcJ` | 1779 | 1 |
| stockplayerheads | `uquv` | 1662 | 4 |
| exhibitiondraftpick | `qHav` | 1260 | 5 |
| nhlschedule | `Smhi` | 1231 | 6 |
| nhlfutureschedule | `DEyb` | 1231 | 6 |
| favoriteteamschedule | `qiwI` | 1231 | 6 |
| ahlschedule | `NKJA` | 1120 | 6 |
| echlschedule | `pGMV` | 972 | 6 |
| stockteamjerseys | `xjgw` | 818 | 28 |
| whlschedule | `DZTk` | 792 | 6 |
| exhibitiongoalieequipment | `fMVl` | 692 | 65 |
| exhibitiongoalieai | `mHuy` | 692 | 69 |
| stockgoalieequipment | `yUAn` | 688 | 68 |
| ohlschedule | `jEiu` | 680 | 6 |
| qmjhlschedule | `cfSy` | 612 | 6 |
| finnishschedule | `QQBR` | 450 | 6 |
| stockretirednumbertable | `wcSL` | 366 | 2 |
| germanschedule | `hxjN` | 364 | 6 |
| czechschedule | `WQzR` | 364 | 6 |
| swedishschedule | `TemL` | 364 | 6 |
| swissschedule | `vlni` | 300 | 6 |
| exhibitionarena | `StEO` | 256 | 124 |
| exhibitionteams | `kOtt` | 256 | 97 |
| exhibitionteamstrategies | `EeYo` | 256 | 61 |
| stockteamcolorstable | `vzos` | 205 | 4 |
| exhibitionfreeagents | `VoEQ` | 166 | 1 |
| stockteamsalaryextras | `Teix` | 30 | 76 |
| wcschedule | `rUGT` | 17 | 6 |

**`eng_us.db`** — localization: LanguageStrings `GJCv`, 30,799 strings.


### NHL 18 — PS4

`nhl_game.db` — **117 tables**, **100,238 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 12 | 39,110 | 12 |
| custom* (Creation Zone content) | 13 | 7,351 | 13 |
| stock* (shipped reference/appearance) | 17 | 32,713 | 7 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 44 | 0 | 44 |
| *schedule (league calendars) | 26 | 15,709 | 20 |
| text/name pools | 2 | 5,303 | 0 |
| other / misc | 3 | 52 | 0 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 13189 | 24 | ro |  |
| exhibitionplayers | `ZBac` | 7735 | 3 | IUD | 4000 |
| exhibitionrostertable | `eGlu` | 7373 | 94 | IUD | 6000 |
| exhibitionplayerbiotable | `ubPc` | 7236 | 102 | IUD | 3000 |
| exhibitionskaterequipment | `xmja` | 6522 | 33 | IUD | 2500 |
| exhibitionskaterai | `dSvy` | 6522 | 81 | IUD | 2500 |
| stockcareerplayoffplayerstats | `OAqO` | 5325 | 21 | ro |  |
| stockteamequipmenttable | `hHbv` | 4876 | 21 | IU· | 1500 |
| stockcareermiscstats | `NPzU` | 3709 | 7 | ro |  |
| customteamequipmenttable | `oEDN` | 2145 | 21 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 1441 | 20 | ro |  |
| exhibitiondraftpick | `qHav` | 1302 | 5 | ·U· |  |
| customplayerbiotable | `KdqM` | 1208 | 102 | IUD | 2000 |
| customplayers | `Myas` | 1208 | 3 | IUD | 2000 |
| stockteamjerseys | `xjgw` | 1030 | 29 | IUD | 100 |
| customskaterequipment | `kVlL` | 1006 | 33 | IUD | 1750 |
| customskaterai | `NXqo` | 1006 | 81 | IUD | 1750 |
| stockgoalieequipment | `yUAn` | 742 | 68 | IUD | 500 |
| exhibitiongoalieequipment | `fMVl` | 714 | 65 | IUD | 500 |
| exhibitiongoalieai | `mHuy` | 714 | 69 | IUD | 500 |
| stockplayervisorexempttable | `zkcJ` | 673 | 1 | IUD | 100 |
| stockcareerplayoffgoaliestats | `nCDp` | 469 | 13 | ro |  |
| stockawardstable | `mkrU` | 429 | 10 | ro |  |
| stockretirednumbertable | `wcSL` | 339 | 2 | I·D | 200 |
| exhibitionarena | `StEO` | 303 | 60 | IUD | 5 |
| exhibitionteams | `kOtt` | 296 | 198 | IUD | 5 |
| exhibitionteamstrategies | `EeYo` | 296 | 73 | IUD | 5 |
| stockteamcolorstable | `vzos` | 215 | 4 | IUD | 30 |
| customgoalieequipment | `yrDg` | 202 | 65 | IUD | 250 |
| customgoalieai | `GezY` | 202 | 69 | IUD | 250 |
| customteamjerseys | `PuTc` | 165 | 29 | ·U· |  |
| stocktweets | `MSRy` | 127 | 7 | ro |  |
| exhibitionfreeagents | `VoEQ` | 97 | 1 | IUD | 5000 |
| stockseasonteamstats | `czBw` | 60 | 15 | ro |  |
| customarena | `Usqf` | 55 | 60 | ·U· |  |
| customteamstrategies | `IEZE` | 55 | 73 | ·U· |  |
| customteams | `SvDE` | 55 | 198 | ·U· |  |
| customrostertable | `eGsL` | 44 | 94 | IUD | 2000 |
| stockplayoffteamstats | `cyZi` | 32 | 12 | ro |  |
| stockteamsalaryextras | `Teix` | 31 | 76 | ·U· |  |
| stockleagues | `fsLK` | 26 | 5 | ro |  |
| customfreeagents | `BeQk` | 0 | 2 | IUD | 3200 |

**`entry_draft.db`** — 10 tables, 38,784 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 7805 | 3 |
| exhibitionrostertable | `eGlu` | 7471 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7350 | 102 |
| exhibitionskaterai | `dSvy` | 6621 | 81 |
| exhibitionskaterequipment | `xmja` | 6621 | 33 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 729 | 69 |
| exhibitiongoalieequipment | `fMVl` | 729 | 65 |
| exhibitionfreeagents | `VoEQ` | 125 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`expansion_draft.db`** — 10 tables, 38,782 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 7811 | 3 |
| exhibitionrostertable | `eGlu` | 7471 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7347 | 102 |
| exhibitionskaterai | `dSvy` | 6621 | 81 |
| exhibitionskaterequipment | `xmja` | 6621 | 33 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 726 | 69 |
| exhibitiongoalieequipment | `fMVl` | 726 | 65 |
| exhibitionfreeagents | `VoEQ` | 126 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`initial_experience.db`** — 39 tables, 65,373 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8617 | 3 |
| exhibitionplayerbiotable | `ubPc` | 8131 | 102 |
| exhibitionrostertable | `eGlu` | 7568 | 94 |
| exhibitionskaterequipment | `xmja` | 7322 | 33 |
| exhibitionskaterai | `dSvy` | 7322 | 81 |
| stockteamequipmenttable | `hHbv` | 4797 | 21 |
| ahl32schedule | `jeaU` | 1313 | 6 |
| nhl32schedule | `AafV` | 1313 | 6 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| nhlschedule | `Smhi` | 1272 | 6 |
| nhlfutureschedule | `DEyb` | 1272 | 6 |
| favoriteteamschedule | `qiwI` | 1272 | 6 |
| ahl31schedule | `eTip` | 1272 | 6 |
| ahlschedule | `NKJA` | 1116 | 6 |
| stockteamjerseys | `xjgw` | 989 | 29 |
| echlschedule | `pGMV` | 972 | 6 |
| stockplayervisorexempttable | `zkcJ` | 815 | 1 |
| exhibitiongoalieequipment | `fMVl` | 809 | 65 |
| exhibitiongoalieai | `mHuy` | 809 | 69 |
| whlschedule | `DZTk` | 792 | 6 |
| stockgoalieequipment | `yUAn` | 756 | 68 |
| ohlschedule | `jEiu` | 680 | 6 |
| qmjhlschedule | `cfSy` | 612 | 6 |
| finnishschedule | `QQBR` | 450 | 6 |
| germanschedule | `hxjN` | 364 | 6 |
| czechschedule | `WQzR` | 364 | 6 |
| swedish2ndschedule | `TJaw` | 364 | 6 |
| swedishschedule | `TemL` | 364 | 6 |
| stockretirednumbertable | `wcSL` | 339 | 2 |
| austrianschedule | `yvJR` | 324 | 6 |
| exhibitionarena | `StEO` | 303 | 60 |
| swissschedule | `vlni` | 300 | 6 |
| exhibitionteams | `kOtt` | 296 | 198 |
| exhibitionteamstrategies | `EeYo` | 296 | 73 |
| stockteamcolorstable | `vzos` | 215 | 4 |
| championsschedule | `fFlH` | 125 | 6 |
| exhibitionfreeagents | `VoEQ` | 104 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |
| spenglerschedule | `mYII` | 11 | 6 |

**`threes_campaign.db`** — 10 tables, 38,290 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 7699 | 3 |
| exhibitionrostertable | `eGlu` | 7391 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7256 | 102 |
| exhibitionskaterai | `dSvy` | 6542 | 81 |
| exhibitionskaterequipment | `xmja` | 6542 | 33 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 714 | 69 |
| exhibitiongoalieequipment | `fMVl` | 714 | 65 |
| exhibitionfreeagents | `VoEQ` | 99 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`eng_us.db`** — localization: LanguageStrings `GJCv`, 33,396 strings.


### NHL 19 — PS4

`nhl_game.db` — **111 tables**, **111,145 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 15 | 41,857 | 15 |
| custom* (Creation Zone content) | 16 | 7,387 | 16 |
| stock* (shipped reference/appearance) | 17 | 41,262 | 7 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 34 | 0 | 34 |
| *schedule (league calendars) | 25 | 14,719 | 19 |
| text/name pools | 2 | 5,893 | 0 |
| other / misc | 2 | 27 | 0 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 16419 | 24 | ro |  |
| exhibitionplayers | `ZBac` | 8581 | 3 | IUD | 4800 |
| exhibitionrostertable | `eGlu` | 8328 | 94 | IUD | 7000 |
| stockcareerplayoffplayerstats | `OAqO` | 7365 | 21 | ro |  |
| exhibitionplayerbiotable | `ubPc` | 7136 | 106 | IUD | 3700 |
| exhibitionskaterequipment | `xmja` | 6389 | 39 | IUD | 3000 |
| exhibitionskaterai | `dSvy` | 6389 | 82 | IUD | 3000 |
| stockteamequipmenttable | `hHbv` | 5908 | 21 | IU· | 4000 |
| stockcareermiscstats | `NPzU` | 4055 | 7 | ro |  |
| customteamequipmenttable | `oEDN` | 2145 | 21 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 2093 | 20 | ro |  |
| stockteamjerseys | `xjgw` | 1532 | 30 | IUD | 500 |
| exhibitiondraftpick | `qHav` | 1302 | 5 | ·U· |  |
| customplayerbiotable | `KdqM` | 1214 | 106 | IUD | 2000 |
| customplayers | `Myas` | 1214 | 3 | IUD | 2000 |
| customskaterequipment | `kVlL` | 1009 | 39 | IUD | 1750 |
| customskaterai | `NXqo` | 1009 | 82 | IUD | 1750 |
| stockgoalieequipment | `yUAn` | 998 | 68 | IUD | 700 |
| stockplayervisorexempttable | `zkcJ` | 825 | 1 | IUD | 100 |
| stockcareerplayoffgoaliestats | `nCDp` | 788 | 13 | ro |  |
| exhibitiongoalieequipment | `fMVl` | 747 | 65 | IUD | 700 |
| exhibitiongoalieai | `mHuy` | 747 | 69 | IUD | 700 |
| exhibitionresortskaterextras | `fuoG` | 552 | 35 | IUD | 2000 |
| stockawardstable | `mkrU` | 427 | 10 | ro |  |
| exhibitionarena | `StEO` | 426 | 65 | IUD | 100 |
| exhibitionteams | `kOtt` | 419 | 182 | IUD | 100 |
| exhibitionteamstrategies | `EeYo` | 419 | 73 | IUD | 100 |
| stockretirednumbertable | `wcSL` | 355 | 2 | I·D | 200 |
| stockteamcolorstable | `vzos` | 217 | 4 | IUD | 30 |
| exhibitionresortgoalieextras | `WnSt` | 209 | 11 | IUD | 500 |
| customgoalieequipment | `yrDg` | 205 | 65 | IUD | 250 |
| customgoalieai | `GezY` | 205 | 69 | IUD | 250 |
| exhibitionfreeagents | `VoEQ` | 169 | 1 | IUD | 5000 |
| customteamjerseys | `PuTc` | 165 | 30 | ·U· |  |
| stocktweets | `MSRy` | 127 | 7 | ro |  |
| stockseasonteamstats | `czBw` | 61 | 15 | ro |  |
| customarena | `Usqf` | 55 | 65 | ·U· |  |
| customteamstrategies | `IEZE` | 55 | 73 | ·U· |  |
| customteams | `SvDE` | 55 | 182 | ·U· |  |
| exhibitionresortplayerextras | `MgUe` | 44 | 15 | IUD | 1000 |
| customrostertable | `eGsL` | 44 | 94 | IUD | 2000 |
| stockplayoffteamstats | `cyZi` | 32 | 12 | ro |  |
| stockteamsalaryextras | `Teix` | 31 | 76 | ·U· |  |
| stockleagues | `fsLK` | 29 | 5 | ro |  |
| customresortplayerextras | `OGPu` | 6 | 15 | ·U· |  |
| customresortgoalieextras | `URpJ` | 3 | 8 | ·U· |  |
| customresortskaterextras | `lIvS` | 3 | 32 | ·U· |  |
| customfreeagents | `BeQk` | 0 | 2 | IUD | 3200 |

**`entry_draft.db`** — 10 tables, 40,116 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8634 | 3 |
| exhibitionrostertable | `eGlu` | 8472 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7195 | 106 |
| exhibitionskaterai | `dSvy` | 6444 | 82 |
| exhibitionskaterequipment | `xmja` | 6444 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 751 | 69 |
| exhibitiongoalieequipment | `fMVl` | 751 | 65 |
| exhibitionfreeagents | `VoEQ` | 92 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`expansion_draft.db`** — 10 tables, 39,819 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8578 | 3 |
| exhibitionrostertable | `eGlu` | 8324 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7138 | 106 |
| exhibitionskaterai | `dSvy` | 6391 | 82 |
| exhibitionskaterequipment | `xmja` | 6391 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 747 | 69 |
| exhibitiongoalieequipment | `fMVl` | 747 | 65 |
| exhibitionfreeagents | `VoEQ` | 170 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`initial_experience.db`** — 41 tables, 72,561 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 10113 | 3 |
| exhibitionplayerbiotable | `ubPc` | 9012 | 106 |
| exhibitionrostertable | `eGlu` | 8567 | 94 |
| exhibitionskaterequipment | `xmja` | 8076 | 39 |
| exhibitionskaterai | `dSvy` | 8076 | 82 |
| stockteamequipmenttable | `hHbv` | 6688 | 21 |
| stockteamjerseys | `xjgw` | 1537 | 30 |
| ahl32schedule | `jeaU` | 1312 | 6 |
| nhl32schedule | `AafV` | 1312 | 6 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| nhlschedule | `Smhi` | 1271 | 6 |
| nhlfutureschedule | `DEyb` | 1271 | 6 |
| favoriteteamschedule | `qiwI` | 1271 | 6 |
| ahlschedule | `NKJA` | 1271 | 6 |
| stockgoalieequipment | `yUAn` | 1008 | 68 |
| echlschedule | `pGMV` | 972 | 6 |
| exhibitiongoalieequipment | `fMVl` | 936 | 65 |
| exhibitiongoalieai | `mHuy` | 936 | 69 |
| whlschedule | `DZTk` | 792 | 6 |
| stockplayervisorexempttable | `zkcJ` | 786 | 1 |
| ohlschedule | `jEiu` | 680 | 6 |
| qmjhlschedule | `cfSy` | 612 | 6 |
| finnishschedule | `QQBR` | 450 | 6 |
| exhibitionarena | `StEO` | 426 | 65 |
| exhibitionteams | `kOtt` | 419 | 182 |
| exhibitionteamstrategies | `EeYo` | 419 | 73 |
| germanschedule | `hxjN` | 364 | 6 |
| czechschedule | `WQzR` | 364 | 6 |
| swedish2ndschedule | `TJaw` | 364 | 6 |
| swedishschedule | `TemL` | 364 | 6 |
| stockretirednumbertable | `wcSL` | 353 | 2 |
| austrianschedule | `yvJR` | 324 | 6 |
| swissschedule | `vlni` | 300 | 6 |
| stockteamcolorstable | `vzos` | 217 | 4 |
| exhibitionfreeagents | `VoEQ` | 141 | 1 |
| championsschedule | `fFlH` | 125 | 6 |
| exhibitionresortplayerextras | `MgUe` | 44 | 15 |
| exhibitionresortskaterextras | `fuoG` | 40 | 35 |
| stockteamsalaryextras | `Teix` | 31 | 76 |
| spenglerschedule | `mYII` | 11 | 6 |
| exhibitionresortgoalieextras | `WnSt` | 4 | 11 |

**`threes_campaign.db`** — 10 tables, 40,119 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8634 | 3 |
| exhibitionrostertable | `eGlu` | 8472 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7196 | 106 |
| exhibitionskaterai | `dSvy` | 6445 | 82 |
| exhibitionskaterequipment | `xmja` | 6445 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 751 | 69 |
| exhibitiongoalieequipment | `fMVl` | 751 | 65 |
| exhibitionfreeagents | `VoEQ` | 92 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

### NHL 20 — PS4

`nhl_game.db` — **111 tables**, **111,145 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 15 | 41,857 | 15 |
| custom* (Creation Zone content) | 16 | 7,387 | 16 |
| stock* (shipped reference/appearance) | 17 | 41,262 | 7 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 34 | 0 | 34 |
| *schedule (league calendars) | 25 | 14,719 | 19 |
| text/name pools | 2 | 5,893 | 0 |
| other / misc | 2 | 27 | 0 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 16419 | 24 | ro |  |
| exhibitionplayers | `ZBac` | 8581 | 3 | IUD | 4800 |
| exhibitionrostertable | `eGlu` | 8328 | 94 | IUD | 7000 |
| stockcareerplayoffplayerstats | `OAqO` | 7365 | 21 | ro |  |
| exhibitionplayerbiotable | `ubPc` | 7136 | 106 | IUD | 3700 |
| exhibitionskaterequipment | `xmja` | 6389 | 39 | IUD | 3000 |
| exhibitionskaterai | `dSvy` | 6389 | 82 | IUD | 3000 |
| stockteamequipmenttable | `hHbv` | 5908 | 21 | IU· | 4000 |
| stockcareermiscstats | `NPzU` | 4055 | 7 | ro |  |
| customteamequipmenttable | `oEDN` | 2145 | 21 | ·U· |  |
| stockcareerseasongoaliestats | `USfF` | 2093 | 20 | ro |  |
| stockteamjerseys | `xjgw` | 1532 | 30 | IUD | 500 |
| exhibitiondraftpick | `qHav` | 1302 | 5 | ·U· |  |
| customplayerbiotable | `KdqM` | 1214 | 106 | IUD | 2000 |
| customplayers | `Myas` | 1214 | 3 | IUD | 2000 |
| customskaterequipment | `kVlL` | 1009 | 39 | IUD | 1750 |
| customskaterai | `NXqo` | 1009 | 82 | IUD | 1750 |
| stockgoalieequipment | `yUAn` | 998 | 68 | IUD | 700 |
| stockplayervisorexempttable | `zkcJ` | 825 | 1 | IUD | 100 |
| stockcareerplayoffgoaliestats | `nCDp` | 788 | 13 | ro |  |
| exhibitiongoalieequipment | `fMVl` | 747 | 65 | IUD | 700 |
| exhibitiongoalieai | `mHuy` | 747 | 69 | IUD | 700 |
| exhibitionresortskaterextras | `fuoG` | 552 | 35 | IUD | 2000 |
| stockawardstable | `mkrU` | 427 | 10 | ro |  |
| exhibitionarena | `StEO` | 426 | 65 | IUD | 100 |
| exhibitionteams | `kOtt` | 419 | 182 | IUD | 100 |
| exhibitionteamstrategies | `EeYo` | 419 | 73 | IUD | 100 |
| stockretirednumbertable | `wcSL` | 355 | 2 | I·D | 200 |
| stockteamcolorstable | `vzos` | 217 | 4 | IUD | 30 |
| exhibitionresortgoalieextras | `WnSt` | 209 | 11 | IUD | 500 |
| customgoalieequipment | `yrDg` | 205 | 65 | IUD | 250 |
| customgoalieai | `GezY` | 205 | 69 | IUD | 250 |
| exhibitionfreeagents | `VoEQ` | 169 | 1 | IUD | 5000 |
| customteamjerseys | `PuTc` | 165 | 30 | ·U· |  |
| stocktweets | `MSRy` | 127 | 7 | ro |  |
| stockseasonteamstats | `czBw` | 61 | 15 | ro |  |
| customarena | `Usqf` | 55 | 65 | ·U· |  |
| customteamstrategies | `IEZE` | 55 | 73 | ·U· |  |
| customteams | `SvDE` | 55 | 182 | ·U· |  |
| exhibitionresortplayerextras | `MgUe` | 44 | 15 | IUD | 1000 |
| customrostertable | `eGsL` | 44 | 94 | IUD | 2000 |
| stockplayoffteamstats | `cyZi` | 32 | 12 | ro |  |
| stockteamsalaryextras | `Teix` | 31 | 76 | ·U· |  |
| stockleagues | `fsLK` | 29 | 5 | ro |  |
| customresortplayerextras | `OGPu` | 6 | 15 | ·U· |  |
| customresortgoalieextras | `URpJ` | 3 | 8 | ·U· |  |
| customresortskaterextras | `lIvS` | 3 | 32 | ·U· |  |
| customfreeagents | `BeQk` | 0 | 2 | IUD | 3200 |

**`entry_draft.db`** — 10 tables, 40,116 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8634 | 3 |
| exhibitionrostertable | `eGlu` | 8472 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7195 | 106 |
| exhibitionskaterai | `dSvy` | 6444 | 82 |
| exhibitionskaterequipment | `xmja` | 6444 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 751 | 69 |
| exhibitiongoalieequipment | `fMVl` | 751 | 65 |
| exhibitionfreeagents | `VoEQ` | 92 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`expansion_draft.db`** — 10 tables, 39,819 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8578 | 3 |
| exhibitionrostertable | `eGlu` | 8324 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7138 | 106 |
| exhibitionskaterai | `dSvy` | 6391 | 82 |
| exhibitionskaterequipment | `xmja` | 6391 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 747 | 69 |
| exhibitiongoalieequipment | `fMVl` | 747 | 65 |
| exhibitionfreeagents | `VoEQ` | 170 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`initial_experience.db`** — 41 tables, 72,561 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 10113 | 3 |
| exhibitionplayerbiotable | `ubPc` | 9012 | 106 |
| exhibitionrostertable | `eGlu` | 8567 | 94 |
| exhibitionskaterequipment | `xmja` | 8076 | 39 |
| exhibitionskaterai | `dSvy` | 8076 | 82 |
| stockteamequipmenttable | `hHbv` | 6688 | 21 |
| stockteamjerseys | `xjgw` | 1537 | 30 |
| ahl32schedule | `jeaU` | 1312 | 6 |
| nhl32schedule | `AafV` | 1312 | 6 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| nhlschedule | `Smhi` | 1271 | 6 |
| nhlfutureschedule | `DEyb` | 1271 | 6 |
| favoriteteamschedule | `qiwI` | 1271 | 6 |
| ahlschedule | `NKJA` | 1271 | 6 |
| stockgoalieequipment | `yUAn` | 1008 | 68 |
| echlschedule | `pGMV` | 972 | 6 |
| exhibitiongoalieequipment | `fMVl` | 936 | 65 |
| exhibitiongoalieai | `mHuy` | 936 | 69 |
| whlschedule | `DZTk` | 792 | 6 |
| stockplayervisorexempttable | `zkcJ` | 786 | 1 |
| ohlschedule | `jEiu` | 680 | 6 |
| qmjhlschedule | `cfSy` | 612 | 6 |
| finnishschedule | `QQBR` | 450 | 6 |
| exhibitionarena | `StEO` | 426 | 65 |
| exhibitionteams | `kOtt` | 419 | 182 |
| exhibitionteamstrategies | `EeYo` | 419 | 73 |
| germanschedule | `hxjN` | 364 | 6 |
| czechschedule | `WQzR` | 364 | 6 |
| swedish2ndschedule | `TJaw` | 364 | 6 |
| swedishschedule | `TemL` | 364 | 6 |
| stockretirednumbertable | `wcSL` | 353 | 2 |
| austrianschedule | `yvJR` | 324 | 6 |
| swissschedule | `vlni` | 300 | 6 |
| stockteamcolorstable | `vzos` | 217 | 4 |
| exhibitionfreeagents | `VoEQ` | 141 | 1 |
| championsschedule | `fFlH` | 125 | 6 |
| exhibitionresortplayerextras | `MgUe` | 44 | 15 |
| exhibitionresortskaterextras | `fuoG` | 40 | 35 |
| stockteamsalaryextras | `Teix` | 31 | 76 |
| spenglerschedule | `mYII` | 11 | 6 |
| exhibitionresortgoalieextras | `WnSt` | 4 | 11 |

**`proam.db`** — 10 tables, 39,819 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8578 | 3 |
| exhibitionrostertable | `eGlu` | 8324 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7138 | 106 |
| exhibitionskaterai | `dSvy` | 6391 | 82 |
| exhibitionskaterequipment | `xmja` | 6391 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 747 | 69 |
| exhibitiongoalieequipment | `fMVl` | 747 | 65 |
| exhibitionfreeagents | `VoEQ` | 170 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`threes_campaign.db`** — 10 tables, 40,119 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 8634 | 3 |
| exhibitionrostertable | `eGlu` | 8472 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7196 | 106 |
| exhibitionskaterai | `dSvy` | 6445 | 82 |
| exhibitionskaterequipment | `xmja` | 6445 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 751 | 69 |
| exhibitiongoalieequipment | `fMVl` | 751 | 65 |
| exhibitionfreeagents | `VoEQ` | 92 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`eng_us.db`** — localization: LanguageStrings `GJCv`, 37,734 strings.


### NHL 21 — PS4

`nhl_game.db` — **115 tables**, **129,056 records**, endian=little, schema meta v6. Family breakdown (tables / records / editable-tables):

| family | tables | records | editable |
|---|--:|--:|--:|
| exhibition* (active editable roster) | 15 | 44,501 | 15 |
| custom* (Creation Zone content) | 16 | 7,387 | 16 |
| stock* (shipped reference/appearance) | 19 | 46,204 | 7 |
| league* (dynasty / Be-A-GM runtime — empty in anchor) | 34 | 0 | 34 |
| *schedule (league calendars) | 27 | 14,627 | 21 |
| text/name pools | 2 | 16,310 | 0 |
| other / misc | 2 | 27 | 0 |

**Editable roster surface** (exhibition* + custom* + stock*):

| table | canon id | records | fields | edit | maxinsert(per-txn) |
|---|---|--:|--:|:--:|--:|
| stockcareerseasonplayerstats | `tsSy` | 18587 | 25 | ro |  |
| exhibitionplayers | `ZBac` | 9281 | 3 | IUD | 4800 |
| exhibitionrostertable | `eGlu` | 9090 | 94 | IUD | 7000 |
| stockcareerplayoffplayerstats | `OAqO` | 7505 | 22 | ro |  |
| exhibitionplayerbiotable | `ubPc` | 7247 | 99 | IUD | 3700 |
| stockteamequipmenttable | `hHbv` | 6925 | 21 | IU· | 4000 |
| exhibitionskaterequipment | `xmja` | 6498 | 39 | IUD | 3000 |
| exhibitionskaterai | `dSvy` | 6498 | 106 | IUD | 3000 |
| stockcareermiscstats | `NPzU` | 4189 | 7 | ro |  |
| stockcareerseasongoaliestats | `USfF` | 2236 | 21 | ro |  |
| customteamequipmenttable | `oEDN` | 2145 | 21 | ·U· |  |
| stockteamjerseys | `xjgw` | 1792 | 31 | IUD | 600 |
| exhibitiondraftpick | `qHav` | 1302 | 5 | ·U· |  |
| exhibitionresortskaterextras | `fuoG` | 1234 | 39 | IUD | 2000 |
| customplayerbiotable | `KdqM` | 1214 | 99 | IUD | 2000 |
| customplayers | `Myas` | 1214 | 3 | IUD | 2000 |
| stockgoalieequipment | `yUAn` | 1066 | 68 | IUD | 700 |
| customskaterequipment | `kVlL` | 1009 | 39 | IUD | 1750 |
| customskaterai | `NXqo` | 1009 | 106 | IUD | 1750 |
| stockrecordstable | `vHde` | 846 | 13 | ro |  |
| stockcareerplayoffgoaliestats | `nCDp` | 776 | 14 | ro |  |
| stockplayervisorexempttable | `zkcJ` | 762 | 1 | IUD | 100 |
| exhibitiongoalieequipment | `fMVl` | 749 | 65 | IUD | 700 |
| exhibitiongoalieai | `mHuy` | 749 | 71 | IUD | 700 |
| stockawardstable | `mkrU` | 485 | 11 | ro |  |
| exhibitionarena | `StEO` | 450 | 67 | IUD | 100 |
| exhibitionteamstrategies | `EeYo` | 433 | 73 | IUD | 100 |
| exhibitionteams | `kOtt` | 433 | 183 | IUD | 100 |
| exhibitionresortgoalieextras | `WnSt` | 381 | 14 | IUD | 500 |
| stockretirednumbertable | `wcSL` | 372 | 2 | I·D | 200 |
| stockteamcolorstable | `vzos` | 217 | 4 | IUD | 30 |
| customgoalieequipment | `yrDg` | 205 | 65 | IUD | 250 |
| customgoalieai | `GezY` | 205 | 71 | IUD | 250 |
| customteamjerseys | `PuTc` | 165 | 31 | ·U· |  |
| stockteamhistory | `TPhA` | 163 | 6 | ro |  |
| stocktweets | `MSRy` | 127 | 7 | ro |  |
| exhibitionfreeagents | `VoEQ` | 112 | 1 | IUD | 5000 |
| stockseasonteamstats | `czBw` | 62 | 15 | ro |  |
| customarena | `Usqf` | 55 | 67 | ·U· |  |
| customteams | `SvDE` | 55 | 183 | ·U· |  |
| customteamstrategies | `IEZE` | 55 | 73 | ·U· |  |
| exhibitionresortplayerextras | `MgUe` | 44 | 17 | IUD | 1000 |
| customrostertable | `eGsL` | 44 | 94 | IUD | 2000 |
| stockplayoffteamstats | `cyZi` | 32 | 12 | ro |  |
| stockleagues | `fsLK` | 31 | 7 | ro |  |
| stockteamsalaryextras | `Teix` | 31 | 76 | ·U· |  |
| customresortplayerextras | `OGPu` | 6 | 17 | ·U· |  |
| customresortgoalieextras | `URpJ` | 3 | 11 | ·U· |  |
| customresortskaterextras | `lIvS` | 3 | 36 | ·U· |  |
| customfreeagents | `BeQk` | 0 | 2 | IUD | 3200 |

**`entry_draft.db`** — 10 tables, 41,559 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 9279 | 3 |
| exhibitionrostertable | `eGlu` | 9098 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7246 | 99 |
| exhibitionskaterai | `dSvy` | 6497 | 106 |
| exhibitionskaterequipment | `xmja` | 6497 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 749 | 71 |
| exhibitiongoalieequipment | `fMVl` | 749 | 65 |
| exhibitionfreeagents | `VoEQ` | 111 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`expansion_draft.db`** — 10 tables, 41,559 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 9279 | 3 |
| exhibitionrostertable | `eGlu` | 9098 | 94 |
| exhibitionplayerbiotable | `ubPc` | 7246 | 99 |
| exhibitionskaterai | `dSvy` | 6497 | 106 |
| exhibitionskaterequipment | `xmja` | 6497 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 749 | 71 |
| exhibitiongoalieequipment | `fMVl` | 749 | 65 |
| exhibitionfreeagents | `VoEQ` | 111 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |

**`initial_experience.db`** — 43 tables, 74,345 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 10815 | 3 |
| exhibitionrostertable | `eGlu` | 9576 | 94 |
| exhibitionplayerbiotable | `ubPc` | 8739 | 99 |
| exhibitionskaterequipment | `xmja` | 7841 | 39 |
| exhibitionskaterai | `dSvy` | 7841 | 106 |
| stockteamequipmenttable | `hHbv` | 5747 | 21 |
| stockteamjerseys | `xjgw` | 1811 | 31 |
| ahl32schedule | `jeaU` | 1312 | 6 |
| nhl32schedule | `AafV` | 1312 | 6 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| nhlschedule | `Smhi` | 1271 | 6 |
| nhlfutureschedule | `DEyb` | 1271 | 6 |
| favoriteteamschedule | `qiwI` | 1271 | 6 |
| exhibitionresortskaterextras | `fuoG` | 1254 | 39 |
| ahlschedule | `NKJA` | 1150 | 6 |
| stockgoalieequipment | `yUAn` | 1116 | 68 |
| echlschedule | `pGMV` | 936 | 6 |
| exhibitiongoalieequipment | `fMVl` | 898 | 65 |
| exhibitiongoalieai | `mHuy` | 898 | 71 |
| stockplayervisorexempttable | `zkcJ` | 844 | 1 |
| whlschedule | `DZTk` | 748 | 6 |
| ohlschedule | `jEiu` | 680 | 6 |
| qmjhlschedule | `cfSy` | 612 | 6 |
| exhibitionarena | `StEO` | 450 | 67 |
| finnishschedule | `QQBR` | 450 | 6 |
| exhibitionteams | `kOtt` | 433 | 183 |
| exhibitionteamstrategies | `EeYo` | 433 | 73 |
| exhibitionresortgoalieextras | `WnSt` | 387 | 14 |
| stockretirednumbertable | `wcSL` | 372 | 2 |
| germanschedule | `hxjN` | 364 | 6 |
| czechschedule | `WQzR` | 364 | 6 |
| swedish2ndschedule | `TJaw` | 364 | 6 |
| swedishschedule | `TemL` | 364 | 6 |
| swissschedule | `vlni` | 300 | 6 |
| austrianschedule | `yvJR` | 270 | 6 |
| stockteamcolorstable | `vzos` | 217 | 4 |
| championsschedule | `fFlH` | 125 | 6 |
| exhibitionfreeagents | `VoEQ` | 121 | 1 |
| exhibitionresortplayerextras | `MgUe` | 44 | 17 |
| stockteamsalaryextras | `Teix` | 31 | 76 |
| spenglerschedule | `mYII` | 11 | 6 |
| worldchampschedule | `BNEz` | 0 | 6 |
| worldjuniorschedule | `ccEK` | 0 | 6 |

**`threes_campaign.db`** — 10 tables, 48,062 records (subset projection; names inferred from shared ids):

| table | canon id | records | fields |
|---|---|--:|--:|
| exhibitionplayers | `ZBac` | 10815 | 3 |
| exhibitionrostertable | `eGlu` | 9576 | 94 |
| exhibitionplayerbiotable | `ubPc` | 8739 | 99 |
| exhibitionskaterai | `dSvy` | 7841 | 106 |
| exhibitionskaterequipment | `xmja` | 7841 | 39 |
| exhibitiondraftpick | `qHav` | 1302 | 5 |
| exhibitiongoalieai | `mHuy` | 898 | 71 |
| exhibitiongoalieequipment | `fMVl` | 898 | 65 |
| exhibitionfreeagents | `VoEQ` | 121 | 1 |
| stockteamsalaryextras | `Teix` | 31 | 76 |


## 7. Expansion scope & recommendations

**GEN7 (PS3 = Xbox360, NHL 10–15 + Legacy).**
- *Players:* headroom to ~16,383 (14-bit index); ~10k ids free above the shipped ~6k. Expansion =
  block growth of the four parallel `exhibition*` player tables **in lock-step** on the shared index,
  plus `eGlu` slot rows. Feasible; bounded by the 14-bit id, not the u16 wall.
- *Teams:* **tight — 8-bit index, 255 ceiling, only ~3 free** (252 shipped). Adding clubs is the
  hardest GEN7 expansion; dynasty `leagueteams` is tighter still (7-bit/127). Any "add expansion team"
  feature must treat 255 as the wall and audit every 8-bit teamid foreign key.
- *Appearance/stock:* heads are 16-bit (roomy); jerseys/colors/retired-numbers grow freely within u16.
- *Every table is packed* → an insert engine must do block-growth + downstream table shift + header
  `db_size`/CRC fix-ups (already the Phase 4.4 capacity-normalization path).

**GEN8 (PS4, NHL 15–21).**
- Player index widened to **16-bit (65,535)** and roster key to 15-bit (32,767); PS4 ships up to
  ~9–11k players, so the practical ceiling is far higher than GEN7. Team index is **10-bit (1,023)** —
  the 255 GEN7 wall is gone.
- `league*` shrinks across the run (59→34 tables NHL 15→21) as modes change; `custom*` grows (Creation
  Zone). PS4 remains the same `exhibition/stock/league` model, so a GEN7 editor generalizes with
  wider id fields + LE byte order.
- *Pre-allocated slack (§2):* PS4 inserts fill reserved slots without block growth until the
  reservation is exhausted — a simpler insert path than GEN7's mandatory block-growth, but the
  reservation itself (and the u16 wall / id width) is still the ceiling.

**Cross-cutting.** The binding limit for expansion is almost always the **key-field bit-width**, not
`max_records` and not `maxinsert`. Before shipping any "add N players/teams" capability, gate on: (a)
key id space, (b) every foreign-key field's width that references that id, (c) block-growth +
CRC/size normalization, (d) round-trip + in-game verification per (title, platform) — never assume a
neighbor's layout (CLAUDE.md rule 4).

---
*Generated from `anchors/` on 2026-06-30. Full machine-readable inventory:
[`data/anchor-tables-full.csv`](data/anchor-tables-full.csv).*

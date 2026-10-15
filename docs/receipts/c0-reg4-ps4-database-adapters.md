# C0-REG4 — seven PS4 database adapters — receipt

> **Corrected by [C0-REG4-R1](c0-reg4-r1-attribution-and-verification.md) (2026-09-15).** Two
> statements below are superseded: (1) `nhl19-ps4` and `nhl20-ps4` are **not** counted as
> measured-on-own-anchor coverage — their anchor bytes cannot be attributed to either title, so
> both adapters now carry every measurement as a candidate and establish nothing (coverage
> **19 attributed + 2 registered-representation-only**, not 21); (2) the NHL 20 Threes-save
> disagreement is recorded without a cause — the "different database generation" reading in §A.2
> and §F.1 is withdrawn. Metadata XML pins are now LF-normalised text digests (§F.7 resolved), and
> the candidate is an exact (autocrlf-free) export. Hashes in §D.1 are superseded by R1 §D.

**Package:** register `nhl15-ps4`, `nhl16-ps4`, `nhl17-ps4`, `nhl18-ps4`, `nhl19-ps4`, `nhl20-ps4`
and `nhl21-ps4` from each target's own committed evidence; reuse the verification mechanics where
they apply and keep PS4-specific ones apart; maintain the cross-game asset inventory; validate on an
isolated candidate and report.
**Baseline:** `c3f4181` (`feat(canonical): asset-output-profile/4 composite identities and resolution
chains`), branch `feat/phase5-asset-tooling`.
**Date:** 2026-09-15.

**Result.** Seven partial `target-adapter/2` files, each measured on its own PS4 anchor and snapshot
by a per-target module (`crates/tdb-canonical/src/target_ps4_tests.rs`; 79 named cases, 366 →
**445** on the isolated candidate, clippy `-D warnings` and rustfmt clean). Nothing was carried from
a gen7 adapter: the PS4 titles are little-endian, their main DB is **not packed** (so capacities are
the main anchor's own directory, `main-db-max-records`, and the ROSTER save's directory is a
*separate* measurement — made on NHL 16–19 through the committed PS4 codec, **blocked** on NHL 15
whose ROSTER `DATA` that codec does not decode, **missing-fixture** on NHL 20/21 which commit no
ROSTER save), their ratings are 7-bit `0..99` with the identity transform, their main-team pointer
is 8, 9 or 10 bits wide, and NHL 16's team table has no field named for the full team name. Two
evidence gaps were found and are recorded, not resolved: the committed **NHL 19 and NHL 20 PS4
anchors are byte-identical**, and NHL 18–21 carry 44 un-rostered main-team pointers outside the
team key set. Uncommitted; index empty; nothing pushed.

---

## A. Per-target registration

### A.1 Identities (measured through `tdb_core` / `tdb_savedata` at HEAD; pinned by SHA-256)

| Target | Main anchor `nhl_game.db` sha256 / bytes | `nhl_game-meta.xml` sha256 (working-tree bytes) | Snapshot | Tables / fields |
|---|---|---|---|---|
| `nhl15-ps4` | `ae9d1d56…` / 16 981 156 | `d9cf1be4…` | `ps4-nhl15.json` | 148 / 3 093 |
| `nhl16-ps4` | `7d95f1db…` / 18 456 008 | `6e497b08…` | `ps4-nhl16.json` | 135 / 3 519 |
| `nhl17-ps4` | `de48b02b…` / 17 798 544 | `f4ddcd4b…` | `ps4-nhl17.json` | 118 / 3 180 |
| `nhl18-ps4` | `65eaa6e8…` / 18 171 964 | `b4d3c39c…` | `ps4-nhl18.json` | 117 / 3 248 |
| `nhl19-ps4` | `265e4a68…` / 10 347 860 | `c67f4bfd…` | `ps4-nhl19.json` | 111 / 3 156 |
| `nhl20-ps4` | **`265e4a68…`** / 10 347 860 — **byte-identical to NHL 19's** | **`c67f4bfd…`** — identical | `ps4-nhl20.json` | 111 / 3 156 |
| `nhl21-ps4` | `90ee1ec4…` / 11 209 672 | `0b262e4a…` | `ps4-nhl21.json` | 115 / 3 265 |

All seven parse as little-endian TDB v6 with header CRC and size valid. The metadata sha256 pins
follow the convention of the earlier modules and the anchors manifest (the bytes as checked out; the
XML blobs carry CRLF, so these are stable on this checkout and are not the raw-blob hashes — a
pre-existing convention, noted, not changed here).

### A.2 Capacities — the main anchor's own directory (`main-db-max-records`), `max` / `current`

| Target | `ubPc` | `ZBac` | `eGlu` | `dSvy` = `xmja` | `mHuy` = `fMVl` | `kOtt` |
|---|---|---|---|---|---|---|
| `nhl15-ps4` | 7034 / 5555 | 8948 / 5984 | 10619 / 5523 | 6479 / 4996 | 855 / 559 | 246 / 246 (packed) |
| `nhl16-ps4` | 7988 / 5632 | 9955 / 6087 | 10879 / 5532 | 7331 / 5066 | 957 / 566 | 252 / 252 (packed) |
| `nhl17-ps4` | 9428 / 6128 | 10935 / 6701 | 11714 / 6430 | 8236 / 5527 | 1192 / 601 | 256 / 256 (packed) |
| `nhl18-ps4` | 10371 / 7236 | 11825 / 7735 | 13502 / 7373 | 9143 / 6522 | 1228 / 714 | 301 / 296 |
| `nhl19-ps4` | 12714 / 7136 | 14913 / 8581 | 15567 / 8328 | 11078 / 6389 | 1636 / 747 | 519 / 419 |
| `nhl20-ps4` | 12714 / 7136 | 14913 / 8581 | 15567 / 8328 | 11078 / 6389 | 1636 / 747 | 519 / 419 |
| `nhl21-ps4` | 12439 / 7247 | 15615 / 9281 | 16576 / 9090 | 10841 / 6498 | 1598 / 749 | 533 / 433 |

All 56 blocks: `source = "main-db-max-records"`, `measured`, provenance = the main anchor path + the
snapshot table + the target's `capacities_are_the_directory_of_the_committed_main_anchor` case + the
save case; every note says *not a demonstrated runtime limit* and *NOT a substitute* for the save's
own capacity. The figures coincide with the roster-capacity audit's PS4 table; the adapters cite the
anchor and the case, not the audit.

**Save capacity — a separate measurement per target, never substituted:**

| Target | Committed save the codec sees | Outcome |
|---|---|---|
| `nhl15-ps4` | `savefiles/84f8102f - ROSTER/DATA` (739 913 B, `17bf9bfa…`) | **BLOCKED** — `is_ps4_save` false, `ps4_extract_roster` → `no LZ4 roster record in DATA` (save-matrix: `unknown,unsupported`). No new codec written. |
| `nhl16-ps4` | `84f8102f - ROSTER/DATA` (1 221 291 B, `7a01bf60…`) → LE TDB 3 393 888 B (`62d28ddf…`), 36 tables, `size_ok` true | decoded; `max_records` AND `current_records` equal the main anchor's on all 36 tables |
| `nhl17-ps4` | `RosterCustom0/DATA` (1 448 394 B, `70143a1d…`) → 5 603 080 B (`a4038e0f…`), 48 tables, `size_ok` **false** (re-serialized) | decoded; `max` and `current` equal the main anchor's on all 48 |
| `nhl18-ps4` | `RosterCustom0/DATA` (1 514 666 B, `32b5d818…`) → 5 999 830 B (`25763d39…`), 52 tables, `size_ok` false | decoded; `max` and `current` equal on all 52 |
| `nhl19-ps4` | `RosterCustom0/DATA` (1 734 662 B, `1f2f83ee…`) → 7 557 054 B (`36305e73…`), 54 tables, `size_ok` false | decoded; `max` equal on the roster core, **populations differ** (community roster: `ZBac` 9801 vs 8581) |
| `nhl20-ps4` | none of ROSTER kind (`BeAGM0`, `BeAPro0`, `Profile0`, `Season0`, `ThreeOnThree0`) | **MISSING-FIXTURE**; `ThreeOnThree0` (11 tables, a Threes projection, `615a107d…`) **disagrees with the anchor on `max_records` and `current_records` for every roster-core table** (e.g. `ubPc` 11201/7501 vs 12714/7136) — observation only, used for nothing; cause not attributed (R1) |
| `nhl21-ps4` | none of ROSTER kind | **MISSING-FIXTURE**; `ThreeOnThree0` (`70d379f0…`) carries the anchor's `max_records` on all eight roster-core tables and other populations — observation only |

### A.3 Semantics measured on each anchor (module `target_ps4_tests::<game>_ps4`)

| Fact | 15 | 16 | 17 | 18 | 19 | 20 | 21 |
|---|---|---|---|---|---|---|---|
| Birth month/day stored ranges → origin | 0..11 / 0..30 → `index-origin-0` on all seven (NHL 15's metadata declares 0..15 / 0..31 like gen7; NHL 16+ declare 0..11 / 0..30; probed either way) |||||||
| Component offsets day / month / year | 1159/1177/816 | 1342/1377/824 | 1431/1468/864 | 1460/1497/856 | 1421/1458/808 | same as 19 | 1348/1384/848 |
| Year stored range | 61..98 | 61..99 | 39..100 | 35..101 | 2..103 | 2..103 | 0..104 |
| Companion partition skaters / goalies (key `wBIz` = `game_id`, 14 → 16 bits) | 4996/559 | 5066/566 | 5527/601 | 6522/714 | 6389/747 | 6389/747 | 6498/749 |
| Position codes | {0,1,2,3} / {4} on all seven; no unassigned row |||||||
| `ubPc.dXSB` offset / width / declared range | 48/8/-1..254 | 48/8/-1..254 | 80/8/-1..254 | 1090/9/-1..510 | 1042/10/-1..1022 | same | 1080/10/-1..1022 |
| Main-team encoding | slot team + 1 on all seven; no rostered row stores 0; the all-ones pattern is stored nowhere |||||||
| Rostered / un-rostered storing 0 / un-rostered storing non-zero | 5107 / 447 / **1 (pattern 135)** | 5110 / 522 / 0 | 5882 / 246 / 0 | 6888 / 304 / **44 (501 ×22, 502 ×22)** | 6898 / 194 / **44** | same | 7055 / 148 / **44** |
| `same` (adjacent-code second slot) | 0 | 0 | 4 | 0 | 17 | 17 | 45 |
| `kOtt.vfEq` width / keys / largest key | 8 / 246 / 245 | 8 / 252 / 251 | 9 / 256 / 399 | 9 / 296 / 399 | 10 / 419 / 679 | same | 10 / 433 / 696 |
| Handedness `GRkp` offset, {0, 1} | 1181 {3797, 1758} | 1381 {3781, 1851} | 1472 {4031, 2097} | 1501 {4798, 2438} | 1462 {4772, 2364} | same | 1388 {4812, 2435} |
| `dSvy.eWUF` `c_overall` offset, stored | 32, 36..99 | 32, 36..99 | 64, 36..99 | 64, 36..99 | 72, 36..99 | same | 96, 36..99 |
| `dSvy.AdEb` `c_speed` offset, stored | 203, 52..95 | 203, 52..99 | 235, 52..99 | 235, 52..96 | 243, 52..99 | same | 316, 52..98 |
| `team.name` field / offset / width; records 0, 1 | `YmkJ` `fullname` 352/400; "Anaheim Ducks®" / "Winnipeg Jets™" | **none** (`shortname`, `artabbr`, `nickname`, `abbrname` only) | `AUsv` `teamname` 1512/320 | `AUsv` 3312/320 | `AUsv` 2768/256 | same | `AUsv` 2672/256 |
| `team.name` round trip | passed (50-byte budget) | omission `unresolved-semantics` | passed (40) | passed (40) | passed (32) | passed (32) | passed (32) |

**Main-team pointer — kept visible.** On NHL 18–21, 44 un-rostered bio rows store patterns 501 /
502 (decoded team codes 500 / 501), which exceed the largest `kOtt.vfEq` key on each of those
anchors (399 / 679 / 679 / 696): containment fails for them. On NHL 15 one un-rostered row stores
135 (a code inside the key range). Pattern 0 is stored on un-rostered rows only and on no rostered
row on all seven, so `literal-code 0 = absent` is established as measured, with every note stating
the exceptions by pattern and count; the parent stays a candidate and the NHL 18–21 candidate notes
record the containment failure as evidence *against* it. The PS4 variant of the codec-order helper
returns the un-rostered non-zero rows by pattern instead of refusing the anchor.

**Ratings.** Both mapped fields are 7-bit `0..99` on every anchor and store values above 63 and up
to 99, so neither 6-bit regime (offset 36 or 50) can describe them and a maximum of 99 leaves no room
for a positive offset. The transform is `identity`, **derived** from those measured premises plus
the ratings audit's `PS4 7-bit … 0` row (asserted to be present) and the engine's metadata-driven
`(0, 99) → offset 0` classification (`attributes::rating_offset`, asserted to be present). The named
case `ratings_write_is_inverse_of_read_ps4_direct` ran on `nhl15/ps4` only, so `inverse_verified`
is true on `nhl15-ps4` and **false** on the other six; qualification is `implemented` on all seven
(the case reads the first rating cell, not these fields). Human range 0..99 is derived, never
declared.

**`team.name`.** NHL 15 maps `fullname` (the descriptor the engine's `team_name_source` reads).
NHL 17–21 map the field EA names `teamname` (`kOtt.AUsv`), derived from that name; `nickname`
stores identical text on records 0/1 and is not mapped. **Finding, not repaired:** the HEAD engine
resolves a team's display name by the `fullname` descriptor and reads an empty string where it is
absent — so it reads nothing on NHL 16–21 PS4 today. NHL 16 has neither field: `team.name` is an
`[[omission]]` with `reason = "unresolved-semantics"` listing the four string fields and the
observation; nothing is mapped.

### A.4 Coverage per target

| Target | Mapped / total fields | Mappings / omissions | Established / unknown / candidate | Blocked |
|---|---|---|---|---|
| `nhl15-ps4` | 10 / 3 093 | 8 / 0 | 44 / 30 / 6 | `player.shoots`, `player.team`; save capacity |
| `nhl16-ps4` | 9 / 3 519 | 7 / 1 | 39 / 27 / 6 | `player.shoots`, `player.team`, `team.name` (omission) |
| `nhl17-ps4` | 10 / 3 180 | 8 / 0 | 44 / 30 / 6 | `player.shoots`, `player.team` |
| `nhl18-ps4` | 10 / 3 248 | 8 / 0 | 44 / 30 / 6 | `player.shoots`, `player.team` |
| `nhl19-ps4` | 10 / 3 156 | 8 / 0 | 44 / 30 / 6 | `player.shoots`, `player.team`; anchor identity gap |
| `nhl20-ps4` | 10 / 3 156 | 8 / 0 | 44 / 30 / 6 | `player.shoots`, `player.team`; anchor identity gap; save missing |
| `nhl21-ps4` | 10 / 3 265 | 8 / 0 | 44 / 30 / 6 | `player.shoots`, `player.team`; save missing |

Candidates on every adapter: `player.shoots` transform / enumeration / null_sentinel (the Legacy-
anchored enum is a candidate here, with each anchor's histogram recorded as counts), both rating
null_sentinels (a range inference over 0..99; the stored minimum on each anchor is 36 / 52), and the
`player.team` foreign key (`kOtt.vfEq`, weakened on NHL 18–21 as above).

### A.5 Evidence strength, per fact class

* **measured on this anchor**: layouts, key/updatable flags, capacities (main DB), birth origin,
  companion partition, discriminator, main-team encoding and sentinel, handedness histogram, rating
  stored ranges, team-name round trip, save decode/comparison — each by a named case that pins the
  file's SHA-256.
* **derived with target-applicable premises**: status blocks (snapshot flags + the mutation audit,
  whose matrix carries five `ps4,<game>` rows), rating transforms (see A.3), `team.name` identity
  (metadata name), `dXSB` storage regime (snapshot + the format record's offset-binary exception +
  the measured behaviour).
* **asserted candidates**: handedness codes, rating absence codes, the main-team parent.
* **blocked / missing**: NHL 15 save capacity (codec), NHL 20/21 save capacity (no fixture).
* **automated-tested**: `player.birth_date` (the `studio-domain` probe lists all seven anchors),
  `team.name` on six targets (this package's round trip).
* Nothing is game-verified; registration is not support.

---

## B. Mechanics

Reused unchanged: `companion_partition`, `position_codes_by_partition`,
`assert_discriminator_declared`, `roster_slots_by_bio_record`, `birth_date_ranges`,
`decode_stored` / `encode_stored`, `candidate_blocks`, `read_pinned`, `Target::main_snapshot`.

Generalised in `target_evidence.rs`, gen7 call sites updated with their own values so every existing
assertion keeps its meaning:

* `Target.roster_fixture / roster_fixture_sha256 / roster_payload_sha256` →
  `Target.roster: Option<RosterFixture>` — `None` is an honest "no decodable ROSTER fixture"
  (NHL 15 / 20 / 21) that makes a gen7 capacity case panic rather than run; the twelve gen7
  declarations wrap their three strings in `Some(RosterFixture { .. })`.
* `main_team_measurement(.., bit_width)` and `assert_main_team_declared(.., bit_width)` — the
  all-ones pattern and the offset-binary range top are the target's own width (gen7 passes 8;
  expected ranges unchanged: `-1..254`, `0..255`).
* `team_name_round_trip` is now a wrapper over `kott_string_round_trip(field_id, offset, width,
  ..)` with the over/at-budget probes at `capacity + 1` / `capacity` bytes;
  `assert_team_name_declared(.., byte_capacity)` (gen7 passes 50).

PS4-specific, added apart: `Ps4RosterFraming` + `Target::decoded_ps4_roster_fixture` (recognition
by `is_ps4_save`, LZ4 extraction, LE, CRC, `size_ok` as recorded per fixture),
`Target::main_db_capacity_discrepancies` + `assert_capacities_are_the_main_anchors` (the main-DB
counterpart of the roster-save checks, including the "not a substitute" wording gate),
`directory_comparison`, `codec_round_trip_over_anchor_with_unrostered_pointers`.

`tests.rs` / `snapshot_tests.rs`: the seven adapters join `COMMITTED_ADAPTERS` (21) and
`MEASURED_ON_OWN_ANCHOR` (19); a `GEN8_ADAPTERS` list gives `a_target_supported_derivation_stays_usable`
its gen8 branch (identity transform, human range 0..99, `main-db-max-records` on every capacity)
while the gen7 branch still asserts offset 36 / `roster-save-max-records`; the sentinel-candidate
total moves 37 → 58 (3 per PS4 adapter), and `an_unsupported_no_sentinel_claim_is_not_operative`
lists each PS4 adapter with its measured team sentinel.

---

## C. Cross-game asset inventory

`docs/receipts/data/c0-reg2-asset-mapping-inventory.csv` (committed at `0251b57`): two cells
updated, seven rows added; unchanged rows byte-identical.

* `assets-jerseys / nhllegacy-xbox360`: the blocker no longer says the composite kit id and the
  shared-fallback write exclusion are unrepresentable — `asset-output-profile/4` (`c3f4181`)
  represents both (`[identity.value.composite]` + `{composite-id}`; `[[resolution_chain]]` with
  owned/shared steps and shared-step write exclusion). What remains: the composite is **not unique**
  on the Legacy anchor (703 rows → 698 ids; `1_19_3`, `1_21_4`, `1_255_0` collide) so `[identity]`
  is a candidate under `ambiguity = reject`; the thumbnail chain is a candidate from working-tree
  source with no runtime evidence; the archive member, `chunklzx` and D7 are unchanged.
* `assets-logos / … all PS4 targets`: "PS4 unknown" on the database side replaced by the fact that a
  field EA names `artid` exists on `kOtt` in all seven PS4 snapshots — with no binding to any file.
* Seven `database-identities (asset-relevant, all families)` rows, one per PS4 target: what A
  established (team key width and key set, player key, main-team pointer), what is merely present
  in the snapshot (`ubPc.lOnr` `artid`, `ubPc.pPaD` `headid`, `kOtt.lOnr` `artid`, `xjgw` with its
  `teamid / style / teamartid / variant` fields and row counts), and — explicitly — that **no
  filename, member, bucket or path is inferred from any of them**; the per-family PS4 rows (GNF by
  analogy, team-name-keyed jersey filenames) are not upgraded.

No schema version, importer, archive installer or mounted-tree inspection.

---

## D. Validation

**Candidate:** `git archive c3f4181` extracted to a fresh scratch directory (1 211 files) plus the
16-file boundary below (→ 1 219 files: 8 additions); dedicated `CARGO_TARGET_DIR`; `QA_WRITE` and
`QA_EVIDENCE_ROOT` unset (verified). No manifest or lock change was needed: the PS4 codec entry
points are already exported by the committed `tdb-savedata` dev-dependency.

| Check | Result | Log (scratchpad `reg4/`) |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **445 passed · 0 failed · 0 ignored** (366 at `c3f4181`; +77 per-target cases + 2 twin cases) | `log-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `log-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-fmt.txt` |

**Candidate preservation:** every non-boundary file of the candidate equals its `c3f4181` blob
after CRLF→LF normalisation (649 files differ only by the checkout's CRLF conversion, 0 by content;
0 missing; 0 extra); every boundary file is byte-identical to the working-tree file hashed below.

Failures met on the way, none fixed by weakening: the decoded-save capacity notes lacked the "NOT a
substitute" wording the PS4 capacity gate requires (wording added); the NHL 16 ledger tuple was
miscounted (corrected to the measured `(7, 1, 39, 27, 6)`); the cross-title identity statement was
line-wrapped through the phrase the twin case looks for (rewrapped); clippy `err_expect`.

### D.1 Exact source boundary (SHA-256 of working-tree bytes; A = new, M = modified vs `c3f4181`)

```
7c4fd1d6d601416936e93862708cd294eb3c30b6b147301d1fa53262c2479c14  crates/tdb-canonical/src/target_ps4_tests.rs        A  1544 lines
256b0e1e747f650386b9481377ca6522aa00d1d75a24654e959ccfa53f2e698a  crates/tdb-canonical/src/target_evidence.rs         M  +325/−29
7d68804e193e54cab95ead18e159676956188152c13e3479ce76f6c2d53b80f2  crates/tdb-canonical/src/tests.rs                   M  +81/−5
0a79390941488fe888c5c96e769b105f8242c6d58f463efd0abc814922669c36  crates/tdb-canonical/src/target_batch_tests.rs      M  +55/−57  (RosterFixture wrap; width/capacity args)
f36126e120b86761561113f0a93304de4036575f0d84421d801505a26a5b0d5f  crates/tdb-canonical/src/snapshot_tests.rs          M  +8/−1
541d0a62f29e9f96a5d946f4b7047ed7632da31384f8e1b7dcf75122eb268806  crates/tdb-canonical/src/legacy_xbox360_tests.rs    M  +9/−6
3fe6e39d636181c51855ddccd59dbcb698091db9dc775e9526d256ce64cb584b  crates/tdb-canonical/src/nhl15_ps3_tests.rs         M  +8/−4
34b9cbcb24d8954463b038e501a679b96d6e33976121de538c7590de49e7c513  crates/tdb-canonical/src/lib.rs                     M  +2/−0   (module declaration)
fe56ca3e65adf08e2057b4e7403bb56b65c835f5fe51cdf09d1e5be88653e5d8  defs/adapters/nhl15-ps4.toml                        A  1057 lines
ddcf7de08e3e603d6257ed0325a26f4a7aec11ef45b66a2f21ab5d51b7b51fd5  defs/adapters/nhl16-ps4.toml                        A   988
401ce2f11e473a2988e7d67c60f5ab9cc0ae04b073a01a4d719eee62a7f45391  defs/adapters/nhl17-ps4.toml                        A  1057
9940b601dfd7f744cce50cc0ca36167ebf410514a7f610a0b517186d3e43061e  defs/adapters/nhl18-ps4.toml                        A  1057
0611c32fe09bd1da091f940415d93fa3019378b4b0ab9cdc12aaa1435a31f628  defs/adapters/nhl19-ps4.toml                        A  1068
8f09776c203a3c9a594a12376e6fde6c48f001fae2a0264ceafcb16ef9a3c53f  defs/adapters/nhl20-ps4.toml                        A  1062
ef985ffd79d8f891055b9a4de35c8893d31804c2077b89f49ac0e83b1e18359e  defs/adapters/nhl21-ps4.toml                        A  1047
75c3aad32b3ac88ab8e192008e87abacc8022b40b2e25d1927a872df90360045  docs/receipts/data/c0-reg2-asset-mapping-inventory.csv  M  +9/−2
```

Outside the boundary (documentation, untracked as before, not part of the candidate):
`docs/plans/project-completion-roadmap.md` (item 7 and Unmet item 3 updated), `docs/STATE-OF-THE-REPO.md`
(canonical row updated), this receipt. The adapters were produced by a scratch generator that reads
every physical number from the target's snapshot and the measured facts from the probe run
(`reg4/gen_ps4.py`, `reg4/probe-out.txt`, scratchpad only).

Live tree: HEAD `c3f4181`, index empty (`git diff --cached --stat` = 0 lines), nothing staged,
nothing pushed; the 309 unrelated modified/untracked entries untouched. Proposed boundary: one
commit, `feat(canonical): register seven measured PS4 adapters` (the 16 files above; expected 445).

---

## E. Coverage

| | Before (at `c3f4181`) | After (if REG4 is accepted) |
|---|---|---|
| Database units with a committed partial adapter | 14 / 28 (12 measured on own anchor, 2 structural) | **19 / 28 attributed** (17 measured on own anchor, 2 structural) **+ 2 registered as representation only** (`nhl19-ps4`, `nhl20-ps4`: attribution unresolved, nothing established — R1) |
| Units with no adapter | 14 | **7**: `nhl07-xbox360`, `nhl08-xbox360`, `nhl09-xbox360`, `nhl08-ps3`, `nhl09-ps3`, the NHL 07 PS3 research copy, Legacy Recompiled (no anchor, FR4) |
| Asset families | 3 / 14 committed (portraits partially populated; logos, jerseys represented) | unchanged; the inventory now separates established database identities from unproven filename bindings |
| Game-verified cells | 0 | 0 |

**Partial registration ≠ usable support.** Each PS4 adapter maps 9–10 of ~3 100–3 500 fields;
`player.shoots` and `player.team` are blocked on all seven; NHL 16 maps no team name; no
save-container round trip, deployment or in-game acceptance is claimed; the NHL 15 ROSTER container
is not decoded and NHL 20/21 have no ROSTER fixture. Usable support on any PS4 title remains C1+
work plus C9 acceptance.

---

## F. Evidence gaps discovered (recorded, not resolved)

1. **`anchors/ps4/nhl20/databases/nhl_game.db` and `nhl_game-meta.xml` are byte-identical to
   NHL 19's** (`265e4a68…` / `c67f4bfd…`; the anchors manifest lists both hashes without remark; the
   roster-capacity audit notes "NHL 20 is identical to NHL 19"). The NHL 20 `ThreeOnThree0` save the
   codec decodes disagrees with that anchor on every roster-core directory figure — **recorded
   without attributing a cause** (R1): a mode projection, a title update, an edit or a different
   capture would each produce it and none has been distinguished. Which shipping build the shared
   bytes come from is not established. **R1:** both adapters are now non-operative (every block a
   candidate); see the R1 receipt. Needs an anchor decision.
2. **Un-rostered main-team pointers outside the team key set** on NHL 18–21 (44 rows, patterns
   501/502; likely a reserved pseudo-team code, unproven) and one in-range un-rostered pointer on
   NHL 15. Weakens the `kOtt.vfEq` parent candidate; no witness exists for what the game does with
   them.
3. **NHL 15 PS4 ROSTER `DATA` is not decoded by the committed codec** (a container variant the
   `ps4.rs` codec does not recognise; FR-class, already noted by the roster-capacity audit). Save
   capacity blocked until a codec package.
4. **No ROSTER save committed for NHL 20 and NHL 21** (fixture gap for the required roster save
   workflow).
5. **Engine team-name resolution reads `fullname` only** — empty on NHL 16–21 PS4 (`teamname` /
   `nickname` carry the text). A C1 defect candidate; not repaired here.
6. **NHL 16 PS4 team table has no full-name field**; which of `nickname` / `shortname` the game
   presents is unknown.
7. The metadata-XML sha256 pins (this package and the earlier modules alike) are working-tree
   (CRLF) bytes; on an LF checkout they would not match. Pre-existing convention; flagged for the
   Windows/Linux CI hardening item (C0.2 R6).
8. `mHuy.eWUF` (`c_overall`) exists on the goalie companion on all seven PS4 titles and is not
   mapped (the canonical model's rating mappings are skater-predicated, as on gen7).

---

## G. Remaining early-generation targets

`nhl09-xbox360`, `nhl09-ps3`, `nhl08-xbox360`, `nhl08-ps3` (gen7 v1-era; NHL 09's known orphan bio
stubs and 0-indexed `dXSB` will need the as-is main-team regime and companion-presence handling),
`nhl07-xbox360` and the NHL 07 PS3 research copy (gen6 `NHL07NG.db`: no equipment companions, thin
two-level team table — a different table model, so `companion_partition` and `roster_slots_by_bio_record`
do not apply as written), and Legacy Recompiled (FR4, no anchor). Also outstanding from earlier
packages: `nhl15-xbox360` / `nhl10-xbox360` re-measurement on their own fixtures, the NHL 10 rating
regime, the RawV1 write path, and the ratings-audit reconciliation.

## H. Limits honoured

No new save codec, runtime investigation, schema version, importer, archive installer or
mounted-tree inspection; no capacity carried between the main DB and a save in either direction; no
gen7 value, transform, origin, sentinel, companion rule or capacity transferred; QA generators not
run and `docs/audits/data/**/capability.csv` untouched; headmesh work untouched; nothing committed
or pushed; unrelated working-tree changes preserved.

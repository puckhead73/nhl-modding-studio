# C0-REG1a-R1 — close Legacy evidence gaps and bind snapshot identity

**Package:** C0-REG1a-R1 · **Date:** 2026-09-14 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `819940a9fcf255221ce64bb6b0b2229942ec304a` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** The five corrections are in the working tree and verified in isolation. The seven
> capacity ceilings are no longer derived from the PS3 audit across a main-DB hash identity: all
> eight capacity blocks (the team table included) are now **measured** on the committed Xbox 360
> Legacy ROSTER fixture, decoded by the production save codec, and described as **observed directory
> capacity, not a demonstrated runtime limit**. Adapters are now bound to their cited target: the
> anchor-snapshot check refuses an adapter attributed to the wrong game/platform even when every
> physical declaration agrees (the PS3/X360 Legacy `nhlng.db` are byte-identical, so that case is
> real). On this exact target, three semantic candidates were **resolved by measurement with
> independently supported labels** — the position discriminator, the companion rule (and with them
> the ratings' applicability), and the main-team pointer's encoding and absence code — and two were
> **left blocked**: handedness (no independently supported label exists in the repository) and the
> main-team pointer's parent table (containment plus a name is not a foreign key). `team.name`'s
> read / edit / serialize / reopen behaviour is now `automated-tested` on an in-memory copy of the
> anchor.
>
> Isolated candidate from `819940a` + a 6-file overlay + 2 lock lines, `--locked --offline`, env
> unset: **`cargo test -p tdb-canonical` 213 passed · 0 failed** (205 → 213), clippy `-D warnings`
> clean, fmt clean, `tdb-core` + `tdb-savedata` suites (the seams used) 39 passed, candidate
> membership unchanged (1 189 files, 0 differing).
>
> Legacy's constraint population moves from **37 established / 37 unknown / 12 candidates** to
> **44 / 30 / 6**; registry coverage is unchanged at **3/28 database units (working tree,
> uncommitted)** and 10 / 2 965 fields. One storage regime was corrected on evidence (§4.3). Nothing
> here makes NHL Legacy Xbox 360 supported or game-verified.

---

## 1. Exact files changed (all uncommitted)

| Path | Op | Lines | What |
|---|---|---:|---|
| `defs/adapters/nhllegacy-xbox360.toml` | M | 1 009 | capacities ×8 measured on the fixture; discriminator, companion rule, ratings applicability ×2, main-team transform + sentinel established; main-team storage regime corrected; `team.name` automated-tested; handedness and main-team parent candidates annotated with what was observed; header correction note |
| `crates/tdb-canonical/src/legacy_xbox360_tests.rs` | **A** | 1 045 | seven named cases (§2–§5, §7) |
| `crates/tdb-canonical/src/snapshot_tests.rs` | M | 807 | target/snapshot/anchor identity guard; capacity check re-scoped to the file its `source` names; identity rejection case; REG1a case assertions updated |
| `crates/tdb-canonical/src/tests.rs` | M (CRLF kept) | +114 / −16 | four adapter-wide loops narrowed to the two neighbours, with Legacy's measured state asserted as the contrast (§6) |
| `crates/tdb-canonical/src/lib.rs` | M | +2 | `#[cfg(test)] mod legacy_xbox360_tests;` |
| `crates/tdb-canonical/Cargo.toml` | M | +5 | **dev-dependencies** `tdb-core`, `tdb-savedata` (commented) |
| `Cargo.lock` | M | +2 | `tdb-canonical` dependencies gain `"tdb-core"`, `"tdb-savedata"` — the only lock change this package makes (the live lock also carries the pre-existing unrelated drift) |
| `docs/receipts/c0-reg1a-legacy-xbox360-adapter.md` | M | +9 | labelled "superseded in part" note pointing here; body untouched |
| `docs/plans/project-completion-roadmap.md` · `docs/STATE-OF-THE-REPO.md` | M | 3 lines | C0 item 7 / §7 population reporting (§8) |
| `docs/receipts/c0-reg1a-r1-legacy-evidence-correction.md` | A | — | this receipt |

**Overlay identities** (SHA-256, as verified in §7):

```
5f098bef…f980  defs/adapters/nhllegacy-xbox360.toml
0cb25ec3…13d3  crates/tdb-canonical/Cargo.toml
3449e9c2…2522  crates/tdb-canonical/src/lib.rs
c373f299…94c3  crates/tdb-canonical/src/tests.rs
c2034e6f…9e73  crates/tdb-canonical/src/snapshot_tests.rs
209cee7f…c0f7  crates/tdb-canonical/src/legacy_xbox360_tests.rs
```

**Dependency boundary.** The measurements need two things HEAD's `tdb-canonical` dev-dependencies
did not reach: the TDB parser (`tdb_core::TdbFile` — parse, `read_u64`, `read_inline_str`,
`set_inline_str`, `recompute_crcs`, `serialize`) and the save codec (`tdb_savedata::SaveContainer`
— header, checksums, `extract_db`). Both are workspace crates already in the lock with no new
third-party surface; `tdb-savedata` brings `flate2`/`lz4_flex`, already locked. No production
`studio-domain` probe was exercised, so nothing from that crate's working-tree changes was imported;
the metadata names came from `tdb_qa::schema::snapshot`, already a dev-dependency.

## 2. Capacity — read from the fixture (task 1)

**Fixture identity.** `anchors/xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052`,
2 456 120 bytes, **sha256 `65a04268a2d1321ee495bd0c2d1509748c2d12ec3495a038d7a567f616ad7d21`**. It is
already outside its STFS wrapper: byte 0 is the `RosterFile` container magic.

**Framing, as decoded by `tdb_savedata`.** magic `RosterFile` → `SaveType::Roster`; version word 4
with a zlib body at `0x30` → `ContainerVersion::ZlibV4` (48-byte header); `checksum_a` and
`checksum_b` both recompute (`checksums_valid() == true`); declared `uncompressed_size` 2 456 076,
`compressed_size` 722 116, `field_20 = 1`. The inflated payload carries a 4-byte trailer past the
inner DB's own `db_size`; `extract_db` trims to **2 456 072 bytes, sha256
`aeadd37cb4a74c39ec3972a161f83e5abf37cf3cf5c40c6eadad0160877796db`**. `tdb_core` parses it: big
endian, **39 tables**, header CRC and size both valid.

**Per-table directory, as stored** (the eight tables the adapter declares; the case
`capacities_are_the_observed_directory_of_the_committed_roster_fixture` asserts each):

| Table | `current_records` | `max_records` | Main anchor `current_records` | REG1a (PS3-derived) ceiling |
|---|---:|---:|---:|---:|
| `ubPc` | 5 632 | **7 988** | 5 632 | 7 988 |
| `ZBac` | 6 087 | **9 955** | 6 087 | 9 955 |
| `eGlu` | 5 532 | **10 879** | 5 532 | 10 879 |
| `dSvy` | 5 066 | **7 331** | 5 066 | 7 331 |
| `xmja` | 5 066 | **7 331** | 5 066 | 7 331 |
| `mHuy` | 566 | **957** | 566 | 957 |
| `fMVl` | 566 | **957** | 566 | 957 |
| `kOtt` | 252 | **252** | 252 | (unknown) |

What changed is not the numbers but what they rest on and what they claim:

- **Source.** Every block now cites the fixture path, the named case and the `save-matrix.csv` row,
  at `confidence = "measured"`, `source = "roster-save-max-records"`. The PS3 audit, the manifest
  hash identity and the "equivalence bridge" derivation are gone from these blocks (the identity
  remains cited where it is the right premise, e.g. the companion rule's corroboration).
- **Claim.** Each note states that `ceiling_records` is *the observed directory capacity of that
  fixture, not a demonstrated runtime limit*; the case asserts that phrase is present on all eight.
  The stronger claim ("the game grows to this") is recorded as NOT ESTABLISHED in the notes; the
  contract has one capacity block per table, so it could not be kept as a separate unknown block.
- **Population.** The save's `current_records` — the fixture's own — and, separately, the
  **observation** that it equals the main anchor's for all 39 tables the save carries. The case
  compares them and says so; nothing assumes it. The `snapshot_tests` capacity check no longer
  requires a `roster-save-max-records` population to equal the main DB's (§3.2); only a
  `main-db-max-records` figure is held to the main DB, and any source is held to "ceiling ≥ main
  population".
- **`kOtt`.** Previously unknown. The fixture's directory holds 252/252 — packed in the save as in
  the main DB — so it is now established at **252 / 252 with no headroom**, worded so that nothing
  suggests a 253rd team is possible.

**Rejection evidence** (`a_capacity_that_is_not_the_fixtures_is_reported`): a ceiling one below
(7 987) or above (8 000) the stored `max_records`, a population that is not the save's, a block whose
provenance omits the fixture, a block attributed to `main-db-max-records`, and a table the save does
not carry are each reported by table with the stored figure. The PS3 audit's figures coincide with
this fixture's, so a value copied from there would pass the numeric check — it is caught by the
provenance and source checks instead, which is why those exist.

## 3. Target / snapshot / anchor identity (task 2)

### 3.1 Guard

`snapshot_tests::anchor_snapshot` now asserts, after selecting the anchor from the
`physical_snapshot` filename, that the parsed snapshot's `(platform, game, db)` are the selected
anchor's and that the anchor lives at `anchors/<platform>/<game>/databases/<db>`; it already
asserted that the cited JSON is byte-for-byte the rendering of that anchor. `discrepancies` then
checks **identity before layout**:

- `adapter.target == "<game>-<platform>"` of the snapshot checked;
- `adapter.physical_snapshot == "docs/audits/data/schema/<platform>-<game>.json"` of the snapshot checked.

Every physical check REG1a introduced is preserved (endian, generation, tdb_version, totals, table
names, per-field offset/width/range/kind/meta type/key/updatable, distinct mapped count, ceilings).

### 3.2 One check re-scoped

The REG1a rule "a `roster-save-max-records` population must equal the main DB's `current_records`"
was the assumption Codex named, and is removed. Consequence stated plainly: the NHL 15 and NHL 10
adapters' roster-save populations are **no longer verified by any test** — they restate the PS3
audit's table and will be verified when their own fixtures are read (out of this package's
one-target scope). Legacy's are verified against its save (§2).

### 3.3 Rejection evidence

`an_adapter_attributed_to_the_wrong_target_is_reported_even_when_the_layout_agrees`:

| Case | Physical disagreement | Reported |
|---|---|---|
| Legacy adapter, `target = "nhllegacy-ps3"`, Xbox 360 snapshot | none (the two `nhlng.db` are byte-identical) | exactly one finding: `target: adapter nhllegacy-ps3 vs snapshot nhllegacy/xbox360 rendered from nhlng.db` |
| Legacy adapter, `physical_snapshot` → `ps3-nhllegacy.json` (PS3 anchor selected and parsed) | none | exactly one finding: `target: adapter nhllegacy-xbox360 vs snapshot nhllegacy/ps3 …` |
| `target = "nhl15-xbox360"`, Legacy snapshot | — | one identity finding |
| `target = "nhl15-xbox360"` + NHL 15 snapshot | totals, offsets, … | zero identity findings, layout findings incl. `fields_total 2965 vs anchor 2969` |
| Xbox 360 target, PS3 snapshot path, Xbox 360 snapshot checked | none | exactly one `physical_snapshot:` finding |

The second row is the load-bearing one: the checked anchor is a different committed file, parses
cleanly, agrees with every declaration, and is still refused.

## 4. Semantics on this exact target (task 3)

Ground rule applied throughout: the expected label of every observation comes from a source
independent of the enum or probe under test. Three such sources exist in the repository for this
anchor: **EA's metadata names** (tables and fields, via the committed snapshot), **EA's declared
foreign keys** (`fk_parent`), and **the anchor bytes** themselves. No runtime probe's result was
used as an expectation; no named-player real-world fact was used.

| Hypothesis | Observation on `anchors/xbox360/nhllegacy/databases/nhlng.db` (sha256 `105d2186…`) | Supports | Outcome |
|---|---|---|---|
| **H1** `ubPc.vjla` code 4 = goalie, {0,1,2,3} = skater | Labels: metadata names `exhibitionskaterai`/`exhibitionskaterequipment` vs `exhibitiongoalieai`/`exhibitiongoalieequipment`. Each companion holds one row per `wBIz` (5 066 / 5 066 / 566 / 566 distinct, no duplicates, all resolving to a bio row); both skater tables hold the same 5 066 bio rows, both goalie tables the same 566; disjoint; union = all 5 632. The goalie set stores exactly {4}; the skater set exactly {0,1,2,3}; per-row `code == 4 ⇔ goalie companions` with no exception. | H1 fully; codes 5–7 unused | **Established, measured.** `[position_discriminator]` promoted. Per-code labels of 0–3 (C/LW/RW/D) NOT established and not declared. |
| **H2** companion rule: `dSvy`+`xmja` 1:1 for skaters, `mHuy`+`fMVl` 1:1 for goalies, key `wBIz` | Same measurement. | H2 fully | **Established, measured.** Previously a candidate "blocked on a dependency". |
| **H3** the `dSvy` ratings apply to skater-code rows | Same measurement: `dSvy` ownership ⇔ code ∈ {0,1,2,3}. | H3 | **Established, measured** (`applicability` target-predicate on `ubPc.vjla` codes [0,1,2,3], for `rating_overall` and `rating_speed`). |
| **H4** main team `ubPc.dXSB` = roster slot team + 1 | Chain: `eGlu.XSWT` → `ZBac.vfEq` is **declared** in EA's metadata (`fk_parent = exhibitionplayers`, asserted); `ZBac.ykFq` (named `playerid`) → `ubPc.wBIz` by value (every one of 5 532 slots resolves). For each of the **5 110** rostered players **exactly one** of its slots satisfies `team == dXSB − 1`; **0** slots satisfy `team == dXSB`; the 370 players holding slots on more than one team never contradict. | H4 fully; refutes origin-0 | **Established, measured**: transform. |
| **H5** absence code | All **522** un-rostered players store pattern 0; no rostered player stores 0; the all-ones pattern is stored on **none** of 5 632 rows. | H5 (0 = absent); refutes "all-ones = absent" as what this anchor stores | **Established, measured**: `null_sentinel`. REG1a's "internally contested" candidate (reading A, `raw-all-ones`) withdrawn; reading B is what the bytes show. |
| **H6** `dXSB` (and `eGlu.dXSB`) reference `kOtt.vfEq` | Every slot team code and every `dXSB − 1` is contained in the 252-key set `vfEq` 0..251; EA declares **no** foreign key on either field; the name is `team`. | Consistent with H6; **not proof** | **Still blocked (candidate).** Containment + name is not a foreign-key relationship (Codex's rule). The mapping keeps `qualification = "blocked"` with the blocker narrowed to this one fact; the observation is recorded in the candidate note, with what would promote it (a metadata declaration or a controlled edit whose effect is observed on the named team). |
| **H7** handedness `GRkp` 0 = left | Stored distribution **{0: 3 781, 1: 1 851}** — exactly the figures `defs/enums/handedness.toml` reports for its uncommitted `default.db` (`Left = 3781`, `Right = 1851`). | Only that the enum's source carries the same values in this field; **nothing about which code is left** | **Still blocked (candidate).** No independently supported hand for any named player exists in this repository (the FHM corpus is untracked; real-world knowledge is not committed evidence; the enum itself notes EA's stored value can differ from real life). The observation is recorded in the candidate note; the case asserts the mapping stays blocked. |
| **H8** parent-team `dbBW` | Read only as a histogram (0: 4 116; 1..30 ≈ 43–58 each); not mapped, not investigated further. | — | Not declared; out of scope. |

**Promotions: 7 blocks** (discriminator, companion rule, 2× applicability, transform, sentinel,
`kOtt` capacity) — the outcome of the measurements, not a target. **Candidates retired: 6**
(those six minus the capacity, which was unknown without a candidate). **Blocked and still blocked:
2** (H6, H7).

### 4.3 A storage regime corrected on evidence

Promoting H5 exposed a contradiction in REG1a's `player.team` storage block. It had classed the field
`neg1-sentinel` by applying the scalar-mutation audit's taxonomy (`range_low == −1` ⇒ absence is the
all-ones pattern) mechanically; the contract enforces that regime's consequence (`SentinelConflict`:
"the neg1-sentinel storage regime encodes absence as raw-all-ones = −1"). The anchor stores absence
as pattern **0** and never stores all-ones, and under the neg1 reading pattern 0 would decode to team
0 for 522 players who have no team. The committed format record already names this field as an
**offset-binary exception** (`docs/formats/tdb-v6-format.md`, "the long-verified '+1, raw 0 = none'
convention on `[−1..N]` reference fields (team `dXSB`, …)" is `raw = value − range_low`). The block
is now `signedness = "offset-binary"`, `derived`, with the premises, the correction and what stays
unobserved (the all-ones pattern) written into its derivation; the mutation-matrix `neg1-sentinel`
row it used to cite (`Egqt.yeEP`) is a different field and no longer cited there.

Consequence for the representation — chosen so that the +1 is applied **once**: storage decodes
pattern `k` to `k − 1` (and 0 to −1 = `range_low`); the transform is therefore `index-origin-0`
over the decoded value; the sentinel is `literal-code`, `raw = −1` **in decoded units** (the same
convention the contract's `raw-all-ones = −1` uses), with the note stating the stored pattern is 0;
the foreign-key candidate's `index_origin` is 0 over the decoded value. `human_range` for the
mapping is `[0, 255]` (decoded −1..254 shifted to canonical 1-origin, 0 being the absence code),
and `tests.rs` asserts that figure for Legacy while still asserting `None` for the two neighbours,
whose origins remain candidates.

## 5. `team.name` behaviour (task 4)

Case `team_name_edit_round_trips_on_a_disposable_copy`, on an **in-memory copy** of the main anchor
(sha256 pinned; the file on disk is never written):

1. the unedited copy re-serializes byte-identically (`serialize() == original`);
2. `kOtt` record 0 `YmkJ` (`fullname`, 400 bits) reads `Anaheim Ducks®` — byte-transparent, the
   `®` is two UTF-8 bytes;
3. `set_inline_str` writes `Mighty Ducks of Anaheim®`, `recompute_crcs` runs, the copy is
   **reopened** with valid header CRC and size, record 0 reads the new value, record 1 still reads
   `Winnipeg Jets™`;
4. the set of differing byte offsets is contained in the field's 50-byte span of record 0 **plus the
   one 4-byte CRC word** that hashes this table's body (the successor table's `prior_crc`), and
   that word did change; every other `kOtt` record's every field, and every other table's record
   block, compare equal;
5. a second no-op `serialize()` of the reopened copy is byte-identical to the edited buffer;
6. a 51-byte value is refused (`false`) with the buffer untouched; a 50-byte value is accepted and
   reads back exactly.

The adapter now declares `qualification = "automated-tested"`, `inverse_verified = true`,
`automated_case = tdb_canonical::legacy_xbox360_tests::team_name_edit_round_trips_on_a_disposable_copy`;
the status block is `measured` (the case performs the scalar write). **Claimed:** exactly the six
behaviours above on this field on this anchor through `tdb_core`. **Not claimed:** that the game
reads the club name from this field (the identification stays `derived` from the metadata name, as
accepted), any UI path, any save-container round trip, any runtime acceptance.

## 6. Existing assertions: tightened, not loosened

Four `tests.rs` loops asserted "unproven" over all three adapters. Each now asserts the same thing
over the two neighbours **and** asserts Legacy's measured state explicitly, so the contrast is a
tested fact rather than a loop that happens to pass:

| Test | Before | After |
|---|---|---|
| `an_unsupported_no_sentinel_claim_is_not_operative` | no sentinel established anywhere; 9 candidates (3/2/4) | the one established sentinel is named (`player.team`, `literal-code`, measured); 8 candidates (3/2/3) |
| `a_candidate_index_origin_does_not_drive_derivation` | `human_range == None` on all three | `None` on NHL 15/10; Legacy has no transform candidate left and `human_range == Some([0, 255])` |
| `an_unproven_discriminator_does_not_drive_dependent_constraints` | nothing operative on all three | Legacy's discriminator and companion rule established and exactly `player.rating_overall`/`rating_speed` position-predicated; neighbours as before |
| `a_target_supported_derivation_stays_usable` | 7 established capacities each | 8 on Legacy (team table included), 7 on the neighbours |

`snapshot_tests::the_legacy_adapter_states_its_own_facts_and_not_nhl15s` now asserts `player.team`
is blocked *with a resolved encoding* and an unestablished parent, that all eight capacities are
`measured`, and `kOtt == (252, 252)`. One gate caught a draft: the contract's `SentinelConflict`
refused `literal-code 0` under `neg1-sentinel` storage — the correction in §4.3 followed from that
refusal, not the other way round.

## 7. Verification — isolated candidate

`…/scratchpad/reg1a-r1/candidate` = `git archive 819940a` (CRLF checkout representation) + the six
overlay files (§1, copied from the live tree, hashes recorded before and after) + `Cargo.lock` =
HEAD's lock with exactly the two lines inserted (candidate lock sha256 `cd2158af…`; HEAD's lock
`d4c15e1c…`; `diff` shows only `+ "tdb-core"` and `+ "tdb-savedata"`). Root `Cargo.toml` is HEAD's.
Build output in `…/reg1a-r1/target-final`, outside the candidate. `QA_WRITE` and `QA_EVIDENCE_ROOT`
verified unset in the shell that ran every command.

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **213 passed · 0 failed · 0 ignored** (REG1a: 205; +7 `legacy_xbox360_tests`, +1 identity case) | `log-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0, no warnings | `log-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty output | `log-fmt.txt` |
| `cargo test -p tdb-savedata -p tdb-core --locked --offline` (the seams the new cases use) | **39 passed · 0 failed** (core 13 + 12 + 5 + 3; savedata 6) | `log-test-seams.txt` |
| Candidate membership (SHA-256 manifest before/after) | 1 189 files, **0 differing entries** | `manifest-before/after.txt` |
| Live tree | HEAD `819940a` before and after; index empty; `git status --porcelain` identical before and after the candidate run | `live-*.txt` |

Development iteration happened in a separate scratch candidate (`…/reg1a-r1/dev`, also from
`819940a`), never in the live tree; the live tree received the finished overlay by copy, and the
live `Cargo.lock` received the same two lines (it still carries the pre-existing unrelated drift,
now 44 insertions vs HEAD). The exploratory probe used to take the measurements before any assertion
was written was deleted from the dev candidate and is not part of the overlay.

## 8. Corrected counts

| Figure | C0-REG1a | **C0-REG1a-R1** |
|---|---:|---:|
| Legacy established / unknown / candidate blocks | 37 / 37 / 12 | **44 / 30 / 6** (unknown counts every undeclared block; the six candidates are listed by name in `the_adapters_population_reflects_exactly_these_measurements`) |
| Registry, three adapters: established / unknown / candidate | 98 / 100 / 32 | **105 / 93 / 26** |
| Legacy mappings by qualification | 1 automated-tested (birth date) · 5 implemented (names ×2, ratings ×2, `team.name`) · 2 blocked | **2 automated-tested** (birth date, `team.name`) · **4 implemented** · **2 blocked** (`shoots`, `team`) |
| Capacities established (measured) | 7 (derived) | **8 (measured)** |
| Fields mapped / units covered | 10 / 2 965 · 3/28 (working tree) | unchanged |
| `tdb-canonical` cases | 205 | **213** |

Reported in `docs/plans/project-completion-roadmap.md` (C0 item 7) and `docs/STATE-OF-THE-REPO.md`
§7; the REG1a receipt is preserved with a labelled supersession note.

## 9. Open for Codex

1. **Main-team parent (H6).** What witness would you accept as promoting `kOtt.vfEq` as the parent
   of `ubPc.dXSB`? The repository holds no metadata declaration for it; a controlled-edit case
   (change `dXSB`, observe the resolved team name change) exercises the same inference unless the
   game's behaviour is observed. Until then `player.team` is blocked on this single fact.
2. **Handedness (H7).** No independently supported label exists here. If an authorized evidence
   source for a named Legacy player's hand is placed in the repository, one named case promotes it;
   real-world knowledge was deliberately not used.
3. **Neighbour populations (§3.2).** NHL 15 / NHL 10 roster-save populations are now untested; the
   fix is to read their fixtures, which is C0-REG1b-scale work, not this package's.
4. **Sentinel units.** The contract states sentinels in decoded units for `raw-all-ones`; this
   package followed that for `literal-code` under offset-binary storage (§4.3) and said so in the
   note. If you prefer stored-pattern units for `literal-code`, it is a one-line change to the value
   and the case.

## 10. Limits honoured

One target. No headmesh work, no external repository, no live evidence generation/publication, no
QA generator run, no `capability.csv` edit, no game installation or runtime claim, no capability
promotion, no broad cleanup or formatting outside the files named in §1. No staging, commit or push.
**C0-REG1b was not started.** Stopping for Codex review.

---

## 11. Corrections (C0-REG1a-R2, 2026-09-14) — labelled, body above preserved

| Where | R1 said | Corrected by R2 |
|---|---|---|
| §4.3, §9 item 4 | sentinel `raw = -1` "in decoded units" for `literal-code` | The contract's `raw` is the **stored code**: for `literal-code` the unsigned stored bit pattern. `player.team` absence is `raw = 0`. `raw-all-ones` keeps its `-1` notation, now documented as the one exception. See [c0-reg1a-r2-representation.md](c0-reg1a-r2-representation.md) §1. |
| §4.3, §6 | `human_range(player.team) == [0, 255]`, "decoded −1..254 shifted to canonical 1-origin" | A reference mapping has **no** human range; `[0, 255]` was a numeric-encoding interval and is neither the canonical team domain nor a proven set of valid references. `human_range` now returns `None` for any mapping carrying a foreign-key block. |
| §4 H7, adapter note | the histogram match "supports the hypothesis that the enum's source carries the same values in this field" | Matching counts prove **matching counts only** — not identical records, file identity or content equivalence. Wording corrected in the adapter, the case (renamed `handedness_counts_match_the_enum_files_reported_counts_but_no_label_is_established`) and here. |
| §3.2 | "any source is held to 'ceiling ≥ main population'" | That generic proxy is removed: ceiling ≥ population is validated only within the declared source's own file. |
| §7 table | `tdb-core` + `tdb-savedata` suites "39 passed" | The log records **47** (tdb-core 13 + 12 + 5 + 3; tdb-savedata 6 + `container.rs` 8, which the tally omitted). |


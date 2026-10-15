# C0-REG4-CLOSE + C0-REG5 — receipt

**Package:** commit the corrected REG4 boundary (CLOSE); register the remaining six physical
database units (REG5: NHL 07/08/09 Xbox 360, NHL 08/09 PS3, the NHL 07 PS3 research copy under
`nhl07-ps3`); record the Legacy Recompiled asset decision; verify and report REG5 (uncommitted).
**Baseline:** `c3f4181` → CLOSE commits `c5e3a2b`; REG5 is measured on `c5e3a2b`.
**Date:** 2026-09-15.

**Result.** REG4 is committed as **`c5e3a2b`** on a tree byte-identical to the tested one (449 cases,
clippy/fmt clean, index empty after). REG5 adds six adapters and 70 named cases (449 → **519** on an
exact isolated candidate, clippy `-D warnings` and rustfmt clean): five attributed partial adapters
measured on their own anchors and the research copy as representation only. With REG5 accepted,
**27 of 28 database units are registered — 24 attributed, 3 representation-only — and Legacy
Recompiled remains absent (FR4)**; its asset side has a user-directed reuse decision (Q47) with a
named contract follow-up. Nothing pushed; index empty.

---

## A. C0-REG4-CLOSE — the commit

| Item | Value |
|---|---|
| Pre-checks | HEAD `c3f4181`; index empty; the 16 receipt §D hashes verified against the working tree — **one mismatch**: the inventory CSV (§D listed the REG4 hash `75c3aad3…`; R1 had corrected the nhl19/nhl20 rows after computing the table). The R1 receipt §D was corrected to the working-tree hash `9bd4ee38…` with a note; the CSV is not read by any test |
| Staged | exactly the 16 boundary paths (`git add` from the list; `git diff --cached --check` clean; 16 files, +10 487 / −139) |
| Staged tree | `6904cda495ae1db5504717030306dda0e08566a1` = `git write-tree` |
| Export | `git -c core.autocrlf=false -c core.eol=lf archive <tree>` → 1 219 files; **every file byte-identical to its staged blob** (0 mismatches, 0 extra) |
| Verification (fresh target dir, `QA_WRITE` / `QA_EVIDENCE_ROOT` unset) | `cargo test -p tdb-canonical --locked --offline` **449 passed / 0 failed**; `cargo clippy … --all-targets -- -D warnings` exit 0; `cargo fmt -- --check` exit 0 (`reg4/log-close-*.txt`) |
| Commit | **`c5e3a2b6d323d1865c7e1ae8a47b76242adb5eeb`** — `feat(canonical): register seven PS4 adapters, two as unattributed representation`; `HEAD^{tree}` = `6904cda…` (the tested tree); message file; Co-Authored-By line |
| Preservation | index empty after the commit; no manifest or lock change; the unrelated modified/untracked entries untouched; nothing pushed |

---

## B. C0-REG5 — six early-generation units

### B.1 Identities (byte-exact for DB / saves; LF-normalised text digests for XML)

| Unit | Main anchor sha256 | Metadata text digest | Snapshot (gen, tables / fields) | Twin (byte-identical main + metadata) |
|---|---|---|---|---|
| `nhl07-xbox360` | `67a00bea…` (`NHL07NG.db`) | `f6e6f2a1…` | `xbox360-nhl07.json` (gen6, 90 / 1 813) | the PS3 research copy |
| `nhl07-ps3` (research) | `67a00bea…` | `f6e6f2a1…` | `ps3-nhl07.json` (gen6, 90 / 1 813) | `nhl07-xbox360` |
| `nhl08-xbox360` | `587204c3…` | `62eed788…` | `xbox360-nhl08.json` (gen7, 79 / 1 578) | `nhl08-ps3` — and the two ROSTER fixtures are **one file** (`b7a37835…`) |
| `nhl08-ps3` | `587204c3…` | `62eed788…` | `ps3-nhl08.json` | `nhl08-xbox360` |
| `nhl09-xbox360` | `0f7c826b…` | `2964a0de…` | `xbox360-nhl09.json` (gen7, 76 / 1 547) | `nhl09-ps3` (different ROSTER files, same figures) |
| `nhl09-ps3` | `0f7c826b…` | `2964a0de…` | `ps3-nhl09.json` | `nhl09-xbox360` |

The `nhl07-ps3` folder commits no save; the scope names it a research copy; its bytes are the
Xbox 360 anchor's; no committed record describes a PS3 capture. It is registered under its
existing registry identity as **representation only** (every measurement a candidate under an
unknown block, every mapping blocked, nothing operative — the R1 mechanism, with a research-copy
reason), never as a PS3 release.

### B.2 Saves and capacities — measured only through the committed codecs

| Unit | Committed ROSTER fixture, as the committed codec sees it | Capacity |
|---|---|---|
| `nhl09-xbox360` | `ROSTER1 20260717230804` (1 573 216 B, `9133606d…`; `Rosters`, **RawV3**, word 3, checksum valid) → 1 573 188 B TDB (`ad8b54d5…`, 19 tables, CRC + size ok) | **established, `roster-save-max-records`**: `ubPc` 4568/3968 · `ZBac` 5740/4240 · `eGlu` 6400/3900 · `dSvy` = `xmja` 4049/3549 · `mHuy` = `fMVl` 515/415 · `kOtt` 188/188 (packed); populations equal the packed main DB on all 19 tables |
| `nhl09-ps3` | `BLUS3016802002026 - ROSTER/SYS-DATA` (1 573 216 B, `ee886e34…`; `RosterFile`, RawV3) → 1 573 188 B (`862c0eba…`, 19 tables) | the same figures from a different file; the twin relabel case reports every capacity |
| `nhl08-xbox360` / `nhl08-ps3` | one file (`b7a37835…`, 1 238 480 B; `Rosters`, RawV3, checksum valid) → the codec yields a **14 780-byte TDB of 30 tables, none of which is a table of the main DB**; the remaining 1 223 672 B are not decoded (save-matrix: `semantic_edit = no`) | **BLOCKED** — all 8 capacity blocks `unknown`, no candidate figure; the packed main DB's population recorded as a population, never a ceiling; no codec research |
| `nhl07-xbox360` | `ROSTER1` (1 598 888 B, `40a06d53…`; `Rosters`, RawV3) → a 10 476-byte TDB of 31 tables, none in the main DB | **BLOCKED** — 6 capacity blocks `unknown` (no `xmja` / `fMVl` on gen6) |
| `nhl07-ps3` | no `savefiles/` directory (QA matrix: `missing-fixture`, save user-supplied) | **MISSING-FIXTURE** — 6 blocks `unknown` |

No figure was carried from a later title or read off a packed main DB.

### B.3 Semantics measured on each anchor (module `target_early_tests::<game>_<platform>`)

| Fact | NHL 07 (both) | NHL 08 (both) | NHL 09 (both) |
|---|---|---|---|
| Birth month / day stored ranges | 1..12 / 1..31 → **1-origin, identity** (probed; the studio-domain probe lists none of these anchors and its prose calls NHL 10 "the series' only 1-indexed title" — a **finding**, not repaired) | same | same |
| Component offsets day / month / year; year width, stored | 3996 / 4005 / 3966; 6 bits, 0..29 | 1396 / 1404 / 1373; 7 bits, 0..30 | 1399 / 1407 / 1304; 8 bits, 2..91 |
| Name fields `HdeP` / `QbMR` | 1280 bits @1280 / @2560 (160-byte budget) | 320 bits @328 / @648 (40 B) | 320 bits @336 / @656 |
| Companions (key `wBIz` = `game_id`, 13 bits) | **two**: `dSvy` 2015, `mHuy` 235 (no `xmja` / `fMVl` on gen6), one-to-one, cover all 2250 | four, one-to-one, cover all 2962 | four; skater companions **`zero-or-one`**: records 3746, 3750, 3758, 3762 (code 0) own none; 3549 / 415 |
| Position codes | {0,1,2,3} / {4} | same | same, plus the four companion-less code-0 rows |
| `ubPc.dXSB` offset / width / range; regime | 3856 / **7** / 0..127; **as-is** (slot team code stored as-is; 2176 rostered, 74 un-rostered store 0, **26 rostered on team 0 store 0**) | 0 / 8 / 0..255; as-is (2276; 686; 23 on team 0) | 0 / 8; as-is (3621; 347; 23 on team 0) |
| `eGlu.dXSB` metadata | **declared** `fkparenttable = exhibitionteams` (the parent candidate on `ubPc.dXSB` is `derived` from it; still a candidate) | undeclared | undeclared |
| Team keys / largest key / largest stored pattern | 93 / 92 / 92 (`vfEq` 7 bits) | 138 / 137 / 106 | 188 / 187 / 157 |
| Handedness `GRkp` offset, {0, 1} | 4009 {1549, 701} | 1408 {2099, 863} | 1411 {2874, 1094} |
| `dSvy.qPWY` `p_speed` offset, stored | 55, 2..49 | 67, 2..48 | 71, 2..47 |
| Six-bit maxima `dSvy` / `mHuy` | 49 / 49 | 49 / **50** (`mHuy.pJbR` `g_positioning`, `mHuy.qeTq` `g_floppiness`) | 49 / 49 |
| Rating regime | **offset 50, established (derived)**: the audit's `NHL07 gen6` row names the title, the probe resolves 50 on the bytes, the gen6 named case ran on the byte-identical research copy; human range 50..113 (audit scale 50..99) | **unresolved** — no audit row names NHL 08; the probe returns 36 because two *goalie* fields store 50 (skater block max 49); candidate `linear-offset 36`, blocked | **unresolved** — no audit row names NHL 09; probe 50; candidate `linear-offset 50`, blocked |
| `overall` name search | 0 hits / 1 813 fields → omission `unknown-field-identity` | 0 / 1 578 | 0 / 1 547 |
| `team.name` | **`HqzU.YmkJ` (`stockteamtable.fullname`, 2048 bits = 256 B)**, reached from the thin `kOtt` index (`table` RREB = 0 on all 93 rows, `teamid` mCXg → `HqzU.index`, 93/93, same row; `customteamtable` `fLEY` 0 rows); applicability predicate `kOtt.RREB` ∈ {0}; round trip passed | `kOtt.YmkJ` `fullname` @664 w400; round trip passed | `kOtt.YmkJ` @656 w400; round trip passed |
| `team.name` records 0 / 1 | "Anaheim Ducks™" / "Atlanta Thrashers®" | same | same |

Blocked on all six: `player.shoots` (codes), `player.team` (parent; **no absence code** under the
as-is regime, as on NHL 10). `player.rating_overall`: omission on all six.

**Representation limit (recorded, no contract change):** `target-adapter/2` has no block for a join
path from an entity's primary table to a field on another table. NHL 07's `team.name` therefore
sits on `HqzU` with the `kOtt.RREB` predicate and the join stated in prose and measured by the
named case; the engine's `team_name_source` thin shape implements the same join at HEAD.

### B.4 Coverage per unit

| Unit | Status | Mapped / total | Mappings / omissions | Established / unknown / candidate |
|---|---|---|---|---|
| `nhl07-xbox360` | attributed, partial | 9 / 1 813 | 7 / 1 | 31 / 33 / 6 |
| `nhl07-ps3` | **research copy, representation only** | 9 / 1 813 | 7 / 1 | **0** / 64 / 37 |
| `nhl08-xbox360` | attributed, partial | 9 / 1 578 | 7 / 1 | 30 / 36 / 7 |
| `nhl08-ps3` | attributed, partial | 9 / 1 578 | 7 / 1 | 30 / 36 / 7 |
| `nhl09-xbox360` | attributed, partial | 9 / 1 547 | 7 / 1 | 38 / 28 / 7 |
| `nhl09-ps3` | attributed, partial | 9 / 1 547 | 7 / 1 | 38 / 28 / 7 |

### B.5 Mechanics (reused where their assumptions apply; generalised without changing gen7 meaning)

* Reused unchanged: `Target`, `read_pinned` (binary), `read_pinned_text` (XML), `decoded_roster_fixture`
  + `assert_capacities_are_the_fixtures` + `roster_capacity_discrepancies` (RawV3 through the same
  `SaveContainer` path; NHL 09), `roster_slots_by_bio_record`, `main_team_measurement` /
  `assert_main_team_declared` under `SlotTeamAsIs` at width 7 or 8, `birth_date_ranges`,
  `position_codes_by_partition`, `candidate_of` / `promoted` / `all_candidate_blocks` (R1's
  candidate mechanics, lifted into `target_evidence`).
* Generalised: `companion_partition` → `companion_partition_with(companion_ids)` returning the bio
  rows without companions (the 4-table wrapper asserts none); `assert_discriminator_declared(..,
  companions, ..)` takes the expected (table, cardinality, role) set (`GEN7_COMPANIONS` at every
  gen7/PS4 call site); `kott_string_round_trip` → `inline_string_round_trip(table_id, ..)`;
  `NON_OPERATIVE_REASONS` = {"ATTRIBUTION UNRESOLVED", "RESEARCH COPY"} and `promoted` lifts only
  blocks whose unknown note gives one of them (so an adapter's ordinary candidates stay candidates).
* Sweeps (`tests.rs`): `RESEARCH_COPIES`, `non_operative()`, `EARLY_ADAPTERS`; the derivation sweep
  asserts offset 50 established on `nhl07-xbox360`, blocked-with-candidate (36 / 50) on NHL 08 /
  09, zero established capacities on NHL 07 / 08; the sentinel-candidate total 60 → 78; the
  representation-vs-coverage test now reports `["nhl19-ps4", "nhl20-ps4", "nhl07-ps3"]`.

---

## C. Legacy Recompiled — asset profile reuse (decision recorded, nothing implemented)

Recorded as **Q47** in [completion-design-questions.md](../plans/completion-design-questions.md)
(decision log) and in the roadmap's item 3: the user explicitly confirms that Legacy Recompiled's
asset files are identical to Legacy Xbox 360's and directs reuse of the same asset profile.
Accepted as the architectural direction, **attributed to the user** — no hash, member identity or
corpus is invented, and no `byte-identical-to-archive-member` bridge is declared (that method
requires a recorded comparison).

**How existing mechanisms can and cannot represent the reuse.** `asset-output-profile/4` scopes
every family block to one `[[target_profile]]`; a `ProvenanceBridge` ties an *unattributable corpus*
to an attributable corpus **on the same target** (`original_corpus` must be declared on that
target), and a `Corpus` is attributable to exactly one target. Neither can point a Recompiled
target profile at the Legacy Xbox 360 one, so today the only way to register Recompiled is to
duplicate every source-side block — which the decision rules out.

**Smallest concrete follow-up (not done here):** an optional `reuses = "<target id>"` on
`[[target_profile]]` — the referenced profile must be declared in the same file and be the source of
identity / container / compression / path families / variants / resolution chains; the referencing
profile may declare only the destination-side blocks (`discovery`, `install`, `runtime_lookup`) and
its own qualification, all of which stay Recompiled's; nothing operative crosses the reference
without its own evidence scope. An additive `/4` field or `/5`; one validator rule; one profile
edit per family. Database equivalence is **not** inferred (FR4 stands); the external headmesh
repository was not inspected.

---

## D. Validation and boundary

**Candidate:** `git -c core.autocrlf=false -c core.eol=lf archive c5e3a2b` (1 211 files) + the
15-file boundary below → 1 226 files; fresh `CARGO_TARGET_DIR`; `QA_WRITE` / `QA_EVIDENCE_ROOT`
unset; no manifest or lock change.

| Check | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **519 passed · 0 failed · 0 ignored** (449 + 70) | `reg4/log-r5-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `log-r5-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-r5-fmt.txt` |
| non-boundary files vs `c5e3a2b` blobs, byte-exact, no normalisation | 0 changed · 0 missing · 0 extra | inline |
| boundary files vs working tree | all 15 byte-identical | below |

Failures met on the way, none fixed by weakening: two borrow-lifetime and one `const fn` compile
errors; the twin-citation rule (an early adapter legitimately *mentions* the other platform's
byte-identical fixture and the gen6 case's research-copy path — the rule now forbids twin files in
**provenance**, not in observations); the research runner asserted a decode the as-is regime cannot
have (no established sentinel); the position sweep did not expect NHL 07's predicated `team.name`;
one dead field and one `nonminimal_bool` lint.

### D.1 Boundary (SHA-256 of working-tree bytes; A = new, M = modified vs `c5e3a2b`)

```
b31ee38e89b22fdfc0c3ba2410c331e76a261ed922881b802f2096e44f178ad9  crates/tdb-canonical/src/target_early_tests.rs      A  1695 lines
fc113be793dd42b91af85f1945e01a72e5f2753e2ffbfedcc2259a6bcb499886  crates/tdb-canonical/src/target_evidence.rs         M  +218/−26
4c98901cc145bda9488a996607e34ab553ab88e1281afbcfef05c39dfaa69dff  crates/tdb-canonical/src/tests.rs                   M  +102/−18
c1bb369dbd41c6bec46d0f3e94a901c790d34f65ec591fc434ad2c395d85ae23  crates/tdb-canonical/src/target_ps4_tests.rs        M  +2/−134  (candidate helpers moved to target_evidence)
33988d7ddd14ab86cea2bd1f786e73d99b618f1c9751c4ed2d29205b679af76b  crates/tdb-canonical/src/snapshot_tests.rs          M  +7/−1
24dd29c508a02eebd17605089b4fa4b7ff8d5554cf04dec673acc10624facb47  crates/tdb-canonical/src/target_batch_tests.rs      M  +9/−1
9e957a6a659a61662960c9122360dbf9287e442b228f9ba8c509111336552f1a  crates/tdb-canonical/src/legacy_xbox360_tests.rs    M  +1/−0
281327672c3990c0c3aacf3fa7fa1147a3282ea88ad7a1bd3008a4d87ffe5ec9  crates/tdb-canonical/src/nhl15_ps3_tests.rs         M  +1/−0
3e6d230b2f364ff7fc022bf3032ecce20dc025e4b65d34f6368f25055173b25e  crates/tdb-canonical/src/lib.rs                     M  +2/−0
226a3bf46dc5b3e251b33be226f52f47a4cf96bfd4986d8ead5dfbfdf10f1fa5  defs/adapters/nhl07-xbox360.toml                    A   861 lines
2fd701ad2435e1834c9838693500b875bc4cf82312e9cdee77697512b1a9f7fe  defs/adapters/nhl07-ps3.toml                        A   949  (demoted: research copy)
50279d8e7d4df0d9663682727ef85aeadb77040ea9bd15efd35fb4b43561d979  defs/adapters/nhl08-xbox360.toml                    A   879
f393309952e65e47790d40d4a48f75f16d20f55328a9e782f1e195033153d05b  defs/adapters/nhl08-ps3.toml                        A   879
c4a9221e8556e6f9f8275bf5c4df308fa6354839a83ca90fa78b32481c8ac8cf  defs/adapters/nhl09-xbox360.toml                    A   960
7f8b05e4577576795067be9a9b065d28ca9589b1375af1f54e2c18eac6fc05a1  defs/adapters/nhl09-ps3.toml                        A   960
```

Outside the boundary (documentation, untracked or already-modified as before): this receipt, the
R1 receipt's §D correction, `docs/plans/project-completion-roadmap.md`,
`docs/plans/completion-design-questions.md` (Q47), `docs/STATE-OF-THE-REPO.md`. The adapters were
produced by a scratch generator (`reg4/gen_early.py` + `demote.py`) reading every physical number
from the target's snapshot and the measured facts from the probe run (`reg4/probe6-out.txt`).

Live tree: HEAD `c5e3a2b`, index empty, nothing staged, nothing pushed, unrelated work preserved.
Proposed boundary: one commit, `feat(canonical): register the six early-generation adapters, the
NHL 07 research copy as representation` (15 files; expected 519).

---

## E. Registry counts (if REG5 is accepted)

| Category | Units |
|---|---|
| Attributed partial adapters, measured on own anchor | 22: `nhllegacy-xbox360`, `nhl15-ps3`, `nhllegacy-ps3`, `nhl14-*`, `nhl13-*`, `nhl12-*`, `nhl11-*`, `nhl10-ps3`, `nhl15/16/17/18/21-ps4`, `nhl07-xbox360`, `nhl08-*`, `nhl09-*` |
| Attributed partial adapters, structural (audit-derived capacities) | 2: `nhl15-xbox360`, `nhl10-xbox360` |
| Registered representation only — attribution unresolved | 2: `nhl19-ps4`, `nhl20-ps4` |
| Registered representation only — research copy | 1: `nhl07-ps3` |
| Absent (no anchor of any kind, FR4) | 1: Legacy Recompiled — asset direction recorded (Q47), database unit still absent |
| Game-verified | 0 |

Capacity evidence across the 27: fixture-measured on 17 (gen7 ROSTER saves; PS4 main DB on 5 — the
save cross-checked on 16–19), audit-derived on 2, **unknown** on 4 (NHL 07/08: fixture decodes without
the roster core; `nhl07-ps3`: no fixture), candidate-only on 2 (unattributed). Blocked on every
adapter: `player.shoots` codes and the `player.team` parent. Partial registration is not usable
support; QA activation stays parked.

## F. Limits honoured

No new codec (the NHL 07/08 roster containers stay undecoded beyond what RawV3 yields); no runtime
research; no later-generation identity, origin, transform, sentinel, companion rule or capacity
carried; no root manifest/lock change; no anchor altered; the research copy never represented as a
PS3 release; no Recompiled hash invented; the external headmesh repository not inspected; nothing
pushed; index empty.

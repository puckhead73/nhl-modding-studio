# C0-REG1b — NHL 15 PS3 adapter and direct fixture verification

**Package:** C0-REG1b · **Date:** 2026-09-14 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `1ede5baa8ba1ab261a253c9bfdcc5e0557116e99` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** `defs/adapters/nhl15-ps3.toml` exists in the working tree: **8 canonical properties
> over 10 of 2 969 physical fields**, 8 tables, 8 capacity blocks, a companion rule and a position
> discriminator — **44 established / 30 unknown / 6 candidate blocks**, measured from the start on
> this target's own files. Every physical number is read from `ps3-nhl15.json` and checked against
> the PS3 anchor bytes by the existing target-bound snapshot check (now over four adapters); every
> capacity is the observed directory of the committed PS3 ROSTER fixture, decoded by the production
> save codec; every semantic is measured on the PS3 anchor with labels from EA's metadata names and
> declared keys. The PS3 / Xbox 360 main-DB **and metadata** identity was verified directly on the
> bytes, recorded as an observation, and carries nothing.
>
> The Legacy module's mechanics were factored into a shared `target_evidence` test-support module
> (parameterised by target identity; no expectation lives in it) so the NHL 15 PS3 module is 433
> lines of target-specific expectations rather than a copy. Legacy's cases and expectations are
> unchanged (one stale case-name citation in its adapter corrected).
>
> Isolated candidate from `1ede5ba` + an 8-file overlay, no manifest or lock change, env unset:
> **`cargo test -p tdb-canonical --locked --offline` 225 passed · 0 failed · 0 ignored**
> (214 → 225), clippy `-D warnings` clean, fmt clean, candidate membership unchanged (1 192 files).
>
> Registry: **3/28 committed + 1 in the working tree**; `player.shoots` and `player.team` stay
> blocked on this target for the same reasons as on Legacy. Adapter presence is not target
> completion: 2 959 fields remain unmapped and no cell is game-verified.

---

## 1. Exact files (all uncommitted), SHA-256 as verified in §6

```
f104802940203fdeb2d55dac1662cfc8a7f5d636d52fc84711946e3ef28d0a0f  defs/adapters/nhl15-ps3.toml                      A  1009 lines
ed0d66cbb5b122f692dc1592665e8a0c07c49f904a05e25f1b2e071b5a64c00b  defs/adapters/nhllegacy-xbox360.toml              M  1 line: stale case citation → current name
a79a1f99f4fc537249f0143b6897a35fa3e1ff287f193bf2bece030faac4b3fa  crates/tdb-canonical/src/lib.rs                   M  +4 (two `#[cfg(test)] mod` lines)
bef74c0728fd771718d9966603f04f82976941234ecd45d8b95c9cf9442cdac7  crates/tdb-canonical/src/target_evidence.rs       A  1027 lines (shared test mechanics)
9dab0dcc3de3503984c24ca6e668097458f2125ffcbca8adcfd395afd9dd0ef2  crates/tdb-canonical/src/legacy_xbox360_tests.rs  M  1191 → 410 lines (mechanics moved out; +1 birth-date case; +1 citation assertion)
a4b20ddfe84c10f561e211da35d0181f010c55d7dab88a4da1bffef6d4c8e644  crates/tdb-canonical/src/nhl15_ps3_tests.rs       A  433 lines
1d145fb8a415104b25e5471ff39b71b53516cb2f8675041b7bf0217534ec7949  crates/tdb-canonical/src/snapshot_tests.rs        M  +3/−2 (four adapters; population 4)
0234a7afd08b26cbeda8300cd66d05b6fb2b41de415cc0a57d14f91c8f43bf42  crates/tdb-canonical/src/tests.rs                 M  +35/−29 (four adapters; per-adapter expectations)
```

No `Cargo.toml`, root manifest or `Cargo.lock` change: the dev-dependencies C0-COMMIT-REG1 added
(`tdb-core`, `tdb-savedata`) are the seams this package uses. Documentation (this receipt, roadmap,
STATE) is outside the proposed commit boundary.

## 2. Measured fixture identities

| File | Identity | Measured |
|---|---|---|
| `anchors/ps3/nhl15/databases/nhlng.db` | sha256 `028ae06ad2b25bf59aae37249b96144add014664771403e0313b5e3054b03398` | byte-equal to `anchors/xbox360/nhl15/databases/nhlng.db` (asserted on the bytes by `the_ps3_main_db_and_metadata_are_byte_identical_to_xbox360s_and_nothing_rests_on_it`) |
| `anchors/ps3/nhl15/databases/nhlng-meta.xml` | sha256 `bc0588daa82306c28f9129b0a33f6d5f46b80c4824f825be81b8a0a3139cb4de` | byte-equal to the Xbox 360 metadata |
| `docs/audits/data/schema/ps3-nhl15.json` | 134 tables / 2 969 fields; identical to `xbox360-nhl15.json` except `platform` | asserted to be the rendering of the PS3 anchor by `snapshot_tests::anchor_snapshot`; the adapter cites only this rendering and no Xbox 360 path (asserted) |
| `anchors/ps3/nhl15/savefiles/BLUS314360200 - ROSTER/SYS-DATA` | 2 259 432 bytes, sha256 `fd234dc030586fb1366ebace47cf805cfd1aa38ac49cf1f9d52783b3d3a2f98c` | magic `PS3RosterFile` → `SaveType::Roster`; **version word 2** with a zlib body at `0x30` → `ContainerVersion::ZlibV4` (resolved by body marker; the Legacy X360 fixture carries word 4 for the same framing); `checksum_a` / `checksum_b` both recompute; declared `uncompressed_size` 2 259 388, `compressed_size` 704 009, `field_20 = 1` |
| decoded payload | 2 259 384 bytes (4-byte trailer trimmed), sha256 `362c40ded18e17f3e412557d5a141096c819684138440c8facfc443fdf974d1e` | big-endian, **38 tables**, header CRC and size valid |

The identity is stated in the adapter header with its limits ("carries NO save capacity,
deployment behaviour or runtime conclusion across targets, and nothing in this file rests on it") —
and the case asserts that sentence is present.

## 3. Evidence disposition

### 3.1 Capacities (task 2) — the fixture's directory, per table

| Table | `current_records` | `max_records` (ceiling) | Main anchor `current_records` |
|---|---:|---:|---:|
| `ubPc` | 5 534 | **7 034** | 5 534 |
| `ZBac` | 5 942 | **8 948** | 5 942 |
| `eGlu` | 5 511 | **10 619** | 5 511 |
| `dSvy` | 4 978 | **6 479** | 4 978 |
| `xmja` | 4 978 | **6 479** | 4 978 |
| `mHuy` | 556 | **855** | 556 |
| `fMVl` | 556 | **855** | 556 |
| `kOtt` | 246 | **246** | 246 |

All eight `measured`, `source = "roster-save-max-records"`, provenance = fixture path + named case +
`save-matrix.csv#ps3,nhl15,BLUS314360200 - ROSTER`, each note stating *observed directory capacity
of that fixture, not a demonstrated runtime limit*. Populations equal the main anchor's for all 38
tables the save carries — asserted as a **fixture observation**; nothing requires it. Ceiling ≥
population is checked within the save only. **Target-specific regression**
(`a_capacity_carried_from_the_xbox360_adapter_is_reported_against_this_fixture`): the seven figures
the NHL 15 Xbox 360 adapter carried from the roster-capacity audit coincide with this fixture's
(the audit measured this very save), so numbers alone cannot distinguish inherited from measured —
carrying the Xbox 360 blocks' provenance is reported for all seven, and `kOtt` exists only here. The
generic one-off / wrong-source / absent-table mutations are not duplicated (Legacy covers them).

### 3.2 Semantics (task 3), labels independent of the enum / probe under test

| Fact | Observation on the PS3 anchor | Outcome |
|---|---|---|
| Birth-date index origin | stored months 0..=11, days 0..=30 (years 61..=98); the named `studio-domain` case lists this anchor by path; component offsets **959 / 977 / 816** (Legacy: 958 / 975) | `automated-tested`, `index-origin-0`, measured |
| Position discriminator / companions | goalie-companion owners (`exhibitiongoalieai`, `exhibitiongoalieequipment`): **556** rows, code {4}; skater-companion owners: **4 978** rows, codes {0,1,2,3}; 1:1 on `wBIz`, disjoint, exhaustive over 5 534 | established, measured; ratings applicability target-predicate [0,1,2,3] |
| Main-team encoding / absence | via declared `eGlu.XSWT → exhibitionplayers` + `playerid` join: **5 088** rostered players each with exactly one slot `team == pattern − 1`, 0 with `team == pattern`; **446** un-rostered store 0; all-ones stored nowhere; codes ⊆ `kOtt.vfEq` (246 keys) | storage `offset-binary` (derived, format-doc exception + measurement), transform `index-origin-0`, sentinel `literal-code raw = 0` — measured; **parent stays a candidate**, mapping **blocked** |
| Codec order | every pattern 0..=255 round-trips through the test helper; on the anchor 446 `Absent` / 5 088 `TeamCode`, inverse restores each pattern; largest stored pattern 217 | codec behaviour only; no production executor |
| Handedness | histogram **{0: 3 782, 1: 1 752}**; the enum file is anchored on another title and reports no counts for this one | **blocked**; counts recorded as counts and nothing more |
| `team.name` | record 0 `Anaheim Ducks®`, record 1 `Winnipeg Jets™`; edit / CRC / reopen / no-op serialize / over-budget refusal as on Legacy | `automated-tested`, `inverse_verified`; identification stays `derived` |
| Names, ratings | inline-string codec row `ps3,nhl15,inline-string,BGwe,vIys`; ratings regime row names NHL 15 PS3 by title and platform; the ratings case ran on nhl14/ps3 | names `implemented` measured; ratings `implemented` derived |

`player.shoots` and `player.team` are `blocked`; their proposals are candidates with the observations
above recorded in the notes.

### 3.3 What the Xbox 360 adapter was, and was not, used for

Structural reference only (which blocks exist). No value, citation or count was carried: the
generator asserted every physical number against `ps3-nhl15.json`, every provenance names a PS3
file / `ps3,nhl15` row / this module's case, and the identity case asserts the adapter text contains
no `anchors/xbox360/` path and no `xbox360-nhl15.json`.

## 4. Verification integration (task 4)

- `snapshot_tests::COMMITTED_ADAPTERS` and `tests::COMMITTED_ADAPTERS` now list four adapters; the
  target/snapshot identity guard and every physical cross-check run over `nhl15-ps3` (0 discrepancies
  against the PS3 anchor); `registry_population` counts four.
- `tests.rs` per-adapter expectations extended: sentinel candidates 3/2/3/3 (11 total) with the
  measured `player.team` sentinel named on Legacy and NHL 15 PS3; discriminator/companions operative
  on exactly those two, unproven on NHL 15 X360 / NHL 10; capacities 8 on those two, 7 on the others.
- `target_evidence.rs` (new, `cfg(test)`): `Target` / `RosterFraming` descriptors and the shared
  mechanics — fixture decode with framing assertions, source-scoped capacity discrepancies, companion
  partition, position codes, roster-slot walk, main-team counts, codec order helpers, team-name round
  trip, birth-date ranges, candidate ledger — with adapter-declaration assertions. No expectation is
  hard-coded in it.
- Legacy module: same case names, same numbers; gains `birth_date_month_and_day_are_stored_zero_origin_on_this_anchor`
  (month 0..=11 / day 0..=30 on the Legacy anchor) and a `cites` assertion for the handedness case.

## 5. Coverage accounting (separately, as asked)

| Figure | Before (HEAD `1ede5ba`) | After (working tree) |
|---|---:|---:|
| Adapter units | 3 / 28 committed | 3 committed + **1 uncommitted** (`nhl15-ps3`) |
| `nhl15-ps3` mapped physical fields / total | — | **10 / 2 969** (8 mappings, 0 omissions) |
| `nhl15-ps3` established / unknown / candidate blocks | — | **44 / 30 / 6** |
| Registry established / unknown / candidate (4 adapters) | 105 / 93 / 26 | **149 / 123 / 32** |
| `tdb-canonical` cases | 214 | **225** (+10 NHL 15 PS3, +1 Legacy birth-date) |
| Asset profiles | 1 / 14 | unchanged |

Adapter presence ≠ target completion: NHL 15 PS3 has 2 959 unmapped fields, two blocked mappings,
no runtime capacity and no game verification.

## 6. Validation in isolation (task 5)

`…/scratchpad/reg1b/candidate` = `git archive 1ede5ba` + the 8 overlay files (§1). `Cargo.lock`
and every manifest are HEAD's. Build output in `…/reg1b/target-final`; `QA_WRITE` /
`QA_EVIDENCE_ROOT` verified unset.

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **225 passed · 0 failed · 0 ignored** | `log-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `log-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-fmt.txt` |
| Candidate membership before/after | 1 192 files, 0 differing | `manifest-before/after.txt` |
| Live tree | HEAD `1ede5ba`; index empty; `git status --porcelain` identical before/after | `live-*.txt` |

No additional focused suite was run: no shared production behaviour changed (`tdb-canonical`'s
non-test sources gained only two `mod` lines). Exploratory measurement was taken with a throwaway
probe in a separate scratch candidate, deleted, and not part of the overlay.

## 7. Unresolved blockers

| Blocker | Status |
|---|---|
| `player.team` parent (`kOtt.vfEq`) | candidate — containment in 246 keys + name `team`; mapping blocked |
| `player.shoots` codes | candidate — enum anchored on another title; no independently supported label here |
| 2 959 unmapped NHL 15 PS3 fields | unknown |
| Runtime ceilings | directory capacities only |
| NHL 15 Xbox 360 / NHL 10 save capacities | still untested by fixture (out of scope, not repaired) |
| Game verification | none |

## 8. Proposed commit boundary

The 8 files of §1 (3 A, 5 M), no lock or manifest change. Documentation (this receipt,
`docs/plans/project-completion-roadmap.md`, `docs/STATE-OF-THE-REPO.md` — both updated to report
the fourth adapter as uncommitted) outside, unless named.

## 9. Limits honoured

One target. NHL 15 Xbox 360 and NHL 10 capacities not repaired. No external repository, headmesh
work, game installation, evidence generation/publication, capability promotion, broad cleanup,
staging, commit or push. Stopping for Codex review.

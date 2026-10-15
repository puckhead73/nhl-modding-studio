# C0-REG1a-R2 — finish sentinel and capacity representation

**Package:** C0-REG1a-R2 · **Date:** 2026-09-14 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `819940a9fcf255221ce64bb6b0b2229942ec304a` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** Both representation issues are resolved in the contract, the adapter and the tests, and
> the accumulated REG1a + R1 + R2 slice is verified in isolation as one commit boundary: **8 files +
> 2 lock lines**, `cargo test -p tdb-canonical` **214 passed · 0 failed**, clippy `-D warnings`
> clean, fmt clean, candidate membership unchanged.
>
> Sentinel `raw` for `literal-code` is now the **unsigned stored bit pattern** (Legacy `ubPc.dXSB`
> absence: `raw = 0`); `raw-all-ones` keeps its `-1` notation, now documented as the single
> exception; the validator enforces both; the processing order (stored bits → absence → storage
> decode → index mapping) is written into the contract and demonstrated with a **test helper** over
> every pattern on the anchor, with the inverse restoring each original pattern. `human_range` yields
> `None` for reference mappings, so `[0, 255]` is no longer said of `player.team` anywhere.
> Capacity `ceiling ≥ population` is validated only within the declared source's own file; the last
> main-DB proxy is gone and a synthetic regression proves a small save directory is not a finding.
> `player.team` stays blocked on its parent; handedness stays blocked; the histogram wording now
> claims matching counts only.

---

## 1. Decisions implemented — sentinel units and processing order

| Decision | Where |
|---|---|
| `NullSentinelValue.raw` for `literal-code` = unsigned stored bit pattern, `0 ..= 2^width − 1`, before any storage decoding | `model.rs` field doc; `lib.rs` `SENTINEL_ENCODING` doc |
| `raw-all-ones` keeps `raw = -1` — the value the `neg1-sentinel` regime decodes the all-ones pattern to — as the **one** encoding whose `raw` is not a pattern; documented explicitly, existing adapters unchanged | same docs; NHL 15 / NHL 10 / Legacy candidates untouched |
| Validator: `raw-all-ones` with `raw ≠ -1` → `SentinelConflict`; `literal-code` with `raw < 0` → `SentinelConflict`; `literal-code` pattern that does not fit `bit_width` → `SentinelConflict`; the neg1-regime and unsigned-range rules kept | `lib.rs` `validate_null_sentinel` |
| Legacy `player.team`: `encoding = "literal-code"`, **`raw = 0`**, `canonical_meaning = "absent"`; note states the pattern is matched before the offset-binary decode runs | `defs/adapters/nhllegacy-xbox360.toml` |
| Processing order stated: (1) inspect stored bits against the sentinel pattern; (2) on match, canonical absence, nothing further decoded; (3) otherwise storage-regime decode; (4) then transform / index mapping and, for a reference, the parent-key mapping — which exists only when `[mapping.foreign_key]` is established | `SENTINEL_ENCODING` doc; adapter header |
| The contract ships **no executor** of that order; the order is exercised by a test helper and the docs say so | `SENTINEL_ENCODING` doc, adapter header, case doc |

**Existing declaration affected.** The only `literal-code` sentinel in the repository before this
package was the labelled synthetic fixture `crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml`
(`raw = -16` on a 5-bit two's-complement field, decoded units). It is now `raw = 16` (`0b10000`,
which that field decodes to −16), with its note recording the change. No real adapter carried a
`literal-code` sentinel; the three real adapters' `raw-all-ones = -1` candidates are unchanged and
still validate under the new rule.

## 2. Processing-order evidence

Case `legacy_xbox360_tests::stored_zero_is_absence_and_nonzero_patterns_decode_to_team_codes`, with
the helpers `decode_stored` / `encode_stored` (test code; a `Decoded::{Absent, TeamCode(u64)}` result
where `TeamCode` is a 0-origin code in the roster table's code space — *which table those codes index
is exactly what remains unresolved, and the helper never reads `kOtt`*):

| Stored pattern | Step reached | Result | Inverse |
|---:|---|---|---:|
| `0` | (1)–(2): matches sentinel pattern 0 | `Absent` — the offset-binary decode (which would read −1) is never applied | `0` |
| `1` | (3) offset-binary → 0; (4) index-origin 0 → code 0 | `TeamCode(0)` | `1` |
| `84` | → 83 → code 83 | `TeamCode(83)` (bio record with roster slots {83, 138} in R1's measurement) | `84` |
| `223` | → 222 | `TeamCode(222)` (the largest pattern stored on the anchor) | `223` |
| `255` | → 254 | `TeamCode(254)` — the all-ones pattern is **not** this field's sentinel; unobserved on the anchor | `255` |
| all `0..=255` | — | `encode(decode(p)) == p` for every pattern | — |
| all **5 632** `ubPc.dXSB` values on `nhlng.db` (sha256 `105d2186…`) | — | 522 `Absent` ⇔ no roster slot; 5 110 `TeamCode(c)` with `c` among that player's roster slot team codes; inverse restores every original pattern | — |

Validator regressions (`tests.rs::a_sentinel_must_match_its_encoding_and_its_storage_regime`, new
trailing block): refused — `raw-all-ones` with `255`, `literal-code` with `-1`, `literal-code` with `256` on 8
bits; accepted — `raw-all-ones -1` under `neg1-sentinel`, `literal-code 0` and `literal-code 255`
under `offset-binary −1..254`.

## 3. `human_range` and the team reference

`human_range` now returns `None` for any mapping that carries a `[mapping.foreign_key]` block,
established or proposed, and its doc says why: it describes a numeric encoding — decoded storage
range shifted by the transform — and a reference's decoded numbers are parent-key indices whose
valid set is the parent table's key set, which the helper cannot see. Regressions:

- `tests.rs::a_candidate_index_origin_does_not_drive_derivation` — Legacy `player.team`: transform
  established, foreign-key proposal present, `human_range == None`; the same blocks with the
  foreign-key block cleared yield `Some([0, 255])`, asserted as *numeric-encoding information only*.
- `legacy_xbox360_tests::main_team_pointer_…` — `human_range(player.team) == None`, "a reference has
  no human range: the parent key set, not a numeric interval, is its domain".
- NHL 15 / NHL 10 unchanged (`None`, candidate transform). Ratings unchanged (`Some([36, 99])`).

The canonical `player.team` stays `type = "reference"` in the project schema; nothing turns it into
an integer identity. `player.team` keeps `qualification = "blocked"` on the unresolved parent.

## 4. Source-specific capacity checks

| Check | Before (R1) | Now |
|---|---|---|
| `snapshot_tests::discrepancies`, any source: `ceiling ≥ main-DB current_records` | present | **removed** |
| `main-db-max-records` vs the main anchor: population == `current_records`, ceiling == `max_records`, and `max_records ≥ current_records` within the anchor | first two | all three |
| `roster-save-max-records` vs the main anchor | nothing (R1) | nothing — stated in the code comment as deliberately incomplete for NHL 15 / NHL 10 until their fixtures are measured; no proxy |
| `legacy_xbox360_tests::roster_capacity_discrepancies` vs the decoded Legacy save: ceiling == `max_records`, population == `current_records`, provenance names the fixture, source is the save | present | present, plus `max_records ≥ current_records` **within the save** |
| Synthetic regression (`a_declaration_that_disagrees_with_the_anchor_is_reported`, case 7): Legacy `ubPc` capacity set to 300 / 250 (a directory far smaller than the main DB's 5 632) | would have been reported | **no finding** from the snapshot check and `validate_adapter` clean; the save-specific check is what would hold it to its fixture |
| Rejection cases in `a_capacity_that_is_not_the_fixtures_is_reported` | 6 | unchanged |

No new real fixture. NHL 15 / NHL 10 save-capacity verification remains explicitly incomplete.

## 5. Bounded conclusions preserved

- **Handedness** — still blocked. Wording corrected in the adapter note, the case (renamed
  `handedness_counts_match_the_enum_files_reported_counts_but_no_label_is_established`, which now also
  asserts the candidate note says "matching counts only") and the R1 receipt (§11): the matching
  histogram proves matching counts only — not identical records, file identity or content
  equivalence.
- **Main-team parent** — still a candidate; `player.team` still blocked on it; the codec case's doc
  says it establishes codec behaviour, not the parent.
- No evidence acquisition was attempted.

## 6. Validation — isolated candidate

`…/scratchpad/reg1a-r2/candidate` = `git archive 819940a` + the 8 overlay files below (copied from
the live tree) + `Cargo.lock` = HEAD's lock with exactly two lines inserted. Root `Cargo.toml` is
HEAD's. Build output outside the candidate; `QA_WRITE` / `QA_EVIDENCE_ROOT` verified unset.

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **214 passed · 0 failed · 0 ignored** (R1: 213; +1 codec-order case; the sentinel-unit regressions extend an existing case) | `log-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0, no warnings | `log-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-fmt.txt` |
| `cargo test -p tdb-savedata -p tdb-core --locked --offline` | **47 passed** (R1's receipt mis-tallied this as 39; corrected there, §11) | `log-test-seams.txt` |
| Candidate membership before/after | 1 189 files, 0 differing | `manifest-before/after.txt` |
| Live tree | HEAD `819940a`; index empty; `git status --porcelain` identical before/after | `live-*.txt` |

The live tree's own `cargo test -p tdb-canonical --locked --offline` also passes 214 (against the
working tree's drifted `tdb-core`/lock); the candidate figure is the authoritative one.

## 7. Final commit boundary — one reviewed slice

**Files (8), SHA-256 of the exact bytes verified in §6:**

```
3b5a5bd64beaf6579b66d64d9eafcadd05d6b1d1dd7c6ea22bd2f244ca0d8abd  defs/adapters/nhllegacy-xbox360.toml                  A  1017 lines
0cb25ec366ea081779d728c45a81d8893c7de50ffeb94efb86dc69e17bfc13d3  crates/tdb-canonical/Cargo.toml                        M  +5
4cc3c347da5b1664c1fc675b1023b2877bc2276f6311b303430a549e7c7143c6  crates/tdb-canonical/src/lib.rs                        M  +66 / −0
55bbaca1f9eb5cb6f9d904f86be9226a4600763c8ce381cbdd98627b11000f39  crates/tdb-canonical/src/model.rs                      M  +3 / −1 (CRLF)
f00980a6e72beafc13d5550ad43d23d49df8a0fde17946e1459526ea31c6275b  crates/tdb-canonical/src/tests.rs                      M  +152 / −16 (CRLF)
f282e0921ba2cac881b6e2af64817eefe1098feb6f229202c5f050a2306a26ec  crates/tdb-canonical/src/snapshot_tests.rs             A  816 lines
94740a0f8ef0303f7d932e869d698abd0a2424114d5b92de8f29af316999f110  crates/tdb-canonical/src/legacy_xbox360_tests.rs       A  1191 lines
151c78bb958cd48003570ea342f8416777701254f8b76d3252eac161f86e28ec  crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml  M  +2 / −2
```

**Lock delta (exactly two lines, in the `tdb-canonical` package block, alphabetical):**

```
 name = "tdb-canonical"
 version = "0.0.0"
 dependencies = [
  "serde",
  "sha2",
+ "tdb-core",
  "tdb-qa",
+ "tdb-savedata",
  "toml 0.8.2",
 ]
```

HEAD's lock (LF) sha256 `d4c15e1c…`; the candidate's derived lock (CRLF archive representation)
`cd2158af…`. As in C0-COMMIT-EVID1, the staged lock should be constructed from `HEAD:Cargo.lock` +
these two lines (`git hash-object` / `update-index --cacheinfo`), **not** from the live
`Cargo.lock`, which carries 42 unrelated inserted lines of working-tree drift.

**Outside the boundary** (documentation, to be named separately if wanted):
`docs/receipts/c0-reg1a-legacy-xbox360-adapter.md` (supersession note),
`docs/receipts/c0-reg1a-r1-legacy-evidence-correction.md` (+§11 corrections),
`docs/receipts/c0-reg1a-r2-representation.md` (this), `docs/plans/project-completion-roadmap.md`
and `docs/STATE-OF-THE-REPO.md` (population reporting: 214 cases, R2 named). Everything else in the
working tree is pre-existing unrelated content and was not touched.

Root `Cargo.toml` is **not** in the boundary (HEAD's names no untracked member the slice needs).

## 8. Limits honoured

No staging, commit or push. No external repository, headmesh work, evidence publication, game
installation or capability promotion. One target. C0-REG1b not started. Stopping for Codex review.

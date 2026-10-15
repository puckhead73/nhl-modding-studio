# C0-EMIT1-R2 — verify fixture identity during evidence discovery

**Package:** C0-EMIT1-R2, closing the fixture-freshness gap left by [C0-EMIT1-R1](c0-emit1-r1-freshness-coverage.md) §6.
**Date:** 2026-09-15 (UTC) · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `58d1bc073bbf9e80d5afb4782bc6c132725e5fed` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** Discovery now hashes every fixture a run attributes — once per resolved file per
> invocation, only the referenced files, never a tree walk — and the merge compares the captured
> digests against those current states before deriving freshness. The rule is run-level and
> conservative: one changed fixture stales every observation of the run that read it; a missing,
> unreadable or unresolvable fixture, a run with no attribution, or an observation whose fixture
> hash is not among the run's inputs is `unknown` and never fresh. The four emitters now attribute
> every DB and save fixture they consume by repository-relative path with its content hash
> (`surface_matrix` and `save_matrix` gained attribution; `operation_matrix`'s directory-name
> attribution became the file path; `mutation_matrix` already did it). No persisted layer-A format
> changed; the provenance sidecar moved to schema 2 with the compared states and per-run verdicts.
>
> Proven three ways: **10 new end-to-end cases** (`evidence_fixtures`) through the real emitter,
> discovery and merge; **real recaptured runs** on a fresh candidate (4 `completed`, every
> observation's fixture attributed: 148 distinct paths, 466 MB hashed in ~10 s, `build` twice →
> byte-identical, disposable `publish` → `check` ok, 2 175 rows); and **mutation probes** on
> disposable copies with the same unchanged runs: one byte appended to `nhl15-ps3`'s `nhlng.db`
> (metadata and code untouched) → the three DB-reading runs `stale`, **140 → 0** qualified, the
> save run still `verified`; one byte in the ROSTER save → only the save run `stale`; the DB
> deleted → `unknown` / `fixture-missing`; a directory in its place → `unknown` /
> `fixture-unreadable`; restored → the published set **byte-identical**. The R1 runs, captured
> before attribution, merge as cited-but-`unknown` with `fixture-attribution-missing`.
>
> `tdb-qa` **247 passed · 0 failed · 1 ignored**; clippy `tdb-qa` and fmt clean; the overlay
> contributes zero clippy diagnostics; the `studio-domain` lib gate is **not** clean (8 HEAD
> diagnostics, C0.2 baseline).

---

## 1. Identity model

### 1.1 What is compared, and where

| Layer | Responsibility | Where |
|---|---|---|
| **Emitter** (capture) | attributes each consumed fixture on the run: `manifest.input_digests[<repo-relative path>] = <sha256 hex of the bytes read>`; every observation over that fixture carries the same hash in `fixture_sha256`; an unreadable fixture is attributed as the literal `unknown` | the four `studio-domain` emitters via `Emitter::input_digest` (unchanged API) |
| **Discovery** (I/O) | `store::verify_fixture_inputs(repo_root, &runs)`: for every distinct attributed path across all runs, classify what stands there now and hash it once per resolved file | `crates/tdb-qa/src/evidence/store.rs` — called by the CLI's `load()` and by every test that assembles a set from a root |
| **Merge** (pure) | `merge::run_fixture_verdict(run, &set.current_input_digests)` → a run-level `Freshness` plus findings; folded into the run's freshness with the scope digest: `combine([scope, fixtures])`, `Unknown` > `Stale` > `Fresh` | `crates/tdb-qa/src/evidence/merge.rs`; the merge reads no file |
| **Provenance** | the compared states under `fixture:<path>` in `merge_input_digests`, and a per-run `fixture_verification` map, in every sidecar and in `build-inputs.json` | `crates/tdb-qa/src/evidence/publish.rs` (schema `qa-evidence-provenance/2`) |

### 1.2 Exact attribution and comparison rules

**Attribution (capture).** A fixture path is the file actually read, spelled repository-relative
with `/` (`db_path.strip_prefix(repo(""))`), never a directory and never inferred from bytes or
detail text. `surface_matrix` and `mutation_matrix`/`operation_matrix` attribute the 27 anchor
databases; `save_matrix` attributes each of the 121 save fixtures it classifies. A unit with no
fixture records `skipped-missing-fixture` with an empty `fixture_sha256` and attributes nothing
for it — a missing fixture stays missing, it is not a pass lacking an optional hash.

**Discovery (`verify_fixture_inputs`).** For each distinct path a run names:

| Path / entry | `InputState` |
|---|---|
| empty, absolute, contains `..`, or a root/prefix component | `Unreadable("not a repository-relative path")` — nothing is read |
| `symlink_metadata` → `NotFound` | `Missing` (the one reading of absence, R-ABSENT) |
| link, junction or reparse point (`paths::entry_kind_of`, non-following) | `Unreadable(discovery-reparse-point …)` |
| directory / non-regular entry | `Unreadable("a directory, not a file")` / `Unreadable("not a regular file")` |
| regular file whose canonical path is not under the repository root | `Unreadable("resolves outside the repository")` — nothing is read |
| regular file, readable | `Present(sha256 hex)`, hashed **once**: a second attribution resolving to the same canonical file reuses the state |
| regular file, read error | `Unreadable("read: …")` |

None of these is an error for the caller: a missing or unreadable anchor is a legitimate state
the merge must report, not a reason to refuse `build`/`check`. The pass reports the number of
attributed paths, files hashed and bytes hashed (`fixtures: …` on stderr).

**Comparison (`run_fixture_verdict`), run-level:**

| Condition | Verdict | Finding (in the row's `unresolved`) |
|---|---|---|
| `input_digests` empty | `Unknown` | `fixture-attribution-missing:<run_id>` |
| an observation's non-empty `fixture_sha256` is not among the run's attributed digests | `Unknown` | `fixture-unattributed-observations:<run_id>:<count>` |
| an attributed digest is `unknown` / empty | `Unknown` | `fixture-digest-unknown:<path>` |
| the path has no current state (discovery did not run over it) | `Unknown` | `fixture-unverified:<path>` |
| `Missing` / `Unreadable` | `Unknown` | `fixture-missing:<path>` / `fixture-unreadable:<path>:<reason>` |
| `Present`, digest differs | `Stale` | `fixture-changed:<path>` |
| every attributed path `Present` and equal | `Fresh` | — |

Findings are attached to the row only for **selected** runs (a suppressed or failed run's fixture
state is not a claim about the row). The verdict enters freshness at contract §4.3 **row 6**
only: an executed failure (row 4a) and damaged evidence (row 4b) are decided first, so an unknown
fixture state can never erase a recorded failure (§3, `unknown_fixture_freshness_does_not_bypass_a_recorded_failure`).

### 1.3 Backward compatibility

* **Layer A is unchanged.** `manifest.json` already carried `input_digests`; `observations.csv`
  already carried `fixture_sha256`. No retained run is rewritten. A run captured before this
  package loads, validates and is cited exactly as before; it simply cannot be fresh
  (`fixture-attribution-missing`, or `fixture-unreadable` for R1's `operation_matrix`, which
  attributed a directory). Shown on the real R1 runs in §4.4.
* **Provenance** moved to `qa-evidence-provenance/2`: `fixture_verification` is new and
  `#[serde(default)]`, so a schema-1 sidecar still parses (tested). `build-inputs.json` gains the
  `fixture:` entries, so `publish`'s A30 re-verification now refuses if a fixture moved between
  `build` and `publish` — no new mechanism, the existing comparison covers the new keys.
* **`EvidenceSet` gains `current_input_digests`**; every constructor site was updated. The
  synthetic test kit (`testkit.rs`) now attributes one synthetic fixture on every run it builds and
  presents it as current, so the acceptance cases keep meaning "a fresh, attributed run qualifies"
  under the tightened rule instead of relying on the loophole this package closes. The synthetic
  emitters in the existing integration suites attribute one real read-only repository file
  (`defs/qa/scopes.toml`) for the same reason; what is attributed is irrelevant to those cases.

## 2. Files changed (over the C0-EMIT1 / R1 overlay)

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/model.rs` | `InputState` (+`render`), `EvidenceSet::current_input_digests` |
| `crates/tdb-qa/src/evidence/store.rs` | `FixtureVerification`, `verify_fixture_inputs` |
| `crates/tdb-qa/src/evidence/merge.rs` | `FixtureVerdict` (+`render`), `run_fixture_verdict`, folded into `automated_view` |
| `crates/tdb-qa/src/evidence/publish.rs` | `Provenance::fixture_verification` (schema 2), `BuildInputs::fixture_verification` |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | `load()` runs the verification, folds `fixture:` states into the merge inputs, reports counts |
| `crates/tdb-qa/src/evidence/testkit.rs` | synthetic runs attribute `FIXTURE_PATH`; `default_set` presents it |
| `crates/tdb-qa/tests/evidence_fixtures.rs` | **new**, 10 cases (§3) |
| `crates/tdb-qa/tests/{evidence_discovery,evidence_emit,evidence_file_reads,evidence_journal,evidence_publication,evidence_freshness}.rs` | set/inputs constructors updated; synthetic emitters attribute an input; `declared_pass` attributes a synthetic fixture |
| `crates/studio-domain/tests/surface_matrix.rs` | +`repo_relative`, `ev.input_digest(<db path>, …)` per anchor (15 lines vs R1) |
| `crates/studio-domain/tests/operation_matrix.rs` | attribution name: `anchors/<p>/<g>/databases` → the database file path (8 lines vs R1) |
| `crates/studio-domain/tests/save_matrix.rs` | `ev.input_digest(<save path>, …)` per fixture in the catalog (13 lines vs R1) |
| `crates/studio-domain/tests/mutation_matrix.rs`, `defs/qa/scopes.toml`, `evidence_emit.rs` cases, `evidence_freshness.rs` cases | unchanged since R1 (the last two only gained the constructor/attribution edits above) |

## 3. End-to-end regressions — `crates/tdb-qa/tests/evidence_fixtures.rs` (10 cases)

Each case copies the committed `defs/qa/scopes.toml` into a synthetic repository (stubs under every
glob; a synthetic `nhlng.db`, `nhlng-meta.xml` and ROSTER `SYS-DATA`), captures runs through the
real `Emitter` attributing the fixtures they consumed, then discovers with the real
`verify_fixture_inputs` and merges with the real merge. Only the synthetic repository is mutated.

| Case | Proves |
|---|---|
| `a_db_only_change_with_metadata_and_code_unchanged_stales_exactly_the_runs_that_read_it` | scope digests asserted unchanged; read + edit runs `stale` / withheld / `fixture-changed`; the save run stays qualified |
| `a_save_only_change_stales_the_save_run_and_nothing_else` | the inverse |
| `a_deleted_or_unreadable_fixture_is_unknown_and_never_fresh` | deletion → `Missing`, `unknown`, `fixture-missing`; a directory at the path → `Unreadable`, `fixture-unreadable`; restored byte-equal → qualified again with the run untouched |
| `a_path_that_is_not_repository_relative_is_unreadable_not_fresh` | a `..` path and an absolute path to a readable file *outside* the repository → both `Unreadable`, **0 files hashed**, `unknown` |
| `an_older_completed_pass_without_attribution_is_cited_but_cannot_qualify` | a pre-R2-shaped run (fixture hash on the observation, no `input_digests`): discovered, cited, selected, `automated = pass`, `fresh = unknown`, withheld, `fixture-attribution-missing`; nothing hashed; the run untouched |
| `an_observation_whose_fixture_is_not_among_the_attributed_inputs_cannot_be_fresh` | partial attribution → `fixture-unattributed-observations`, `unknown` |
| `unknown_fixture_freshness_does_not_bypass_a_recorded_failure` | a `fail` observation with its fixture deleted → `reason = unreviewed-failure` (row 4a), not `stale-evidence` |
| `unchanged_inputs_and_restored_bytes_reproduce_identical_results` | two discoveries over identical bytes → identical states and rows; mutate → differs; restore → identical to the first |
| `a_file_attributed_by_several_runs_is_hashed_once_per_invocation` | three attributions over two files → `files_hashed == 2`, `bytes_hashed ==` the two files' sizes; no runs → nothing read |
| `the_provenance_sidecar_records_what_was_compared` | schema 2 carries `fixture:<path> = sha256:…` and per-run `verified`; a schema-1 sidecar still parses |

Existing rules preserved by the unchanged suites: failure/supersession (acceptance 32, journal 31),
variant-parent (`evidence_emit`'s late-declaration case), damaged journals (R2/R3), discovery
(R4/R5). The freshness cases from R1 (7) still pass with their captures now attributing the DB.

## 4. Real captured evidence

### 4.1 Candidate

`git archive 58d1bc07…` (1 192 files) + the 19-path overlay of §6 + the one-line lock →
**1 194 files**, `…/scratchpad/emit1r2/candidate`; HEAD's root manifest; dedicated
`CARGO_TARGET_DIR`; `--locked --offline` on every command; `QA_WRITE` never set.

### 4.2 Recapture — all four, because no prior run keeps a valid identity

`surface_matrix` and `save_matrix` changed their persisted attribution; `operation_matrix` changed
its attribution name; and since `db-edit`'s `mapping` includes `operation_matrix.rs`,
`mutation_matrix`'s scope digest moved too. Reusing any R1 run would have been a stale identity.

| Emitter | Runtime | `run_id` | Scope · digest | Attributed inputs | Observations (fixture-hashed / unhashed) |
|---|---|---|---|---|---|
| `surface_matrix` | 7.0 s | `surface_matrix-20260915t045731z-0e282d69` | `db-read` v3 · `sha256:…` | **27** DB paths | 28 (27 / 1 skipped) |
| `mutation_matrix` | 110.0 s | `mutation_matrix-20260915t045741z-af3c664d` | `db-edit` v4 | 27 | 175 (174 / 1 skipped) |
| `operation_matrix` | 329.6 s | `operation_matrix-20260915t045936z-68e5714b` | `db-edit` v4 | 27 | 302 (297 / 5 skipped) |
| `save_matrix` | 80.1 s | `save_matrix-20260915t050508z-f41ca533` | `save-containers` v4 | **121** save paths | 447 (363 / 78 parent `not-attempted` + 6 skipped) |

Audit on the sealed runs (script over `manifest.json` + `observations.csv`): every attributed
path is repository-relative, exists in the candidate and re-hashes to the captured value; **0**
observations carry a fixture hash outside their run's inputs; the only unhashed observations are
`skipped-missing-fixture` and the deliberate variant-less parent cells. Totals unchanged from R1:
952 observations, 189 unmapped, 1 452 facets, 28 `skipped-missing-fixture`.

### 4.3 Deterministic build → disposable publication → check

| Step | Result | Exit |
|---|---|---|
| `check --data <empty>` | `fixtures: 148 attributed path(s), 148 file(s) hashed, 466 003 284 byte(s)`; 10 files differ | 2 |
| `build --out out1` | **11.3 s** wall (≈10 s hashing 466 MB with debug-profile `sha2`; R1's build was 1.5 s) | 0 |
| `build --out out2` → `diff -r` | **byte-identical**, 11/11 | 0 |
| `publish --from out1 --data data-scratch` | id `11f2c21f00372657`, prior generation retained | 0 |
| `check --data data-scratch` | **`ok: 2175 row(s), no validation failure, no drift`** | 0 |
| `publish --recover` | nothing to do | 0 |
| `build --out <evroot>/auto` | refused, `path-overlap` | 4 |

**Matrix:** 2 175 rows · **140 `automated-qualified`** (`db-presentation` 27 · `db-operations`
113 · 27 targets) · 159 `withheld` (`unknown-applicability`) · 36 `not-applicable` · 1 840
`unqualified` · **0 `game-verified`** · `fresh` 411 / `unknown` 1 764 · `unresolved` empty on
every row (no fixture finding on a verified set). **Provenance** (schema 2): 148 `fixture:` entries
all `sha256:…`; `fixture_verification` = `verified` for all four runs; `build-inputs.json` = 153
entries (4 runs + `manual/` + 148 fixtures).

**Cost.** 148 distinct files = 27 databases (shared by three runs, hashed once) + 121 save
fixtures; 466 MB per `check`/`build`/`publish`, ~10 s in the debug profile. The emitters
themselves are unaffected (they already hashed what they read). Test suites resolve fixtures
against the real repository only where a synthetic run attributes `defs/qa/scopes.toml` (a few KB);
`tdb-qa` suite wall time 2 m 41 s including compilation.

### 4.4 Mutation probes — disposable copy, same unchanged runs, `build` only

`candidate-mut` is a copy of the candidate; the evidence root is read-only throughout (manifest
identical before and after). Each row is one `build --repo candidate-mut --root evroot`:

| # | Mutation | hashed | `automated-qualified` | cited rows (fresh) | run verdicts |
|---|---|---|---|---|---|
| 0 | none | 148 files / 466 003 284 B | **140** | 411 fresh | 4 × `verified` — matrix **byte-identical to `out1`** |
| 1 | `anchors/ps3/nhl15/databases/nhlng.db` **+1 byte**; `nhlng-meta.xml` SHA-256 asserted unchanged, no code touched | 466 003 285 B | **0** | 168 `stale` (`db-presentation` 28, `db-operations` 140, all `fixture-changed`) · 243 fresh | surface/mutation/operation `stale`, save `verified` |
| 2 | ROSTER `SYS-DATA` **+1 byte** only | 466 003 285 B | 140 | 243 `stale` (`save-inner-container`, `fixture-changed`) · 168 fresh | save `stale`, others `verified` |
| 3 | `nhlng.db` **deleted** | 147 files | **0** | 168 `unknown` (`fixture-missing`) · 243 fresh | three `unknown`, save `verified` |
| 4 | a **directory** at `nhlng.db` | 147 files | **0** | 168 `unknown` (`fixture-unreadable`) · 243 fresh | three `unknown`, save `verified` |
| 5 | everything restored | 148 / 466 003 284 B | 140 | 411 fresh | 4 × `verified` — matrix **byte-identical to `out1`** |
| 6 | the **R1 runs** (`…/emit1r1/evroot`, pre-attribution) against this candidate | 54 paths, 27 hashed | **0** | 411 `unknown`: 271 `fixture-attribution-missing`, 140 `fixture-unreadable` | mutation `verified`; operation `unknown:fixture-unreadable:anchors/ps3/nhl07/databases:a directory…`; surface/save `unknown:fixture-attribution-missing:…` — all still cited by their original ids |

Row 1 is the objective: a database edit with metadata and code untouched cannot leave a dependent
row fresh — and it stales exactly the runs that read the database. Row 6 is the legacy rule on
real material: R1's evidence stays readable and cited, its root untouched (4 runs), and gains
nothing through the empty-input loophole. The run-level conservatism is visible in rows 1 and 3–4:
one `nhl15-ps3` fixture stales all 27 anchors' rows for the three DB runs; per-observation
precision is a possible later refinement, not a prerequisite.

## 5. Validation, baseline vs regression, preservation

| Check (candidate) | Result | Classification |
|---|---|---|
| `cargo test -p tdb-qa --locked --offline` | **247 passed · 0 failed · 1 ignored** (72 unit · 32 acceptance · 13 discovery · 9 emit · 9 file-reads · **10 fixtures** · 7 freshness · 31 journal · 19 + 1 ignored migration · 36 publication · 9 inventory) | clean |
| `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | exit 0 | clean |
| `cargo fmt -p tdb-qa -- --check` | exit 0 | clean |
| `rustfmt --check` on all 17 overlay `.rs` files | all clean | clean |
| `cargo clippy -p studio-domain --no-deps --test {4 emitters} -- -D warnings` | exit 101: **8** diagnostics in `studio-domain/src/{assets ×3, attributes, formula, head_catalog ×2, view}.rs` | **baseline** — HEAD lib sources, C0.2; the gate is not clean and this package does not claim it |
| same, warnings non-fatal, by path | the four emitter targets contribute **0** diagnostics (`tdb-rx2/src/playermesh.rs` 1, HEAD) | overlay clean |
| Candidate manifest before → after | 1 194 files, **0 differing**; no `in-progress/`, `.qa-prev-*` or `docs/audits/evidence` under it | — |
| Live `anchors/**` + `docs/audits/**` | 621 files, **0 mismatches**; `docs/audits/evidence` absent | — |
| Live `git status --porcelain` | 269 → 269 (the package's edits were already present when the baseline was taken; nothing else changed) | — |
| HEAD / index / push | `58d1bc07…` / empty / none | — |
| Retained evidence (`…/emit1/evroot`, `…/emit1r1/evroot`, `H:\c0-s2-int1\…`) | read only; 4 + 4 runs unchanged | — |

## 6. Final commit candidate — files, hashes, lock delta

Baseline HEAD + this overlay = the accumulated EMIT1 / R1 / R2 slice. **20 paths: 2 A · 18 M.**
SHA-256 as placed in the candidate (CRLF, the archive representation); `git hash-object` equals
the live file's blob in every case (the live `Cargo.lock` differs only by its unrelated drift).

| Path | Op | SHA-256 (candidate) | Blob |
|---|---|---|---|
| `crates/studio-domain/Cargo.toml` | M | `e87e8f12a7ceac18…` | `af8e2b5d` |
| `crates/studio-domain/tests/surface_matrix.rs` | M | `ed48dc8f1afc3bc3…` | `e1586e25` |
| `crates/studio-domain/tests/mutation_matrix.rs` | M | `b90c921e0a740027…` | `6e0117af` |
| `crates/studio-domain/tests/operation_matrix.rs` | M | `3ef283efd526a4fe…` | `26e14def` |
| `crates/studio-domain/tests/save_matrix.rs` | M | `d9fb9d2f9cb50423…` | `51e693cf` |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | M | `ef916551b3830048…` | `a145e738` |
| `crates/tdb-qa/src/evidence/merge.rs` | M | `377b9a52c71918c8…` | `0e2f1bb1` |
| `crates/tdb-qa/src/evidence/model.rs` | M | `d9be5e07d03020f6…` | `1b5ecf96` |
| `crates/tdb-qa/src/evidence/publish.rs` | M | `f890a2c07cadf11c…` | `88603971` |
| `crates/tdb-qa/src/evidence/store.rs` | M | `5370e1ee09a386b0…` | `b42aee27` |
| `crates/tdb-qa/src/evidence/testkit.rs` | M | `acaa610b4fdd1416…` | `60fd6b88` |
| `crates/tdb-qa/tests/evidence_discovery.rs` | M | `d005f393a7d6b11d…` | `1709182c` |
| `crates/tdb-qa/tests/evidence_emit.rs` | M | `7d01ab5058452256…` | `40e965e5` |
| `crates/tdb-qa/tests/evidence_file_reads.rs` | M | `f3ac9d81464c61d6…` | `833f6b32` |
| `crates/tdb-qa/tests/evidence_journal.rs` | M | `1e6898d06543ee3e…` | `26f390ec` |
| `crates/tdb-qa/tests/evidence_publication.rs` | M | `79270d56facc881f…` | `c1b83602` |
| `crates/tdb-qa/tests/evidence_fixtures.rs` | **A** | `2dabf65ecf2f8ba6…` | `71102f04` |
| `crates/tdb-qa/tests/evidence_freshness.rs` | **A** | `9bf71c9f760cb847…` | `84decdd2` |
| `defs/qa/scopes.toml` | M | `71c944736355f61b…` | `1a2bf8a1` |
| `Cargo.lock` | M | `4a4cc094aa6f2776…` | `8decfca0` |

Full 64-character hashes: `…/scratchpad/emit1r2/logs/overlay-manifest.txt`.

**Lock delta — exactly one line**, built from `HEAD:Cargo.lock`, unchanged since EMIT1:

```
@@ -3372 +3373 @@ name = "studio-domain"
+ "tdb-qa",
```

Root `Cargo.toml` is HEAD's. Documentation (this receipt and the EMIT1/R1 receipts) stays outside
the commit. Subject proposal: `feat(qa): wire matrix emitters to the evidence observation lifecycle`,
with the freshness coverage and fixture verification described in the body.

## 7. Remaining decisions vs. implementation defects

**Activation decisions (architect):**

| Area | Status |
|---|---|
| Storage (U3), selected-run retention / recapture (D2) | unchanged: `QA_EVIDENCE_ROOT` must be outside the repository; the published generation now names every fixture state it compared, but the runs it selects still live only in that root |
| Consumer transition (R-TRANSITION, Q-C) | the `QA_WRITE=1` in-place CSV writes are untouched; the observe call is parallel |
| CI (R-NO-CI) | nothing activated; a clean-checkout gate additionally needs the `studio-domain` lib clippy baseline (8 diagnostics) and Windows CI (C0.2 R6) |
| Per-observation fixture precision | optional refinement: today one changed anchor stales a whole run (all 27 anchors' rows for the DB emitters); an observation-level comparison against `fixture_sha256` would narrow that. Not a prerequisite |
| Hashing cost policy | 466 MB / ~10 s per `check`/`build`/`publish` in the debug profile. Acceptable now; if it is not later, the options are a release-profile CLI or a size-bounded policy — a decision, not a defect |
| Q1–Q4 / R3 | open, unchanged |

**Known non-defects / cosmetics carried:** the `save_matrix` parent-cell detail spacing (EMIT1 §4.4)
and the same-class spacing in two `paths.rs` rule messages — payload only; untouched here.

**Implementation defects found:** none open. The `operation_matrix` directory-path attribution
that R1's runs carried (now visible as `fixture-unreadable:…:a directory, not a file`) was a
defect in the previous attribution and is corrected in this overlay.

## 8. Limits honoured

No staging, commit or push; no live migration or publication (`docs/audits/data` and
`docs/audits/evidence` never named as a target); no CI change; no game installation; no headmesh
work; no external repository; no registry expansion; no baseline repair. Mutations happened only in
disposable copies; live anchors and retained evidence roots are byte-unchanged. Stopping for Codex
review.

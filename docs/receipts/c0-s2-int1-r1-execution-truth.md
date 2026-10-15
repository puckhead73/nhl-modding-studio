# C0-S2-INT1-R1 — Preserving incomplete execution evidence

> **Package:** C0-S2-INT1-R1, a bounded repair of the defects Codex review found in
> [C0-S2-INT1](c0-s2-int1-emitter-integration.md). **Date:** 2026-09-08.
> **Author role:** developer. Reviewer: Codex (architect).
> **Status: SUPERSEDED IN PART — see the review correction below.** Original status was "Passed".

## ⚠ Review correction (C0-S2-INT1-R2, 2026-09-08)

Three integrity statements in §4 of this receipt are **corrected**; see
[c0-s2-int1-r2-integrity-gates.md](c0-s2-int1-r2-integrity-gates.md) §1.

1. **"A record file that is absent → empty" was applied to every read failure.** A directory at
   `observations.csv`, a permission error and invalid UTF-8 all reported "the emitter recorded
   nothing". Only `NotFound` may mean absent; everything else is now a hard error.
2. **Dropping a corrupt record and reporting a token was not sufficient.** An older completed pass
   could stay `automated-qualified` on the very key whose newer evidence had been destroyed.
   Damaged evidence now withholds across the journal's declared scope (qualification row 4b), and
   damaged *declarations* — which is what bounds that scope — are fatal.
3. **"The separator is a byte that cannot occur in a CSV field value" is false.** A quoted CSV field
   may contain any byte, including that one: `["a\x1fb"]` and `["a", "b"]` hashed
   identically under `/2`. The record
   framing is now length-prefixed with escaped fields and is versioned `qa-run-journal/3`.

Additionally, §3's deduplication rested on a **matching `sealed_run_id` string alone**, which let a
wrong or planted id delete a journal's evidence. Equivalence is now verified before deduplicating.

**No figure in this receipt changes.** Rebuilding its real evidence under R2 produces a
byte-identical published set, and its 140 `automated-qualified` rows stand.

> Nothing staged, committed or pushed. **No CI activation, no live publication,
> no migration installation.** Every run, publication and recovery here ran in an isolated
> directory outside the repository. `QA_WRITE` was never set.
>
> **S2 remains partial.** This package does not close it, and §8 records what is still open.

---

## 1. Corrections to the previous receipt

The C0-S2-INT1 receipt overstated two things. Both are withdrawn here, in its own terms.

### 1.1 "215 automated-qualified" did not survive an honest freshness scope

That receipt reported **215 `automated-qualified` rows over 27 targets** as the first real merge.
Those observations declared `db-read` / `db-edit` / `save-containers` at **v1**, and v1 omitted
dependencies their assertions actually exercise — `Capabilities::probe`, `RosterModel::build`,
`Project::load`, the invariant audit, and the emitter test sources that carry the coverage-key
mapping itself (§6). With the scopes corrected and versioned, building over **the same preserved
runs** now yields:

| | C0-S2-INT1 claim | Same runs, corrected scopes |
|---|---|---|
| `automated-qualified` | **215** | **0** |
| `withheld` | 0 | **246**, every one `stale-evidence` |
| `fresh` | 252 rows | **0 rows** |

Nothing about those runs changed; what changed is that their declared freshness scope now names
what they actually depend on. **The 215 figure should not be cited.** The runs are retained as
historical evidence exactly as instructed, and they are correctly not-fresh.

### 1.2 "All four ran to completion" and "no fixture was unavailable" are different claims

That receipt wrote: *"All four ran to completion — **no emitter was blocked and no fixture was
unavailable**."* The first half is true; the second half is contradicted by the receipt's own
numbers, which report **28 `skipped-missing-fixture` observations** in the same table. Stated
correctly, and re-measured on this package's run:

* **All four emitters completed.** Every one sealed `completed`; none was blocked.
* **Not every declared fixture existed.** 28 of the 952 observations are
  `skipped-missing-fixture`: 1 per emitter for `legacyrecomp-recomp`, which has no anchor tree at
  all, plus 21 in `operation_matrix` (`Status::NoFixture` — the schema carries the feature but the
  anchor ships no entity to exercise it) and 6 in `save_matrix` (units with no save fixtures).

A missing fixture is a recorded skip, never a pass, and "the emitter finished" says nothing about
whether the material it needed was there. The two claims are tracked separately from here on.

---

## 2. What was wrong, and what it is now

| # | Defect | Repair |
|---|---|---|
| **F1** | An unsealed journal was invisible to the merge. A hard kill left evidence on disk that `check` merely *named*; nothing read it, so a persisted failure vanished. | New [`evidence::journal`](../../crates/tdb-qa/src/evidence/journal.rs): journals load **read-only** as incomplete execution evidence — cited, never selected, never sealed (§3). |
| **F2** | `persist_observations` rewrote the entire CSV with `fs::write` on every record, so one interrupted write destroyed **every** previously persisted observation, and a failure was only `eprintln!`ed. | Append-only records with a per-record digest (§4), plus a persistence-failure workflow that cannot report success. |
| **F3** | The seven unmapped `operation_matrix` operations executed and were then discarded — a source comment was the only trace. | `unmapped.csv` per run and journal, published to `qa-evidence-unmapped.csv`; 189 real records this run (§5). |
| **F4** | Container codec and save mode lived in free text, and every extract/repack failure was attributed to `container-repack` regardless of which assertion failed. | Codec in the key's `variant` component + `facets.csv`; detect / extract / repack are three separate cases (§6). |
| **F5** | The freshness scopes omitted the emitters' real dependencies **and** their own mapping logic, which is what made §1.1 possible. | Scopes widened, a `mapping` input class added, and `db-read` / `db-edit` / `save-containers` versioned to **v2** (§7). |

### Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/journal.rs` | **New.** Journal format, append-only record integrity, `load_journals`, `journal_bytes`. 2 unit tests. |
| `crates/tdb-qa/src/evidence/emit.rs` | Append-only persistence, atomic identity, `persist_error` workflow, `observe_faceted`, `observe_unmapped`, `case_unmapped`, `sealed_run_id` linkage. |
| `crates/tdb-qa/src/evidence/model.rs` | `Journal`, `UnmappedObservation`, `Facet`; `Run.unmapped` / `.facets`; `EvidenceSet.journals`. |
| `crates/tdb-qa/src/evidence/merge.rs` | Journals as incomplete evidence: citations, axes, uncleared-failure participation. |
| `crates/tdb-qa/src/evidence/store.rs` | `unmapped.csv` / `facets.csv` read + write; `Result` made public. |
| `crates/tdb-qa/src/evidence/paths.rs` | `append_file` — the guarded append path. |
| `crates/tdb-qa/src/evidence/publish.rs` | `qa-evidence-unmapped.csv` + `UnmappedRow`. |
| `crates/tdb-qa/src/evidence/vocab.rs` | `applicability_for_key` — the variant rule. |
| `crates/tdb-qa/src/evidence/scopes.rs` | `mapping` input class. |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | Loads journals and unmapped records; a malformed active input is now exit **3**. |
| `crates/tdb-qa/tests/evidence_journal.rs` | **New.** 14 cases. |
| `crates/tdb-qa/tests/{evidence_emit,evidence_publication}.rs` | Adopt the new fields; file-count assertions derived from `OUTPUT_FILES`. |
| `crates/studio-domain/tests/operation_matrix.rs` | Unmapped executions recorded; assertions unchanged. |
| `crates/studio-domain/tests/save_matrix.rs` | Extract/repack proofs split; variant keys and facets; conservative parent cells. |
| `defs/qa/scopes.toml` | `db-read`, `db-edit`, `save-containers` → **v2**, widened. |

---

## 3. Item 1 — incomplete evidence, visible to the merge

A journal is loaded for exactly what is on disk: *this emitter declared these keys and persisted
these observations*. It asserts nothing about whether the process is alive, and **nothing seals it**
— inventing a terminal state for a possibly-running emitter would be a fabricated `interrupted`.

| Rule | How |
|---|---|
| Never selected | citation `kind = auto-journal`, `state = incomplete`, `selected = false`, always |
| Incomplete pass confers nothing | journal observations never enter `selected_outcomes`; the axis is `executed-unselected` |
| Persisted failure blocks | §12.3 clause 2 clears a failure only via a **completed run that supersedes the observer**, and no run supersedes a journal — so a journal `fail` is uncleared by construction and fires qualification row 4a |
| Declaration ≠ execution | a declared-only key is `declared-not-executed` + `declared-in-unsealed-journal-not-executed:<id>`; an executed one is `executed-unselected` + `executed-only-in-unsealed-journal:<id>` |
| No double counting | a seal writes `sealed_run_id` into `journal.json` **before** removing the directory; a journal naming a present run is skipped, one naming an absent run is loaded and flagged `sealed-run-missing` |
| Source bytes preserved | the module only reads; `journal_bytes` proves it in the regression |

**The headline case, tested:** a completed run passes `nhl15-ps3/db-presentation/read`
(`automated-qualified`), then an unsealed journal persists a `fail` on the same key. Result:
`automated` stays `pass` and the run stays cited and selected, but qualification becomes
**`withheld`, `reason = unreviewed-failure`**, with both sources cited. The failure does not
disappear behind the older completed pass.

**Demonstrated against a genuinely live emitter.** While `mutation_matrix` was still running,
`qa-evidence build` read its journal: 23 keys `executed-unselected`, 5 `declared-not-executed`,
**none qualified**, while the already-sealed `surface_matrix` run kept its 27
`automated-qualified` rows. The journal was untouched by the merge and kept growing (114 → 160
records), and no run was sealed for it.

---

## 4. Item 2 — recoverable persistence

**Append-only.** Every declaration, observation, unmapped record and facet is appended; nothing is
rewritten. `journal.json` is the one exception and is written **atomically** (temp + rename), so
identity is either whole or absent. Every append goes through the guarded `paths::append_file`.

**Per-record integrity.** Each appended record carries a `record_sha256` over its own fields,
separated by a byte that cannot occur in a CSV value. That separates three kinds of damage:

| Damage | Handling | Tested by |
|---|---|---|
| trailing record with no newline (an interrupted append) | dropped; everything before it kept; `truncated-trailing-record:<file>` | `an_interrupted_append_costs_only_the_record_it_was_writing` |
| a record whose digest does not match | that record dropped, **neighbours on both sides survive**; `corrupt-record:<file>:<line>` | `a_corrupt_record_is_dropped_and_reported_while_its_neighbours_survive` |
| a record that parses but whose key does not | dropped; `unparseable-key:<file>:<line>` | loader |
| `journal.json` absent / unparseable / wrong schema / id ≠ directory | **fail closed** — hard error naming the file; no material is attributed | `a_journal_whose_identity_cannot_be_established_fails_closed` (4 variants) |

Nothing is repaired in place, nothing is silently ignored: every drop surfaces as a
`Journal::integrity` finding and as `journal-integrity-findings:<id>` on every row the journal
mentions. A malformed active input is now exit **3**, the contract's code for it.

**Persistence failure is a non-success workflow.** The first failed append is recorded; the next
observation call panics naming the retained journal, and `finish()` seals the run `failed` and
panics. Verified by injecting a directory at the target file name mid-run: the workflow panicked
with *"could not persist … retained at …"*, the run sealed **`failed`**, and the record written
before the fault survived.

**No second panic during unwinding.** `check_persistence` returns immediately when
`std::thread::panicking()`, and the drop guard's seal is wrapped in `catch_unwind`. An original
assertion panic is always resumed with its own payload.

**Remaining window, stated.** Nothing is flushed to the device, so this is durability against
*process* interruption, not power loss. There is no concurrency control. A hard kill still leaves an
unsealed journal — but that is now evidence the merge reads, not a silent gap.

---

## 5. Item 3 — unmapped executions preserved

The seven `operation_matrix` operations with no operation in the closed `db-operations` vocabulary
are recorded as `UnmappedObservation`s: **no coverage key**, so they enter no axis, no roll-up and
no qualification — and no adjacent key is invented. Each carries the exact emitter operation id,
the target, the case id, the outcome, the fixture digest and the reason
`operation-not-in-closed-db-operations-vocabulary`.

This run produced **189 published unmapped records** — 27 anchors × 7 operations:

| Operation | Records | Outcomes across the 189 |
|---|---|---|
| `add-remove-membership`, `autocorrect-jerseys`, `career-add-delete`, `release-then-sign`, `repair-roster-blocks`, `retire-unretire-jersey`, `trade-draft-pick` | 27 each | 157 `pass`, 16 `refused-clean`, 16 `skipped-missing-fixture` |

They reach layer C as `qa-evidence-unmapped.csv` (a **fifth**, additive published file; contract
§8.1 names four — flagged in §8). A regression asserts the matrix mentions none of them.
`operation_matrix`'s existing assertions are unchanged, and its CSV is byte-identical.

---

## 6. Item 4 — variants structural, attribution correct

**Structural, not prose.** Every `save_matrix` observation carries the codec in the key's `variant`
component (`v1`–`v4`, `ps4-lz4`, `raw-db`, `unsupported`) and repeats codec, container version, save
mode and slot as **facets** — 1452 facet records this run.

**Parent qualification stays conservative.** `catalog_universe()` enumerates variant-less cells and
no accepted document declares which variants a target must cover, so `applicability_for_key` returns
`unknown` for any variant key (withheld, §4.3 row 2) and the variant-less parent is never satisfied
by variant evidence. The research-only rule still wins over the variant rule, so `nhl07-ps3` keeps
`not-applicable` with its cited reason. **No required-variant catalog was invented.**

The measured effect is a deliberate *reduction*: `save-inner-container` contributed **75
`automated-qualified` rows** under the previous receipt and contributes **0** now. All 159 variant
rows are `withheld / unknown-applicability`.

**Attribution.** `container_roundtrip` is split into `container_extract_proof` and
`container_repack_proof` (and `ps4_extract_proof` / `ps4_repack_proof`), each run in its own
`Emitter::case` against its own key — so a failure attaches to the operation whose assertions
failed, not to repack by default. Refusal is never read as success: an unsupported or loose-DB
payload records `refused-clean` on **detect** and `not-attempted` on extract and repack, with the
detail saying so explicitly.

---

## 7. Item 5 — freshness dependencies audited

| Scope | v1 declared | v2 adds | Why |
|---|---|---|---|
| `db-read` | `view.rs`, `surfaces.rs`, schema | `capabilities.rs`, `crates/tdb-relations/src/**`, **mapping** `tests/surface_matrix.rs` | the read assertions drive `Capabilities::probe` and `RosterModel::build`, and the key mapping is in the test |
| `db-edit` | `tdb-edit`, `session.rs` | `project.rs`, `tdb-relations`, `tdb-domain-validation`, **mapping** `tests/{mutation,operation}_matrix.rs` | operations open through `Project::load`, rebuild the model and are gated on the invariant audit |
| `save-containers` | `tdb-savedata` | `tdb-edit`, `session.rs`, `project.rs`, **mapping** `tests/save_matrix.rs` | the repack proof takes a real `EditSession` edit through the container |

A new `mapping` input class names the assertion/mapping logic itself: an observation is a claim
about *what a case asserted*, so changing the assertion invalidates it exactly as changing the
implementation does. Proven over a synthetic repository —
`changing_the_assertion_or_mapping_logic_makes_prior_evidence_stale` builds two repos differing only
in an assertion body, gets different digests, and shows the merge turn that into
`stale-evidence / withheld` rather than silence. **Nothing was narrowed to obtain a known digest;**
every change widens a scope, which is what produced §1.1's correction.

---

## 8. Verification

| Check | Command | Result |
|---|---|---|
| New lifecycle / merge / mapping cases | `cargo test -p tdb-qa --test evidence_journal` | **Passed** — 14/14 |
| Emitter lifecycle (C0-S2-INT1) | `cargo test -p tdb-qa --test evidence_emit` | **Passed** — 7/7 |
| Whole `tdb-qa` crate | `cargo test -p tdb-qa` | **Passed** — **174**, 0 failed, 0 ignored |
| Four emitters, **default mode** (no `QA_EVIDENCE_ROOT`, no `QA_WRITE`) | `env -u … cargo test -p studio-domain --test <each>` | **Passed** — 4/4, all four CSVs byte-identical |
| Four emitters, observation on | `QA_EVIDENCE_ROOT=H:/c0-s2-int1/run3 …` | **Passed** — 4/4 sealed `completed` |
| Strict clippy, scoped | `cargo clippy -p tdb-qa -p studio-domain --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, repo-wide | `cargo fmt --all -- --check` | **Passed**, exit 0 |
| DTO bindings | `export-bindings -- --check` | **Passed** — 70 roots, 124 files, in sync |
| Full workspace suite | — | **Not run.** The only non-test change outside `tdb-qa` is a dev-dependency line. |

### The corrected-semantics run (`H:\c0-s2-int1\run3`, isolated)

| Emitter | Runtime | Declared | Executed | Observations | Unmapped | Facets |
|---|---|---|---|---|---|---|
| `surface_matrix` (`db-read` v2) | 6 s | 28 | 27 | 28 — 27 pass, 1 missing fixture | 0 | 0 |
| `mutation_matrix` (`db-edit` v2) | 100 s | 28 | 27 | 175 — 147 pass, 27 refused-clean, 1 missing fixture | 0 | 0 |
| `operation_matrix` (`db-edit` v2) | 305 s | 140 | 119 | 302 — 271 pass, 10 refused-clean, 21 missing fixture | **189** | 0 |
| `save_matrix` (`save-containers` v2) | 69 s | 243 | 116 | 447 — 190 pass, 74 refused-clean, 177 not-attempted, 6 missing fixture | 0 | **1452** |

**952 observations, 189 unmapped records, 1452 facets, 4 completed runs.** Runtimes match the
pre-change figures, so append-only journalling costs nothing measurable. No journal remained after
the runs — each seal linked and removed its own.

**End to end:** `build` twice → **byte-identical (10/10 files)**; `publish` → `check` reports
`ok: 2175 row(s), no validation failure, no drift`, exit 0; `publish --recover` clean; `build` into
the evidence root refused (`path-overlap`).

**The published matrix (2175 rows = 2016 universe + 159 variant cells):**

| Axis | Distribution |
|---|---|
| `qualification` | **140 `automated-qualified`** (db-operations 113, db-presentation 27, across 27 targets) · 36 `not-applicable` · 159 `withheld` · 1840 `unqualified` · **0 `game-verified`** |
| `automated` | 230 pass · 32 mixed · 121 declared-not-executed · 28 skipped-missing-fixture · 1764 not-attempted |
| `applicability` | 978 required · 1002 optional · 36 not-applicable · **159 unknown** (every variant cell) |
| `fresh` | 411 fresh · 1764 unknown |

Down from the previous receipt's 215 — `save-inner-container`'s 75 rows are now withheld by the
conservative variant rule, and nothing was promoted anywhere.

### Preservation

| Check | Measured how | Result |
|---|---|---|
| HEAD | `git rev-parse HEAD` | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25`, unchanged |
| Index | `git diff --cached --name-only` | **empty** |
| `anchors/**` | 552 files, `sha256sum -c` vs the pre-package list | **0 mismatches** |
| `docs/audits/**` | 69 files, same method | **0 mismatches** — the four emitter CSVs included |
| `git status --porcelain` vs package start | diffed | **4 added entries**, all intended (`M crates/studio-domain/Cargo.toml`; `?? crates/tdb-qa/build.rs`; `?? crates/tdb-qa/tests/evidence_emit.rs`; `?? crates/tdb-qa/tests/evidence_journal.rs`). **0 removed.** |
| Pre-existing dirty/untracked work | same diff | **preserved** — nothing reset, restored, cleaned, stashed or checked out |
| C0-S2-INT1's runs (`H:\c0-s2-int1\run1`) | 4 run directories present | **retained unchanged** as historical evidence |
| Layer B (`manual/`) | provenance digest | empty-tree digest; never written |
| Live publication / migration / CI / staging / commit / push | — | **none** |

---

## 9. Referred to the architect

| # | Item |
|---|---|
| **R1** | `qa-evidence-unmapped.csv` is a **fifth** published file; contract §8.1 names four. Additive, and no existing file's schema changed — but it is a contract-visible extension. |
| **R2** | The axis vocabulary has no value for *"declared by a still-running emitter"*. `declared-not-executed` is used, with a precise `unresolved` token beside it, because inventing an axis value is a contract change. |
| **R3** | Making variant evidence qualify anything needs an explicit **declared variant catalog** per target. Until one exists, variant cells are `unknown` and parents are unsatisfied — the conservative reading, and the reason the qualified count fell to 140. |
| **R4** | C0-S2-INT1's §8 questions **Q1** (should `mutation_matrix` also claim `save-raw-db`?), **Q2** (extend the `db-operations` vocabulary for the seven unmapped operations, or accept the gap? — now countable: 189 records), **Q4** (should an unsealed journal be sealable as `interrupted`? — still deliberately not) and **Q5** (U3) are unchanged and still open. |
| **R5** | Journals are read by the merge but are **not** subject to V1–V12; their integrity is enforced by the loader (fail-closed identity, per-record digests). Whether they should carry validation rules of their own is a contract question. |

## 10. Status

| Item | Status |
|---|---|
| **S2** | **Still partial.** This package makes incomplete evidence honest; it does not close S2. No CI gate is proposed or activated (R-NO-CI). |
| C0 | **Not complete.** |
| The C0-S2-INT1 receipt | corrected in §1: its 215-qualified figure and its "no fixture was unavailable" claim are withdrawn. Its runs are preserved and are now correctly stale. |
| Carried-over gaps | no power-loss durability, no concurrency control, hardlink aliasing undetected, file-symlink reparse coverage unavailable, canonical adapter format gaps, checkpoint 7's unmeasured write surface, and every open product question. |

Stopping for Codex review.

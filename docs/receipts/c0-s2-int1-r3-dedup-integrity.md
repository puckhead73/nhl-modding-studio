# C0-S2-INT1-R3 — integrity-aware deduplication and discovery

**Package:** C0-S2-INT1-R3. **Result:** complete; two defects corrected, six regression cases added,
both guards proved load-bearing.
**Branch:** `feat/phase5-asset-tooling`. **HEAD at start and at end:** `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` (unchanged).
**Not authorized and not done:** staging, commit, push, CI change, live publication, migration installation.

This correction is deliberately narrow. R2's record framing, integrity gates and read-only journal
handling are unchanged; only the two paths named in the package were touched.

---

## 1. The combined-state defect

### 1.1 What was wrong

`load_journals` evaluated two conditions that R2 had made independent, and let one of them decide:

```rust
let findings = equivalence_findings(&journal, run);
if findings.is_empty() {
    continue; // proved identical; counting it twice would be a fabrication
}
```

`equivalence_findings` compares the journal's **loaded** records against the run. A damaged record is
never loaded — `load_journal` drops it and reports it in `Journal::integrity` — so the comparison
sees only the survivors. When those survivors all matched the named run, the branch fired and the
journal was dropped from the evidence set entirely.

Two things were lost with it, and the second is the serious one:

1. the integrity findings themselves, so nothing reported the damage; and
2. the journal's **withholding effect** — R2's qualification row 4b withholds across a damaged
   journal's declared scope, and a journal that has been deduplicated away withholds nothing.

The result was that a damaged journal carrying a valid `sealed_run_id` **silently qualified** the
keys it should have withheld. The exact evidence R2's row 4b exists to distrust was discarded on the
strength of the records that happened to survive.

The reasoning error is a small one: **a surviving-record subset cannot prove equivalence for the
records that were dropped.** The dropped records are precisely the ones equivalence has no access
to. Their content, key and outcome are unknown — that is what "damaged" means — so the agreement of
everything else says nothing about them.

### 1.2 The fix

`crates/tdb-qa/src/evidence/journal.rs`, in `load_journals`:

```rust
// Damage is decided *before* anything is appended below, so the guard reflects the
// journal's own condition and not the outcome of this comparison.
let damaged = !journal.damaging_findings().is_empty();
match runs.iter().find(|r| r.run_id() == run_id) {
    Some(run) => {
        let findings = equivalence_findings(&journal, run);
        if findings.is_empty() && !damaged {
            continue; // proved identical; counting it twice would be a fabrication
        }
        if findings.is_empty() {
            journal.integrity.push(format!(
                "sealed-run-unverifiable:{run_id}:damaged records cannot be compared, \
                 so the surviving subset does not prove equivalence"
            ));
        } else {
            journal.integrity.push(format!(
                "sealed-run-mismatch:{run_id}:{}", findings.join("; ")
            ));
        }
    }
    None => journal.integrity.push(format!("sealed-run-missing:{run_id}")),
}
```

Notes on the exact semantics:

* `damaged` is computed **before** any finding is appended in this block, so the guard reflects the
  journal's own condition, not a token this code just added.
* `damaging_findings()` is R2's existing filter: it excludes `sealed-run-missing:` (a link that
  cannot be checked, whose records are nonetheless digest-verified) and includes everything else.
* A damaged-but-matching journal gets a **new, distinct** finding, `sealed-run-unverifiable:`, so the
  reason it survived deduplication is recorded rather than inferred. It is a damaging finding, which
  is correct: the journal was already damaged, and this only names why it could not be dismissed.
* **Clean deduplication is untouched.** A journal with no damaging findings whose records match the
  run it names is still dropped, still cited once, and still qualifies — regression 3 below.

`docs/plans/qa-evidence-contract.md` is unchanged: this is a defect in the implementation of the
existing dedup rule, not a change to the rule. The module documentation in `journal.rs` §"Double
counting" now records the distinction explicitly.

---

## 2. Fail-closed journal discovery

### 2.1 What was wrong

```rust
let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)?
    .flatten()                    // a failed iteration step vanishes
    .map(|e| e.path())
    .filter(|p| p.is_dir())       // is_dir() follows links, and a failed stat reads as "not a dir"
    .collect();
```

Three silent losses, each of which removes a journal — and any withholding it carried — from the
evidence set without a word:

* `.flatten()` discards an `Err` from the directory iterator. Enumeration failing part-way is not the
  end of the listing; journals after that point are simply never seen.
* `Path::is_dir()` returns `false` when the metadata read *fails*, so a permission error or a
  vanished entry is indistinguishable from a plain file and is dropped.
* `is_dir()` follows symlinks, so a link was silently resolved through, while a broken link was
  silently dropped.

### 2.2 The fix

Discovery is now explicit, and split into a filesystem step and a pure classification step so both
are testable:

```rust
pub const PERMITTED_AUXILIARY_ENTRIES: &[&str] = &[];

pub enum EntryKind { Dir, File, Other }

pub fn read_entry_kind(path: &Path) -> Result<EntryKind>   // symlink_metadata; a failure is an error
pub fn classify_entry(path: &Path, kind: EntryKind) -> Result<Option<PathBuf>>
```

| Entry | Disposition |
|---|---|
| directory | a journal directory, loaded |
| regular file whose name is in `PERMITTED_AUXILIARY_ENTRIES` | permitted, skipped |
| any other regular file | `journal-unexpected-entry` — error |
| symlink or other non-regular entry | `journal-unsupported-link` — error |
| metadata read fails for any reason (including `NotFound`) | `StoreError::Io` naming the entry |
| the directory iterator yields `Err` | `StoreError::Io` naming the directory |

`read_entry_kind` uses `symlink_metadata`, so classification does **not** follow links: a symlink
arrives as `EntryKind::Other` and is refused rather than resolved through.

`PERMITTED_AUXILIARY_ENTRIES` is **empty, and explicitly so**. Nothing in this crate writes an
auxiliary entry at that level: the emitter's only temporary file, `journal.json.new`, is created
*inside* a journal directory and renamed over its target, so it never appears in `in-progress/`. The
list exists so that permitting such an entry in future is a recorded decision rather than a filter
predicate someone widens in passing. The test asserts the list is empty, so adding a name to it
requires updating the test that says why it was empty.

`journal_bytes` — the helper that proves a read-only operation left the source bytes alone — carried
the same `.flatten()` / `is_file()` pattern and is corrected the same way. A file that silently
disappeared from that listing would have weakened exactly the proof it exists to provide.

The behaviour for a missing `in-progress` directory is unchanged: `Ok(Vec::new())`, per **R-ABSENT**.
Absence still asserts nothing.

---

## 3. Regression results

All cases are in `crates/tdb-qa/tests/evidence_journal.rs`, section
`R3 — dedup integrity and fail-closed discovery`. Every one runs against a **synthetic** evidence
root under the system temp tree. Nothing reads or writes `docs/audits/`, `anchors/`, a game tree or a
private fixture.

The two dedup cases follow the R2 shape: start from `older_qualified_pass()`, which builds a
completed run and **asserts it is genuinely `AutomatedQualified`** before any damage is introduced,
so a case cannot pass by starting from something that never qualified.

| Case | What it plants | Assertions |
|---|---|---|
| `r3_a_corrupt_record_in_a_matching_leftover_is_not_deduplicated_away` | a `mirroring` leftover — identity and every record copied from the run it names — plus one extra record whose digest is then broken | `corrupt-record:observations.csv:*` reported; journal retained; `sealed-run-unverifiable:` present; the row still cites the journal (`auto-journal`); `Withheld` / `damaged-evidence`; `damaged-evidence:<journal_id>` in `unresolved` |
| `r3_a_truncated_tail_in_a_matching_leftover_is_not_deduplicated_away` | the same leftover with an interrupted append (45 bytes chopped) | `truncated-trailing-record:observations.csv`; **asserts `equivalence_findings` is empty**, i.e. the surviving records really do match the run, so the case tests the intended condition and not an accidental mismatch; then the same retention and withholding assertions |
| `r3_a_clean_matching_leftover_is_still_deduplicated` | an undamaged `mirroring` leftover | `set.journals.is_empty()`; the key remains `AutomatedQualified` |
| `r3_a_stray_file_in_the_journal_directory_fails_discovery_closed` | `observations.csv.orig` beside the journals — records rescued by hand, the realistic case | load fails with `journal-unexpected-entry` naming the file; `PERMITTED_AUXILIARY_ENTRIES` asserted empty |
| `r3_an_unsupported_link_is_refused_rather_than_skipped` | fault injection: `classify_entry(path, EntryKind::Other)` | `journal-unsupported-link`; the `Dir` and `File` dispositions asserted for contrast |
| `r3_a_failed_metadata_read_is_an_error_not_an_absence` | `read_entry_kind` on an entry that cannot be stat'ed | error, naming the entry; a real directory still classifies as `Dir` |

The last two use direct fault injection because neither failure can be produced reliably on this
platform: creating a symlink on Windows needs developer mode or elevation, and a permission-denied
stat cannot be arranged inside a temp tree the test owns. Splitting `classify_entry` out of the
filesystem is what makes those branches reachable at all.

### 3.1 Proving the guards are load-bearing

Each guard was reverted in source, the suite re-run, and the guard restored immediately. Both
mutations were made to `journal.rs` only, and the file was restored from a byte-for-byte copy.

| Mutation | Result |
|---|---|
| dedup guard reverted (`findings.is_empty() && !damaged` → `findings.is_empty()`) | **2 failed** — `r3_a_corrupt_record_…` and `r3_a_truncated_tail_…`; 29 passed |
| discovery reverted to `.flatten()` + `filter(is_dir)` | **1 failed** — `r3_a_stray_file_…`; 30 passed |

The second mutation fails only one case, and that is honest rather than a gap being glossed over:
`r3_an_unsupported_link_…` and `r3_a_failed_metadata_read_…` exercise the extracted classifier
directly, and the classifier still exists under that mutation — what the mutation removes is the
loop's *use* of it, which is what the stray-file case covers.

### 3.2 Suite, lint and format

| Check | Result |
|---|---|
| `cargo test -p tdb-qa` | **193 passed, 0 failed, 0 ignored** (67 lib + 32 acceptance + 7 emit + **31** journal + 12 migration + 36 publication + 8 inventory) — 187 before this package, +6 |
| `cargo clippy -p tdb-qa --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | exit 0 |

No emitter was re-run and no full workspace suite was run, per the package.

---

## 4. Preservation

| Item | Before | After |
|---|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` | unchanged |
| git index | empty | empty |
| files modified by this package | — | `crates/tdb-qa/src/evidence/journal.rs`, `crates/tdb-qa/tests/evidence_journal.rs` (both already untracked from C0-S2-INT1/R1/R2) |
| `git status --porcelain` | as at end of R2 | identical; **no new entry** |
| real evidence `H:\c0-s2-int1\run1` | 4 sealed runs, 0 journals | unchanged |
| real evidence `H:\c0-s2-int1\run3` | 4 sealed runs, 0 journals | unchanged |
| `docs/audits/`, `anchors/` | — | untouched (`check` writes nothing; a `--data` comparison reads only) |

Both real evidence roots still load and rebuild **byte-identically** under R3:

```
check --root run1 --data out-r1      → ok: 2016 row(s), no validation failure, no drift   (exit 0)
check --root run3 --data out-r2-run3 → ok: 2175 row(s), no validation failure, no drift   (exit 0)
```

A fresh `build --root run1` was also diffed file-by-file against R2's published set: **no differences**.
Neither root holds a journal, so the dedup change cannot alter them — the identity is the expected
result, and it confirms the discovery rewrite did not perturb loading.

---

## 5. Exact failure behaviour

| Condition | Where | Behaviour |
|---|---|---|
| damaged journal, `sealed_run_id` names a run all survivors match | `load_journals` | journal **retained**, `sealed-run-unverifiable:<run_id>:…` appended, declared scope withheld as `damaged-evidence` (row 4b) |
| damaged journal, records also disagree with the run | `load_journals` | retained, `sealed-run-mismatch:` — unchanged from R2 |
| clean journal, records match the run | `load_journals` | deduplicated — unchanged from R2 |
| clean journal, `sealed_run_id` names an absent run | `load_journals` | retained, `sealed-run-missing:` (non-damaging) — unchanged from R2 |
| stray file in `in-progress/` | `classify_entry` | `StoreError::Rule { rule: "journal-unexpected-entry" }` → `qa-evidence` exit **3** |
| symlink or non-regular entry in `in-progress/` | `classify_entry` | `StoreError::Rule { rule: "journal-unsupported-link" }` → exit **3** |
| metadata read fails on a listed entry | `read_entry_kind` | `StoreError::Io` naming the entry → exit **3** |
| directory iterator yields `Err` | `load_journals` | `StoreError::Io` naming the directory → exit **3** |
| `in-progress/` absent | `load_journals` | `Ok(vec![])` — R-ABSENT, unchanged |

Exit 3 is the contract's code for a malformed **active** input (R-ACTIVE-STRICT); the CLI already
mapped journal load errors to it, and these new errors travel that same path. `check` and `build`
both refuse rather than publishing a set built from a listing that may be incomplete.

---

## 6. Corrections to earlier receipts

**C0-S2-INT1-R2** (`docs/receipts/c0-s2-int1-r2-integrity-gates.md`) claimed that journal-to-run
equivalence was verified before deduplication and that damaged evidence always withholds. Both were
true only in isolation. Their **combination** was not: a journal that was damaged *and* whose
surviving records matched the run it named was deduplicated away, taking its damage report and its
withholding with it. R2's own regression cases did not catch this because none of them combined a
valid `sealed_run_id` with damage — the damage cases used journals with no `sealed_run_id`, and the
dedup cases used clean journals. A labelled revision recording this has been added to that receipt.

No qualification figure published by R1 or R2 changes: neither `run1` nor `run3` contains a journal,
so no row in either published set was affected by the defect. The defect was reachable only by a
sealing whose cleanup failed **and** whose journal was subsequently damaged.

---

## 7. Remaining limitations

1. **Withholding granularity is still the whole declared catalog.** R2's limitation stands: one
   damaged record withholds every key its emitter declared, because the damaged cell is
   unattributable. `sealed-run-unverifiable` inherits that bound. Narrowing it needs per-key
   checkpointing, which is not in scope here.
2. **`load_runs` still uses `.flatten()` and `is_dir()`.** The same three silent losses exist in the
   sealed-run loader in `store.rs`. The package scoped this correction to journal discovery, so it
   was not touched. It should be corrected on the same reasoning; a run that silently disappears
   from discovery removes evidence just as effectively.
3. **`PERMITTED_AUXILIARY_ENTRIES` is empty by assertion, not by construction.** Nothing prevents a
   future writer from creating an entry there; the loader would then fail closed, which is the safe
   direction, but the failure would surface as a refusal rather than as a design decision.
4. **The symlink and metadata-failure branches are covered by injection, not by a real filesystem.**
   On a platform where those conditions can be produced (elevated Windows, or Linux CI once C0.2 R6
   lands), a fixture test over a genuine link and a genuinely unreadable entry would be stronger.
5. **No new evidence was produced.** The four emitters were not re-run, per the package, so the
   published figures in R1 and R2 remain the most recent real measurements.

---

## 8. Files changed

| File | Change |
|---|---|
| `crates/tdb-qa/src/evidence/journal.rs` | `load_journals` dedup guard; `PERMITTED_AUXILIARY_ENTRIES`, `EntryKind`, `read_entry_kind`, `classify_entry`; explicit discovery loop; `journal_bytes` entry handling; module docs |
| `crates/tdb-qa/tests/evidence_journal.rs` | +6 cases and `assert_damaged_leftover_is_retained` |
| `docs/receipts/c0-s2-int1-r3-dedup-integrity.md` | this receipt |
| `docs/receipts/c0-s2-int1-r2-integrity-gates.md` | labelled revision (§6) |

## 9. Unresolved questions for Codex

* Limitation 2 — should `load_runs` be corrected the same way, and does that belong in a follow-up
  package or in the next scheduled change to `store.rs`?
* Should `sealed-run-unverifiable` be a distinct conflict class in the contract's §12 vocabulary, or
  is it adequately covered as a `damaged-evidence` cause?

**Stopping for Codex review.** Nothing staged, committed or pushed.

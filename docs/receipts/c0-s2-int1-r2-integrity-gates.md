# C0-S2-INT1-R2 — Failing closed on damaged execution evidence

> **Package:** C0-S2-INT1-R2, a bounded repair of the integrity defects Codex review found in
> [C0-S2-INT1-R1](c0-s2-int1-r1-execution-truth.md). **Date:** 2026-09-08.
> **Author role:** developer. Reviewer: Codex (architect).
> **Status: Passed.** Nothing staged, committed or pushed. **No CI change, no live publication, no
> migration installation.** Every case ran over synthetic evidence in an isolated directory outside
> the repository. The four expensive emitters were **not** re-run; the previous real runs are
> preserved unchanged.
>
> R1's append-only observations, incomplete-run visibility, structured variants, unmapped
> observations and corrected freshness scopes are **all preserved**. This package only narrows what
> a damaged, unreadable or falsely-linked journal is allowed to do.
>
> **S2 remains partial.**

---

## 1. Revision to R1's integrity and qualification claims

R1 §4 presented its integrity table as the complete story. Three of its statements were wrong or
too weak, and are **corrected here**:

| R1 said | Correction |
|---|---|
| "a record file that is absent → empty" | R1 treated **every** read failure as absence. A directory planted at `observations.csv`, a permission error, or invalid UTF-8 all reported "the emitter recorded nothing". Only `NotFound` may mean absent; everything else is now a **hard error** (§2). |
| "a corrupt record is dropped and reported … neighbours are unaffected" — presented as sufficient | Dropping and reporting was **not** sufficient. R1 emitted an `unresolved` token and then let an older completed pass stay `automated-qualified` on the very key whose newer evidence had been destroyed. Damage now **withholds** across the journal's declared scope (§3). |
| "the separator is a byte that cannot occur in a CSV field value" | **False.** A quoted CSV field may contain any byte, including `0x1f`, and `["a\x1fb"]` hashed identically to `["a", "b"]`. The framing is replaced and versioned (§5). |

R1 also deduplicated a journal against a sealed run on a **matching id string alone**, which let a
wrong or planted `sealed_run_id` delete a journal's evidence outright (§4).

R1's `truncated-trailing-record` handling additionally rested on "one record is one line", which was
only true by accident: a `detail` containing a newline produced a multi-line CSV record, and the
"text after the last newline" rule would have mis-identified the interrupted record (§5).

**No qualification figure changes for an undamaged tree.** Rebuilding R1's real evidence
(`H:\c0-s2-int1\run3`) under R2 produces a **byte-identical** published set, and `check` still
reports `ok: 2175 row(s), no validation failure, no drift`. The 140 `automated-qualified` rows R1
reported stand.

---

## 2. Item 1 — read errors are never empty evidence

`journal::read_optional` replaces every `let Ok(..) = read_to_string(..) else { empty }`:

| Condition | Result |
|---|---|
| `NotFound` | `Ok(None)` — the emitter had not created the file yet |
| entry exists but is a directory or other non-regular entry | `journal-unexpected-entry-type`, naming what was found |
| invalid UTF-8 | `journal-unreadable-file`, "not valid UTF-8; the records in it cannot be attributed" |
| any other I/O failure (permission, sharing violation) | `StoreError::Io`, naming the path |

It is applied to `journal.json`, `keys-declared.txt`, `observations.csv`, `unmapped.csv` and
`facets.csv`, and to the `in-progress` directory itself. Presence is established with
`symlink_metadata`, so a dangling link is not read as absent. Every one of these reaches
`qa-evidence check` / `build` as **exit 3**, the contract's malformed-active-input code.

---

## 3. Item 2 — integrity uncertainty withholds qualification

A new qualification row, placed above the freshness and pass rows and below the existing
blocked/unreviewed-failure rows:

> **Row 4b** — evidence covering this key could not be read back with certainty ⇒
> `qualification = withheld`, `reason = damaged-evidence`.

**The affected scope is the journal's declared key set, and nothing is guessed.** A damaged record's
key is precisely what cannot be trusted, so it is never attributed to a specific cell; the honest
bound is every key the journal declared. Two consequences follow, and both are enforced at load:

* **Damaged declarations are fatal.** `keys-declared.txt` truncated, unparseable or unreadable ⇒
  `journal-declarations` error. Without declarations there is no bound, so the tree fails closed.
* **Damage with no declarations at all is fatal** — `journal-unbounded-damage`.

**What is *not* damage.** A clean active journal, and a `sealed-run-missing` link. `Journal::
damaging_findings()` excludes the latter deliberately: the run a journal claims to have been sealed
into being absent means the link cannot be checked, but every record it holds is still
digest-verified. Nothing about the evidence is in doubt, so it is reported without withholding.
That keeps R1's rule intact — *"incomplete clean passes need not invalidate older valid evidence
merely because a run is active; damaged evidence is a different condition."*

Verified neighbours are preserved for diagnostics (the records either side of a corrupt one survive
and are loaded), and **no original byte is ever rewritten**: the module only reads, and
`check_and_build_leave_the_journal_bytes_untouched` hashes the directory before and after a full
load → merge → build.

---

## 4. Item 3 — equivalence is verified before deduplication

`journal::equivalence_findings(journal, run)` must be **empty** before a journal is skipped. It
checks:

* **provenance** — `emitter`, `started_utc`, `scope_id`, `scope_version`, `implementation_digest`,
  `toolchain`, `host_os` all equal;
* **containment** — every verified declaration, observation, unmapped record and facet the journal
  holds is present in the run.

Any difference records `sealed-run-mismatch:<run_id>:<what differs>`, which is a damaging finding:
the journal stays loaded, is cited with the mismatch in its citation detail, and withholds across
its declared scope. A `sealed_run_id` naming an **absent** run keeps its R1 behaviour
(`sealed-run-missing`, non-damaging).

Both directions are tested: a journal naming an existing **unrelated** run is not deduplicated, and
a genuine post-seal leftover — a journal mirroring the run's identity and records — still is.

---

## 5. Item 5 — record framing, `qa-run-journal/3`

| Property | `/2` | `/3` |
|---|---|---|
| digest | fields joined with `0x1f` | **length-prefixed**: field count, then each field's byte length, then its bytes |
| collision | `["a\x1fb"]` == `["a","b"]` | impossible by construction; asserted directly |
| record ↔ line | assumed | **guaranteed** — `escape_field` replaces `\`, newline, CR and every other control character before writing, so no raw newline reaches the file |
| digest input | the written text | the **unescaped** values — it certifies what was meant, not how it was spelled |
| row schema | reused the sealed-run `qa-observation/1` | its own `qa-journal-{observation,unmapped,facet}/1` |

Because the digest semantics changed, the format is versioned: a `/2` journal is **refused**
(`unsupported schema … expected "qa-run-journal/3"`), never reinterpreted. Escaping is strict in
both directions — a malformed `\x`, an unknown escape or a trailing backslash makes the record
uncertain, so it is dropped and reported rather than decoded on a best-effort basis.

An interruption inside what would have been a quoted multi-line field is tested end to end: the
record before it survives with its multi-line `detail` round-tripping exactly, and only the
interrupted record is lost.

---

## 6. Item 4 — regression results

`cargo test -p tdb-qa --test evidence_journal` — **25 passed, 0 failed**. Every R2 damage case
starts from an older, fresh, completed pass that is asserted to be genuinely `automated-qualified`
before the damage is introduced, so none of them can pass vacuously.

| Case | Starting point | Result |
|---|---|---|
| `r2_a_corrupt_failure_record_withholds_the_older_completed_pass` | qualified pass | `withheld / damaged-evidence`; no observation invented from the damaged bytes |
| `r2_a_truncated_record_of_unknown_outcome_withholds_the_older_completed_pass` | qualified pass | both declared keys withheld; the verified neighbour is kept |
| `r2_an_unreadable_or_wrong_type_record_file_is_never_read_as_empty` | qualified pass | hard error naming `observations.csv` and `journal-unexpected-entry-type` |
| `r2_invalid_utf8_in_a_record_file_is_an_error_not_an_absence` | qualified pass | hard error, `journal-unreadable-file` |
| `r2_damaged_declarations_are_fatal_because_nothing_could_bound_the_damage` | qualified pass ×3 | truncated list, unparseable key, and damage-with-no-declarations all hard errors |
| `r2_a_false_sealed_run_id_naming_an_existing_unrelated_run_is_not_deduplicated` | qualified pass | journal kept, `sealed-run-mismatch` cited, row withheld |
| `r2_a_sealed_run_that_lost_a_record_the_journal_proves_was_made_is_a_mismatch` | qualified pass | `… absent from the run`, withheld |
| `r2_a_legitimate_sealing_overlap_is_still_deduplicated_and_still_qualifies` | qualified pass | journal skipped, cited **once**, still qualified |
| `r2_a_clean_active_journal_does_not_invalidate_an_older_completed_pass` | qualified pass | **stays qualified**, incomplete work still reported |
| `r2_an_ordinary_completed_run_is_unaffected_by_any_of_this` | qualified pass | pass / fresh / qualified, no `unresolved` tokens |
| `r2_an_interruption_inside_a_quoted_multiline_field_costs_only_that_record` | — | one record per line; earlier record and its multi-line value intact |
| `r2_a_journal_written_under_the_old_framing_is_refused_not_reinterpreted` | — | `/2` refused |
| `a_journal_naming_an_absent_run_is_kept_flagged_and_still_trusted` | qualified pass | flagged, **not** treated as corruption |

### Are they load-bearing?

Four deliberate mutations of the source, each reverted immediately:

| Mutation | Cases that failed |
|---|---|
| `read_optional` reverted to R1's "any read error is an empty file" | the wrong-type and invalid-UTF-8 cases |
| qualification row 4b disabled | corrupt-failure, truncated-unknown-outcome, incomplete-seal-mismatch |
| deduplicate on the `sealed_run_id` string alone (R1) | false-sealed-id, incomplete-seal-mismatch |
| digest reverted to the `0x1f` separator and escaping removed | the two framing unit tests **and** the multi-line interruption case |

---

## 7. Verification

| Check | Command | Result |
|---|---|---|
| Journal / merge / mapping cases | `cargo test -p tdb-qa --test evidence_journal` | **Passed** — 25/25 |
| Emitter lifecycle | `cargo test -p tdb-qa --test evidence_emit` | **Passed** — 7/7 |
| Whole `tdb-qa` crate | `cargo test -p tdb-qa` | **Passed** — **187**, 0 failed, 0 ignored |
| Emitter, default mode | `env -u QA_EVIDENCE_ROOT -u QA_WRITE cargo test -p studio-domain --test surface_matrix` | **Passed**, CSV unchanged |
| Emitter, observation on (smoke) | `QA_EVIDENCE_ROOT=… --test surface_matrix` | **Passed** — sealed a `/3`-journalled run, scope v2, real digest, no journal left behind |
| R1's real runs still load | `check --root run3` | `ok: 2175 row(s), no validation failure, no drift`, exit 0 |
| R1's published output | `build --root run3` vs R1's output | **byte-identical** — no regression for undamaged trees |
| C0-S2-INT1's runs still load | `build --root run1` | 10 files, no error |
| Strict clippy, scoped | `cargo clippy -p tdb-qa -p studio-domain --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, repo-wide | `cargo fmt --all -- --check` | **Passed**, exit 0 |
| Four-emitter rerun | — | **Not run.** Not required by this package. |

### Preservation

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25`, unchanged |
| Index | **empty** |
| `anchors/**` (552) · `docs/audits/**` (69) | **0 mismatches** against the pre-package hash lists |
| `git status --porcelain` vs package start | **4 added entries**, all intended and unchanged since R1 (`M crates/studio-domain/Cargo.toml`; `?? crates/tdb-qa/build.rs`; `?? crates/tdb-qa/tests/evidence_emit.rs`; `?? crates/tdb-qa/tests/evidence_journal.rs`). 0 removed |
| Previous real runs (`run1`, `run3`) | **4 sealed run directories each, unchanged**; both still load and build |
| Live publication / migration / CI / staging / commit / push | **none** |

### Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/journal.rs` | `read_optional`; escaping + length-prefixed digest; `/3` schema and journal row schemas; fatal declaration damage; unbounded-damage refusal; `equivalence_findings`; verified deduplication. 4 unit tests. |
| `crates/tdb-qa/src/evidence/model.rs` | `Journal::damaging_findings()`. |
| `crates/tdb-qa/src/evidence/merge.rs` | `damaged_evidence` collection and qualification **row 4b**. |
| `crates/tdb-qa/src/evidence/emit.rs` | Writes the journal row schemas. |
| `crates/tdb-qa/tests/evidence_journal.rs` | 14 → **25** cases; `JournalWriter::mirroring`, `older_qualified_pass`. |

---

## 8. Remaining limitations

| # | Limitation |
|---|---|
| **L1** | **The damaged scope is the whole declared set, not the damaged cell.** That is deliberate — the damaged record's key is unattributable — but for a real emitter the declared set is its whole catalog, so one corrupt record withholds that emitter's entire coverage. Narrowing it safely would need per-key checkpointing in the journal, which is a design change, not a repair. |
| **L2** | **A damaged journal degrades, it does not fail the whole tree.** Only *unbounded* damage and unreadable material are hard errors. The alternative — rejecting `check`/`build` on any damage — is the other option the package allowed; the withholding path was chosen so a single corrupt record does not make the whole evidence tree unreadable. Both remain available; this is an architect call if the stricter reading is wanted. |
| **L3** | **Escaping restricts what a field may contain on disk, not in memory.** A `detail` with control characters round-trips exactly, but the on-disk form is no longer the literal value. Anything reading the CSV by hand must unescape. |
| **L4** | **No power-loss durability and no concurrency control**, unchanged from R1. A concurrent writer can still append while a reader is mid-file; the reader would see a truncated tail and report it, which is the designed behaviour, but it is not a lock. |
| **L5** | **Digest strength is SHA-256 over content, not a signature.** It detects damage and accidental rewriting; it does not detect an attacker who recomputes the digest after editing a record. |
| **L6** | R1's open items carry over: `qa-evidence-unmapped.csv` as a fifth published file, the missing axis value for "declared by a still-running emitter", the absent required-variant catalog, and C0-S2-INT1 §8 Q1/Q2/Q4/Q5. |

## 9. Status

| Item | Status |
|---|---|
| **S2** | **Still partial.** No CI gate proposed or activated. |
| C0 | **Not complete.** |
| R1's receipt | corrected in §1 by this labelled revision; its 140-qualified figure and its real runs stand unchanged. |

Stopping for Codex review.

---

## Revision — C0-S2-INT1-R3 (2026-09-07): the two guards were not combined

**Corrected claim.** This receipt stated that journal-to-run equivalence is verified before
deduplication (§3) and that damaged evidence always withholds its declared scope (§2, row 4b). Each
statement was true on its own; the two were **not** combined in `load_journals`.

`equivalence_findings` compares only the records that were successfully **loaded**. A damaged record
is dropped before that comparison, so when every survivor matched the run named by `sealed_run_id`,
the dedup branch fired and the journal was discarded — along with its integrity findings **and its
withholding effect**. A damaged sealing leftover therefore silently qualified the keys row 4b exists
to withhold. A surviving-record subset cannot prove equivalence for the records that were dropped;
those records are precisely the ones equivalence had no access to.

This receipt's regression cases did not catch it because none combined the two conditions: the
damage cases used journals with no `sealed_run_id`, and the dedup cases used clean journals.

**Fixed in R3.** A journal with any damaging finding is never deduplicated; it is retained, flagged
`sealed-run-unverifiable`, and continues to withhold. Clean equivalent leftovers still deduplicate.
Journal discovery also no longer swallows enumeration errors, failed metadata reads or links.

**Unaffected.** No figure in this receipt changes. Neither `run1` nor `run3` contains a journal, so
no published row was ever affected by the defect, and both roots still rebuild byte-identically
under R3.

See [c0-s2-int1-r3-dedup-integrity.md](c0-s2-int1-r3-dedup-integrity.md).

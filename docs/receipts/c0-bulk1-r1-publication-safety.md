# C0-BULK1-R1 — Publication safety and accurate checkpoint status

> **Package:** C0-BULK1-R1, a bounded repair of the blocking defects Codex review found in
> [C0-BULK1 checkpoint 3](c0-bulk1-03-publication.md). **Date:** 2026-09-07.
> **Author role:** developer. Reviewer: Codex (architect).
> **Status: Passed.** Nothing staged, committed or pushed. **No live migration installation.** No
> product decision taken. Every publication, recovery and migration execution in this package ran in
> an isolated fixture directory.
>
> **Superseded in part by C0-BULK1-R2** — see the review correction below.

## ⚠ Review correction (C0-BULK1-R2, 2026-09-07)

**D11 below overstated what it closed.** It said the boundary checks now "live inside" the mutating
functions "so no caller can skip them". Two things made that untrue, and Codex review found both:

1. **A library caller could still opt out of protection.** The public `write_to`, `publish` and
   `migrate` wrappers passed an **empty** protected set, so calling the library directly gave weaker
   checks than the CLI gave. The wrappers are removed in R2; a validated `paths::Layout` is now a
   required argument at every one of those entry points.
2. **Only the output *directory* was validated, not the file written into it.** `fs::write` follows
   an existing child link, so a link planted at an output file name, at `build-inputs.json`, or at
   the journal’s `.new` temporary redirected the write out of the checked directory entirely. R2
   validates every mutation target in its own right.

The rest of §1 stands. The current boundary behaviour is
[c0-bulk1-r2-boundary-enforcement.md](c0-bulk1-r2-boundary-enforcement.md); the names
`write_to_guarded` / `publish_guarded` / `migrate_guarded` used below no longer exist.

## 1. What was wrong, and what it is now

| # | Defect | Repair |
|---|---|---|
| **D1** | `prepared`-phase recovery assumed nothing had moved — but retention moves files, and a crash between a move and its journal update left files in `.qa-prev-<id>/` that recovery never restored | Recovery derives every per-file action from **digests of the bytes on disk**. Retention is bracketed by an explicit `retaining` phase written *before* the first move, and the `prepared` and `retaining` phases take the identical rollback path |
| **D2** | Rollback restored previous files but **never removed outputs that had not existed before**, leaving a half-published new file behind | A target whose `previous_digest` is `None` is **deleted** on rollback. A post-condition check then verifies exact bytes **and** membership |
| **D3** | Recovery trusted the per-file `installed` boolean, which is exactly the field a rename-then-crash leaves wrong | The booleans are marked advisory and **read by nothing**. `Observed` hashes the published, retained and staged copy of every target and compares against the recorded digests |
| **D4** | The single journal was rewritten by truncation, so a failure during the write could destroy the only record of an in-flight transaction | `Journal::write` writes a sibling `.new` temporary and **renames it over** the journal. The rename is the single commit point; a failure before it leaves the previous journal intact |
| **D5** | `Journal::read` returned `Ok(None)` for **every** error, so an unreadable or corrupt journal read as "no transaction in progress" | **Only `NotFound` is `None`.** Unreadable, corrupt, wrong-schema and self-inconsistent journals are hard errors with diagnostics naming the file and the retained material |
| **D6** | A new publication could start on top of an unresolved one | A journal in any phase but `completed` **refuses** the publication, and the diagnostic names the phase, the staging directory, the retained generation and `--recover` |
| **D7** | Recovery validated nothing about the journal before acting | Schema, non-empty id and targets, **plain target names restricted to the published output set**, `data_dir` identity and containment of `staging`/`prev` are all checked **before any write or delete** |
| **D8** | The publication id was a digest of the output, so republishing identical content produced the same id and the second attempt's `.qa-prev-<id>` **collided with and overwrote** the first retained generation | The id carries an attempt nonce (pid + counter + clock reading). Two attempts on identical content have different ids, and an existing `.qa-publish-`/`.qa-prev-` directory for an id is refused outright |
| **D9** | Generations were auto-pruned to the last 3, **ordered by digest** — which is not chronology, so pruning discarded arbitrary generations | `prune_generations` is **removed**. Nothing is pruned in this revision |
| **D10** | Recovery deleted staging and the retained generation before verifying the restored state | Material is removed **only after** the post-condition check passes. On failure everything is retained and the diagnostic names both directories |
| **D11** | Boundary checks lived only in the CLI, so a library caller — or a `..`, symlink or junction alias — could write into the evidence tree, manual records, frozen sources or the published set | New [`evidence::paths`](../../crates/tdb-qa/src/evidence/paths.rs). Checks run **inside** `write_to_guarded`, `publish_guarded`, `migrate_guarded` and `install`, before any directory is created |
| **D12** *(found while repairing)* | Recovery classified a file as *previous* **or** *staged*, first match winning — wrong when both generations share a byte-identical file, which turned a completable install into a rollback | `Observed`'s predicates are independent; a file that is both is both. Caught by the mixed-generation regression case |

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/paths.rs` | **New.** Boundary enforcement: `resolve_for_write`, `assert_no_reparse_points`, `assert_disjoint`, `protected_under`, `assert_output_is_safe`. 8 unit tests. |
| `crates/tdb-qa/src/evidence/publish.rs` | Journal, publish and recover **rewritten** (D1–D10, D12). `OutputSet::write_to_guarded` added (D11). |
| `crates/tdb-qa/src/evidence/migrate.rs` | `migrate_guarded` added; `install` gained a candidate/root disjointness check (D11). |
| `crates/tdb-qa/src/evidence/mod.rs` | Wired `paths`. |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | Uses the guarded entry points; `--recover` reports retained material. |
| `crates/tdb-qa/tests/evidence_publication.rs` | **Rewritten**: 26 cases (was 9). |
| `docs/receipts/c0-bulk1-03-publication.md` | ⚠ review correction: **S2 is partial**; publication superseded. |
| `docs/receipts/c0-bulk1-06-canonical-contracts.md` | ⚠ review correction: contract **not complete** — expressive coverage missing. |
| `docs/receipts/c0-bulk1-07-closure.md` | ⚠ review correction §2.5: the "no repository write path" claim withdrawn; items 4 and 7 restated. |
| `docs/plans/project-completion-roadmap.md` | C0 status table: items 4 and 7 restated as **partial**. |
| `docs/receipts/c0-bulk1-r1-publication-safety.md` | **New.** This receipt. |

## 3. Guarantees, stated precisely

**Against process interruption** — a crash, a kill, or an error at any point — publication and
recovery restore either the complete previous state or the complete new state, on the assumption that
the operating system has already made completed `write` and `rename` calls visible. Every
intermediate filesystem state the sequence can produce is covered by a regression case (§4).

**Not against power loss or an OS crash.** **No `fsync` or `FlushFileBuffers` is issued anywhere**,
and no parent directory is flushed. After such an event the journal and the data files may disagree
in ways recovery reports as foreign bytes rather than repairs. That claim is deliberately **not**
made: making it would require flushing every file and its parent directory and testing it, which
this revision does not do.

**Not against a concurrent writer.** The boundary checks are check-then-act guards, not locks; a
privileged concurrent writer can still replace a path component between the check and the write. No
evidence operation takes a filesystem lock. Stated in the module documentation.

**Recovery is fail-closed.** Bytes that match neither the recorded previous nor the staged content
stop the recovery **before it changes anything**, retaining the staged material and the retained
generation, with a diagnostic naming the file, the digest found and both directories.

## 4. Regression tests — 26 cases

Every case asserts against the **filesystem state a crash would really leave**, not against the
injected fault. Run: `cargo test -p tdb-qa --test evidence_publication` — **26 passed, 0 failed**.

| Required coverage | Case | Demonstrated state |
|---|---|---|
| Interruption after **each** old-file move | `r1_interruption_after_any_retention_move_rolls_back_completely` | loops **N = 0..8**, crashing after the Nth retention move and **before** its journal update; asserts the published set is genuinely `8 − N − 1` files mid-flight, then byte- and membership-identical after recovery |
| … and after its journal update | `r1_interruption_after_a_retention_journal_update_also_rolls_back` | phase `retaining`, journal current |
| Mutation **before** journal persistence | `r1_an_install_rename_that_beat_its_journal_update_is_still_recoverable` | asserts `installed == false` for the file whose bytes are already the new ones — the state D3 could not survive — then completes |
| Prepare-phase interruption | `r1_a_prepare_interruption_leaves_the_published_set_untouched` | nothing moved; rollback is a no-op that still verifies |
| Rollback with a **mix** of pre-existing and new outputs | `r1_rollback_removes_outputs_that_did_not_exist_before_and_restores_those_that_did` | 3 of 8 targets deleted before publishing, so they have no previous state; after rollback the 5 survivors match byte-for-byte and the 3 are **absent again** |
| Partial journal write/replacement failure | `r1_a_failed_journal_replacement_leaves_the_previous_journal_intact` | fails after the `.new` temporary is written, before the rename; the prior journal still parses and the temporary is left for inspection |
| Unreadable / corrupt journal | `r1_an_unreadable_or_corrupt_journal_fails_closed_with_a_diagnostic`, `r1_an_unsupported_journal_schema_is_refused_rather_than_guessed`, `r1_only_not_found_means_no_journal` | corrupt JSON, unknown schema, and a **directory** where the journal file should be |
| Second publish while recovery is pending | `r1_a_second_publish_is_refused_while_a_transaction_is_unresolved` | refuses, names `--recover`, then succeeds after recovery |
| Repeated publication of identical output | `r1_republishing_identical_content_does_not_collide_with_the_retained_generation` | distinct ids, **both** retained generations present |
| No pruning | `r1_many_publications_are_all_retained_because_pruning_was_removed` | 5 publications → 5 retained generations |
| Modified staged / retained bytes | `r1_modified_staged_bytes_are_detected_and_recovery_stops_without_changing_anything`, `r1_modified_retained_bytes_are_detected` | tampered file → `foreign-bytes`, nothing changed, material retained |
| Path traversal, overlapping roots | `r1_a_build_output_inside_the_evidence_tree_is_refused_at_the_mutation`, `r1_publishing_into_the_evidence_tree_is_refused`, `r1_staged_and_published_directories_may_not_overlap` | four protected roots + `..`; asserts protected **bytes and membership** unchanged |
| Windows junction aliases | `r1_a_junction_aliasing_a_protected_root_is_refused_when_one_can_be_created` | creates a real junction (or symlink) onto `manual/`; **skips loudly** when the platform refuses, never silently passing |
| Recovery failure retains material | asserted inside the foreign-bytes cases | `staging` and `prev` still directories; diagnostic says "are kept" |

Journal-validation cases assert the published digests are **unchanged** after the refusal, proving
the checks run before any write or delete.

## 5. Verification

| Check | Command | Result |
|---|---|---|
| Publication regressions | `cargo test -p tdb-qa --test evidence_publication` | **Passed** — 26/26 |
| Whole crate | `cargo test -p tdb-qa` | **Passed** — 55 + 32 + 12 + 26 + 8 = **133**, 0 failed, 0 ignored |
| Strict clippy, scoped | `cargo clippy -p tdb-qa --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, repo-wide | `cargo fmt --all -- --check` | **Passed**, exit 0 |
| CLI end to end, isolated | `build` → `publish` → `publish` again → `check` → refused build into the evidence root | **Passed** — 2 distinct ids, **2 retained generations**, `ok: 2016 row(s)`, and `path-overlap … resolves inside the evidence root` |
| Broad workspace suite | — | **Not run.** Not required by this package, and nothing outside `tdb-qa` changed |

Isolation: `H:\c0-bulk1\sandbox-r1\` and per-case directories under `%TEMP%\tdb-qa-evidence-pub\`,
`%TEMP%\tdb-qa-paths\`. **No live evidence tree, `docs/audits/`, anchor or game file was written**,
and no destructive case was pointed at anything but a synthetic root built by the test.

## 6. Preservation checks

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` — unchanged |
| Index | **empty** before and after |
| Reset / restore / clean / stash / checkout / branch change / blanket staging | **none** |
| Baseline `git status` entries (236) | **identical** — 0 added, 0 removed; the delta is empty |
| Tracked files modified vs HEAD | **170**, the same count as the R1 baseline. Of the baseline hash list, exactly **3** files differ: `crates/tdb-qa/src/bin/qa_evidence.rs`, `crates/tdb-qa/tests/evidence_publication.rs` and `docs/plans/project-completion-roadmap.md` — all intended |
| Scope of that hash check, stated exactly | the baseline list covered the **225 individually-listed files**. Eleven working-tree entries are untracked **directories** (`crates/tdb-qa/src/evidence/`, `crates/tdb-canonical/`, `defs/adapters/`, `defs/canonical/`, `defs/qa/`, `docs/receipts/`, and five pre-existing ones), so their contents were not hashed one by one. The files this package changed inside them are `evidence/{paths,publish,migrate,mod}.rs` and four receipts — all listed in §2; **no pre-existing directory** (`Nhl19BigStudio-Tester/`, `corpus/`, `crates/blender-job/`, `crates/tdb-headmesh/`, `app/src/assets/ui/tabs/`) was written to |
| Files modified outside this package's scope | **none** — the changed set is the 11 paths in §2 |
| `docs/audits/**`, `anchors/**`, `defs/**` | **not written** |
| Historical evidence records (`docs/validation/c0-0*`, `c0-fx1`, `c0-dt1`, `c0-int1`) | **unmodified.** The corrections are additive, clearly-labelled blocks inside the **bulk receipts**; no historical record was rewritten |
| Migration installation | **not performed** — no candidate installed anywhere |
| Commits / pushes | **none** |

## 7. Remaining gaps

| # | Gap | Class |
|---|---|---|
| **G1** | **No durability against power loss or OS crash.** No `fsync` anywhere. Implementing it means flushing each file and its parent directory and testing the interruption, which is its own package | deliberate, stated |
| **G2** | **No concurrency control.** The boundary checks are TOCTOU-vulnerable to a privileged concurrent writer; no operation locks | deliberate, stated |
| **G3** | **S2 remains partial.** The four `QA_WRITE=1` emitters are still unwired (Q-C), and `qa-evidence check` has still never been validated against a **non-empty** evidence tree. No CI gate may depend on S2 until both are done | blocked on Q-C sequencing |
| **G4** | The **canonical adapter format** still cannot represent enum stored codes, null sentinels, string-pool behaviour, companion/cardinality rules or capacity (U-A3/U-A4/U-A6) | not implemented; out of this package's scope |
| **G5** | Checkpoint 7's consolidated suite ran on the live tree under a claim now withdrawn. The three verified endpoints stand; **writes outside them were not measured**, and `src-tauri/gen/schemas/` (gitignored) is a known repository write path that was not hashed beforehand | recorded in §2.5 of that receipt; the suite is **not** re-run |
| **G6** | Every product question from C0-BULK1 (P1–P4, M1/P3, U2, U3, FR2, FR3, FR12) is **untouched** — no product decision was authorized here | architect / user |

Stopping for Codex review.

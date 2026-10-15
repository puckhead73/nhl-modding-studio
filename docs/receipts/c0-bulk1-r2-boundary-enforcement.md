# C0-BULK1-R2 — Closing the filesystem-boundary bypasses

> **Package:** C0-BULK1-R2, a bounded repair of the boundary bypasses Codex review found in
> [C0-BULK1-R1](c0-bulk1-r1-publication-safety.md). **Date:** 2026-09-07.
> **Author role:** developer. Reviewer: Codex (architect).
> **Status: Passed.** Nothing staged, committed or pushed. **No live evidence publication and no
> migration installation.** No product decision taken. Every publication, recovery and migration
> execution in this package ran in an isolated fixture directory under the system temp tree.

R1's digest-driven recovery, interrupted-retention handling, attempt identities and no-pruning
policy are **preserved unchanged**. This package only narrows what a path, a journal or a caller is
allowed to do.

---

## 1. What was wrong, and what it is now

| # | Defect | Repair |
|---|---|---|
| **E1** | `write_to_guarded` resolved the output **directory** and then wrote its children with a bare `fs::write`, which follows an existing child link. A link planted at an output file name, at `build-inputs.json`, or at the journal's `.new` temporary redirected the write out of the checked directory | New `paths::resolve_file_for_write` validates **every mutation target** in its own right. `paths::write_file` / `copy_file` / `rename_file` are the only write paths the evidence code uses, and every call site in `publish.rs`, `migrate.rs` and `store.rs` goes through them |
| **E2** | Link detection used `FileType::is_symlink` alone | `paths::is_reparse_point` additionally tests `FILE_ATTRIBUTE_REPARSE_POINT` on Windows, so reparse classes that are not symlinks or mount points are caught too |
| **E3** | Presence was established with `exists()`, which reports a **dangling** link as absent — so the write proceeded and created the link's target | Presence is established with `symlink_metadata` throughout, including `store::write_manual_record`'s never-overwrite check |
| **E4** | An existing directory (or other non-regular entry) at a file's name was not distinguished from an absent one | `path-unexpected-entry-type` refuses any target that exists and is not a regular file |
| **E5** | The journal checked only that each target name was *a member* of the output set. A modified journal could **omit** targets — recovery acts only on recorded ones, so an omitted target stayed half-published with no diagnostic — or **repeat** one, making its previous/staged digests ambiguous | `Journal::validate` now checks the **exact** expected set: no duplicates, no omissions, nothing extra. The expected set is derived from `OUTPUT_FILES` and `provenance_name`, so it cannot drift from `build` |
| **E6** | Digest and id fields were unconstrained strings | `sha256:<64 lowercase hex>` and a 16-lowercase-hex attempt id are both enforced. The id matters structurally: both working-directory names are built from it, so an unconstrained id was an unconstrained path |
| **E7** | Staging and prev only had to be **somewhere under** `data_dir`. Recovery removes both recursively, so a modified journal could point `prev` at any existing child — an operator's directory beside the published set — and have recovery delete it | Both must equal the exact derived direct child (`.qa-publish-<id>` / `.qa-prev-<id>`) of the resolved published directory, and they must differ from each other |
| **E8** | Cleanup was `fs::remove_dir_all` on the journal's strings | `cleanup_owned_dir` verifies ownership and membership first: every entry must be a regular file belonging to the output set. Anything unexpected **refuses and keeps the whole directory** — an unexplained file inside a recovery directory is evidence, not garbage |
| **E9** | Public `write_to` / `publish` / `migrate` wrappers passed an **empty** protected set, so a library caller silently received weaker checks than the CLI | The wrappers are **removed**. A validated `paths::Layout` is a required argument on all three. There is no unguarded overload to reach for, so CLI and library callers get identical checks |
| **E10** | `migrate::install` checked the candidate against the evidence **root** only | It builds a `Layout` and checks the candidate against every protected location, so a candidate inside `manual/`, `legacy/frozen/` or `auto/runs/` is refused by name |
| **E11** | *(found while repairing)* `assert_no_reparse_points` tried to `stat` path **prefix** components. On Windows a verbatim prefix (`\\?\H:`) returns `ERROR_INVALID_FUNCTION`, and the fail-closed branch turned that into a hard error for any already-canonical path | Prefix and root components are skipped — neither can be a reparse point — and everything below them is still checked |

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/paths.rs` | `is_reparse_point`, `resolve_file_for_write`, `write_file`, `copy_file`, `rename_file`, `Layout`; prefix-component fix; 5 new unit tests (13 total) |
| `crates/tdb-qa/src/evidence/publish.rs` | `write_to` takes a `Layout` and validates each file; `publish` takes a `Layout` and the `publish_guarded` wrapper is gone; journal target-set / digest / id / derived-name validation; `cleanup_owned_dir` replaces both `remove_dir_all` calls; all moves and copies routed through the checked helpers |
| `crates/tdb-qa/src/evidence/migrate.rs` | `migrate` takes a `Layout` and `migrate_guarded` is gone; `install` checks the candidate against the whole layout; every write routed through `paths::write_file` / `copy_file` |
| `crates/tdb-qa/src/evidence/store.rs` | `write` routed through `paths::write_file`; the manual-record never-overwrite check uses `symlink_metadata`, not `exists` |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | Builds the `Layout` once per mutating command and passes it through |
| `crates/tdb-qa/tests/evidence_publication.rs` | **26 → 36 cases**: the 10 R2 scenarios, plus the reparse-point planting helpers |
| `crates/tdb-qa/tests/evidence_migration.rs` | Adopts the mandatory `Layout` |
| `docs/receipts/c0-bulk1-r1-publication-safety.md` | Labelled review correction withdrawing D11's "no caller can skip them" claim |

## 3. What is enforced, and what is not

**Enforced.** Every mutation target — output files, `build-inputs.json`, the journal, the journal's
`.new` temporary, staged copies, retained copies, migration claims, manual records — is resolved and
type-checked before it is written, copied onto or renamed onto. Reparse points are refused at any
component *and* at the target itself, dangling or not. A journal can name only the exact expected
target set, only syntactically valid digests and ids, and only the two directory names derived from
its own validated id. Recursive deletion happens only after proving the directory holds nothing but
expected output files. Protection is not optional at any entry point.

**Not enforced, and not claimed.**

* **No concurrency control.** These remain check-then-act guards. A privileged concurrent writer can
  replace a component between the check and the write. No operation takes a filesystem lock; R2 was
  explicitly not asked to add one, and does not pretend to have.
* **No power-loss or OS-crash durability.** Unchanged from R1: no `fsync`/`FlushFileBuffers`
  anywhere. Guarantees hold against *process* interruption only.
* **Hardlinks are not detected.** A hardlink is not a reparse point and carries no marker on the path
  that leads to it; distinguishing one needs a file-index comparison this revision does not do.
* **The evidence root itself is trusted.** `Layout::new` validates the root it is given; it does not
  verify that root is the *right* one.

## 4. Regression tests — 10 R2 cases, isolated fixtures only

All run under `%TEMP%\tdb-qa-evidence-pub\`. Nothing reads or writes `docs/audits/`, `anchors/`,
`defs/` or any game tree. Every protected root and every sentinel is synthetic and built by the test.

| Case | Scenario | Asserted |
|---|---|---|
| `r2_an_output_file_that_is_a_link_to_a_protected_sentinel_is_refused` | reparse point planted at the output file name, aliasing a synthetic `manual/` | `path-reparse-point`; sentinel **bytes and `manual/` membership unchanged** |
| `r2_a_dangling_output_link_is_refused_rather_than_created_through` | dangling reparse point at the output file name | `path-reparse-point`; the link's target **was not created** |
| `r2_a_link_planted_at_the_journal_temporary_cannot_redirect_the_journal` | reparse point at `.qa-publish-journal.json.new` | `path-reparse-point`; sentinel bytes unchanged; **nothing published** |
| `r2_a_journal_pointing_prev_at_an_unrelated_child_cannot_delete_it` | `prev` re-pointed at a bystander directory inside `data_dir` holding an irreplaceable file | `journal-containment`; the file and the directory **both survive** |
| `r2_a_journal_whose_staging_and_prev_are_the_same_location_is_refused` | identical staging/prev | `journal-containment`; published digests unchanged |
| `r2_a_journal_that_duplicates_or_omits_a_target_is_refused` | a duplicated target, then a removed one | `journal-target-set` both times; published digests unchanged |
| `r2_a_journal_with_a_malformed_id_or_digest_is_refused` | `id = "../../escape"`, then a non-digest `staged_digest` | `journal-id` / `journal-digest` |
| `r2_an_unexpected_file_in_a_cleanup_directory_stops_the_deletion` | an operator note dropped into a real interrupted transaction's staging directory | `cleanup-unexpected-entry`; the note's **bytes survive** and the directory is retained |
| `r2_every_library_mutation_entry_point_requires_a_validated_layout` | library `write_to` and `publish` aimed at `manual/` and `auto/runs/` | `path-overlap`; evidence bytes and membership unchanged |
| `r2_an_ordinary_publication_and_recovery_still_succeed` | two generations, second interrupted mid-install, recovered | 8 files installed exactly, `phase_found = installing`, journal `completed` |

### Are they load-bearing?

Two deliberate mutations of the source, each reverted immediately:

| Mutation | Cases that failed |
|---|---|
| `paths::write_file` reverted to a bare `fs::write` (the R1 behaviour) | the three reparse-point cases |
| journal duplicate/omission checks and `cleanup_owned_dir` neutered | the target-set case and the cleanup case |

The remaining cases (`journal-containment` ×2, `journal-id`/`journal-digest`, mandatory layout,
happy path) assert checks that did not exist in R1 at all — R1's containment test was
`starts_with(data_dir)`, which accepts every one of the paths these cases plant.

### Link-creation coverage — reported explicitly

**Creating a Windows *file* symlink needs `SeCreateSymbolicLink` (elevation or Developer Mode) and
is not available in the session these tests were last run in.** The helpers therefore fall back to a
directory **junction**, which needs no privilege and *is* available here; the three link cases
printed `planted a directory junction …` and **executed**, they did not skip.

That fallback is not a weaker test of the check under test — `resolve_file_for_write` refuses any
reparse point, and a junction is one. What it does **not** cover is a file symlink's distinct reparse
tag reaching the identical branch. Where neither can be created the case prints a skip line and
returns; per the evidence rules that is a **skip, not a pass**.

## 5. Verification

| Check | Command | Result |
|---|---|---|
| Publication regressions | `cargo test -p tdb-qa --test evidence_publication` | **Passed** — 36/36 |
| Unit tests | `cargo test -p tdb-qa --lib` | **Passed** — 60/60 |
| S1 acceptance | `cargo test -p tdb-qa --test evidence_acceptance` | **Passed** — 32/32 |
| S3 migration | `cargo test -p tdb-qa --test evidence_migration` | **Passed** — 12/12 |
| Pre-existing generator regression | `tests/inventory.rs` | **Passed** — 8/8 |
| Strict clippy, scoped | `cargo clippy -p tdb-qa --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, repo-wide | `cargo fmt --all -- --check` | **Passed**, exit 0 |
| Live evidence publication / migration installation | — | **Not run.** Not authorized |
| Broad workspace suite | — | **Not run.** Not required by this package |

**148 tdb-qa tests pass.** Per the evidence rules that is baseline health, not target qualification.

## 6. Preservation checks

| Check | Measured how | Result |
|---|---|---|
| HEAD | `git rev-parse HEAD` | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25`, unchanged |
| Index | `git diff --cached --name-only` | **empty** — 0 entries |
| `docs/audits/data/**` bytes | `sha256sum -c` against the checkpoint-7 baseline list (`H:\c0-bulk1\logs\cp7-audits-before.txt`) | **41/41 match, 0 mismatches** |
| `anchors/**` | 552 files present; `git status --porcelain anchors` | **0 entries** — untouched |
| Files this package modified | the 8 listed in §2, all under `crates/tdb-qa/` and `docs/receipts/` | as listed |
| `Cargo.lock`, `Cargo.toml`, `crates/tdb-qa/Cargo.toml` | **not edited by R2.** They carry the C0-BULK1 changes recorded in that package's receipts and are byte-identical to what it left | unchanged by this package |
| Game trees, private fixtures | not read, not written | — |
| Commits / pushes | **none** | — |

**Stated exactly, so it is not read as more than it is.** The working tree carries 236 entries of
pre-existing modified and untracked work from earlier packages. No baseline hash list of the *whole*
tree was captured at the start of R2, so the claim here is **not** "nothing else in the repository
changed"; it is the narrower, measured claim above: HEAD and the index are untouched, the hashed
audit data and the anchors are byte-identical, and the edits are confined to the eight named files.

## 7. Remaining gaps

| # | Gap | Class |
|---|---|---|
| **H1** | **No concurrency control** — check-then-act, TOCTOU-vulnerable to a privileged concurrent writer. Explicitly out of scope for R2 | deliberate, stated |
| **H2** | **No power-loss durability** — no `fsync` anywhere. Unchanged from R1 G1 | deliberate, stated |
| **H3** | **Hardlink aliasing undetected** — needs a file-index comparison | not implemented |
| **H4** | **File-symlink reparse coverage unavailable** in this session; junction coverage substitutes. Re-running under Developer Mode would close it | missing-fixture (privilege) |
| **H5** | R1 gaps **G3–G6** carry over unchanged: S2 partial, canonical adapter format gaps, checkpoint 7's unmeasured write surface, and every product question | see the R1 receipt |

Stopping for Codex review.

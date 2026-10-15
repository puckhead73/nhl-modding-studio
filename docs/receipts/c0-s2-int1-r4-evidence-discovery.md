# C0-S2-INT1-R4 — consistent evidence discovery

**Package:** C0-S2-INT1-R4. **Result:** complete; five walkers corrected onto one shared policy,
13 new cases, three deliberate mutations proving the guards load-bearing.
**Branch:** `feat/phase5-asset-tooling`. **HEAD at start and at end:** `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` (unchanged).
**Not authorized and not done:** staging, commit, push, CI activation, live publication, migration installation.

R3's integrity-aware deduplication is preserved unchanged; R4 only makes the rest of the tree read
the way the journal loader already does, and adds the reparse policy to both.

---

## 1. Audited entry points

Every filesystem read in `crates/tdb-qa/src/evidence/` was inspected — `read_dir`, `is_dir()`,
`is_file()`, `exists()` and `.flatten()`.

### 1.1 Corrected

| Entry point | Defect | Now |
|---|---|---|
| `store::load_runs` | `!runs_dir.is_dir()` → `Ok(vec![])`; `.flatten()`; `.filter(is_dir)` | `paths::child_dirs(&runs_dir, PERMITTED_RUNS_ENTRIES)` |
| `store::walk_toml` (→ `load_manual`, `load_legacy`) | same three, recursively | `paths::walk_files` + the unchanged `.toml` selection |
| `store::walk_all` (→ `manual_digest` V11, `collect_scope_contents`) | same three | `paths::walk_files` |
| `store::collect_scope_contents` | `is_file()` / `is_dir()` on each scope root: an unreadable or link-aliased root classified as *neither* and its contents silently left out of the digest | `symlink_metadata` + `entry_kind_of`; absent still contributes nothing, anything else is an error |
| `migrate::walk` (migration digest) | `!dir.is_dir()`; `.flatten()`; `is_dir()` | `paths::walk_files` |
| `journal::read_entry_kind` (R3) | classified by `is_dir`/`is_file` only — no reparse test | a reparse point classifies as `Other`, so `classify_entry` refuses it as `journal-unsupported-link` |
| `journal::load_journals` root check (R3) | its own bespoke wrong-type message | `paths::optional_dir_exists`, the same rule and token as every other layer |
| `journal::journal_bytes` (R3) | `.flatten()` + `is_file()` | corrected in R3, unchanged here |

The `collect_scope_contents` case is the quietest of the six and worth stating plainly: a scope root
that could not be classified was dropped from the digest **without the digest becoming `unknown`**.
The result was a different-but-plausible digest — which is to say, a stale scope that looked fresh.

### 1.2 Audited and deliberately not changed

| Entry point | Why |
|---|---|
| `publish::cleanup_owned_dir` | already correct, and the pattern the rest now follows: explicit `NotFound`, `entry.map_err`, `symlink_metadata`, `is_reparse_point`, and an allow-list of expected names |
| `paths::resolve_for_write`, `assert_no_reparse_points`, `resolve_file_for_write` | write-side guards; already `symlink_metadata`-based with reparse rejection |
| `publish.rs` `exists()` calls (staging/prev/recovery) | mutation-planning decisions on paths this crate creates, not evidence discovery; each is followed by a guarded write that re-validates |
| `emit.rs:600` `!path.exists()` | asserts a file the emitter just wrote is gone after cleanup; a false "absent" makes the assertion stricter, not weaker |
| `validate.rs:293` `.flatten()` | an `Option` iterator in the cycle detector — not filesystem |
| `store::write_manual_record`'s `symlink_metadata(&path).is_ok()` | already the R2-correct form: `exists()` would report a dangling link absent and write through it |

### 1.3 Explicit exclusions (file-selection rules preserved)

| Rule | Statement |
|---|---|
| `manual/` and `legacy/claims/` records | **only** `.toml` files are parsed as records — unchanged. A non-`.toml` file is excluded from *parsing*, never from *sight*: `walk_all` feeds the V11 `manual_digest` over the same traversal, so a README beside a record cannot be added, edited or removed without the fingerprint changing. Tested. |
| `auto/runs/` | `store::PERMITTED_RUNS_ENTRIES` — **empty**, and asserted empty by test |
| `auto/in-progress/` | `journal::PERMITTED_AUXILIARY_ENTRIES` — **empty** (R3), and asserted empty by test |
| `EntryKind::Other` anywhere in an evidence tree | refused, never skipped |

Both permitted-entry lists are empty *by decision*, not by accident: nothing in this crate writes a
file at either level, and the emitter's only temporary (`journal.json.new`) lives inside a journal
directory and is renamed over its target. The lists exist so that permitting an entry later is a
recorded decision rather than a widened filter predicate.

---

## 2. The shared policy

Five primitives in `paths.rs`, so the rules are stated once and cannot drift between layers.
Discovery **and** digest calculation both go through them, which is what makes "the digest covers
exactly what discovery saw" a property rather than a coincidence.

| Primitive | Contract |
|---|---|
| `entry_kind(path)` / `entry_kind_of(path, meta)` | no-follow classification; a reparse point is refused **before** dir/file classification (`discovery-reparse-point`); a metadata failure is an error, never "not a directory" |
| `optional_dir_exists(path)` | `Ok(false)` **only** for `NotFound`. An existing root of the wrong type is `discovery-unexpected-root-type`; unreadable is `Io`; a link is `discovery-reparse-point` |
| `read_dir_sorted(dir)` | a failed *iteration step* propagates — entries after it exist and would never be seen |
| `walk_files(dir)` | depth-first, sorted, files only; absent root yields nothing; `Other` entries refused; no link followed, and a `visited` set of canonical directory paths makes `discovery-cycle` reachable if a platform ever presented one twice |
| `child_dirs(dir, permitted)` | immediate subdirectories; a non-directory not in `permitted` is `discovery-unexpected-entry` |

`is_reparse_point` is the **existing** R2 check, reused verbatim: `FileType::is_symlink` plus the
Windows `FILE_ATTRIBUTE_REPARSE_POINT` attribute, because `is_symlink` does not cover every reparse
class. No new platform code was written.

**Cycles.** With every reparse point refused, a tree of real directories cannot contain itself, so
the `visited` guard is defensive rather than load-bearing — stated here rather than implied by a
test that cannot exist. On Windows a junction is caught as a link before the cycle question arises;
the test at `r4_helper_optional_dir_exists_separates_the_four_conditions` plants a junction pointing
at its own parent and confirms the link check fires first.

---

## 3. Test results

New file `crates/tdb-qa/tests/evidence_discovery.rs`, **13 cases**, all over synthetic roots in the
system temp tree. Nothing reads or writes `docs/audits/`, `anchors/`, a game tree or a private
fixture.

**Links are real, not simulated, in both groups.** `mklink /J` needs no elevation, so the Windows
cases create genuine junctions; `#[cfg(unix)]` creates a symlink. Neither path skips.

### 3.1 End-to-end — through the production traversal, asserting qualification

These plant on-disk state and go through `store::load_runs` / `load_manual` / `load_legacy` /
`journal::load_journals` — the same four loaders, in the same order, that `check` and `build` call.

| Case | What it proves |
|---|---|
| `r4_a_run_that_cannot_be_discovered_fails_instead_of_qualifying` | **The package's central case.** An older passing run plus a failing sealed run ⇒ `withheld / conflict`, `unresolved` naming `uncleared-executed-failure`. Move the failing run out of the tree — the silent-loss outcome — and the row becomes `automated-qualified`. Put an unexplained file where it stood and discovery **fails** (`discovery-unexpected-entry`) instead of publishing that green row |
| `r4_a_wrong_type_runs_root_is_an_error_not_an_empty_layer` | a file at `auto/runs` is `discovery-unexpected-root-type`, not "no runs" — the failing run's verdict cannot vanish with the root |
| `r4_a_link_inside_the_runs_root_is_refused` | a junction beside the runs is `discovery-reparse-point`, so material outside the evidence root is never read as a run |
| `r4_a_lost_manual_record_must_not_resolve_a_disagreement` | two disagreeing manual records, one **nested** two levels deeper ⇒ `withheld / conflict`. Delete the nested one and the conflict is simply gone (`reason` becomes `stale-evidence`), with nothing in the output saying a record went missing. Also asserts both nested records load |
| `r4_a_wrong_type_manual_root_is_an_error_not_an_empty_layer` | layer B does not read as absent; **and** `manual_digest` — the V11 traversal — refuses the same tree rather than fingerprinting nothing |
| `r4_a_link_inside_the_manual_tree_fails_both_discovery_and_the_digest` | discovery and digest agree about a link: both refuse |
| `r4_the_digest_and_record_discovery_walk_the_same_tree` | a `README.md` beside a record is **not** parsed as evidence, but **is** covered by the V11 digest |
| `r4_a_valid_empty_root_and_a_normal_tree_are_unaffected` | absent roots, present-but-empty roots, and an ordinary tree that loads both layers and still qualifies |
| `r4_the_journal_root_and_its_entries_share_the_platform_policy` | a link among the journals is `journal-unsupported-link`; a wrong-type journal root now reports the same `discovery-unexpected-root-type` as every other layer |

### 3.2 Helper-level — deterministic fault injection

Labelled as such in the file, because they are **not** end-to-end filesystem coverage: they exist for
branches that cannot be produced reliably on every platform.

| Case | Branch |
|---|---|
| `r4_helper_optional_dir_exists_separates_the_four_conditions` | absent / present / wrong-type / link, one assertion each |
| `r4_helper_a_failed_child_metadata_read_is_not_an_absence` | a stat that fails; the error names the path |
| `r4_helper_walk_files_is_sorted_deep_and_refuses_links` | sorted depth-first order, absent root, link refused mid-walk |
| `r4_helper_child_dirs_permits_only_what_is_named` | an unnamed file refused; a named one permitted |

### 3.3 Proving the guards are load-bearing

Each was reverted in source, the suite re-run, and the file restored from a byte-for-byte copy.

| Mutation | Result |
|---|---|
| `optional_dir_exists` → `Ok(path.is_dir())` (the old absence guess) | **4 failed** — both wrong-type-root cases, the journal-policy case, the helper case |
| the `is_reparse_point` test removed from `entry_kind_of` **and** from `journal::read_entry_kind` | **4 failed** — both link cases and two helper cases |
| `child_dirs` silently skipping an unexpected file | **2 failed** — the central run-loss case and the helper case |

One honest note on the second mutation: on Windows a junction's `symlink_metadata` reports neither
`is_dir` nor `is_file`, so it still lands in `EntryKind::Other` and is still refused — with a
different token. The cases fail on the token, not on a green result. The explicit reparse test is
what covers the reparse classes `is_symlink` does not, which is exactly why R2 wrote it.

### 3.4 Suite, lint, format

| Check | Result |
|---|---|
| `cargo test -p tdb-qa` | **206 passed, 0 failed, 0 ignored** (67 lib + 32 acceptance + **13 discovery** + 7 emit + 31 journal + 12 migration + 36 publication + 8 inventory) — 193 before, +13 |
| `cargo clippy -p tdb-qa --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | exit 0 |

No emitter was re-run and no full workspace suite was run, per the package.

---

## 4. Contract clarification

`docs/plans/qa-evidence-contract.md`:

* **§4.3** gains row **4b** in the qualification table — `a damaged in-progress journal declares the
  key ⇒ withheld (reason = damaged-evidence)` — with a note on why it sits directly below 4a. R2
  implemented this row but never wrote it into the table; the table now matches the code.
* **§4.6 (new)** lists the integrity findings, marks each damaging or not, and documents
  **`sealed-run-unverifiable`** among them. It states explicitly that it introduces **no new conflict
  class, axis value or qualification outcome** — its only distinct effect is on deduplication — and
  records the two conditions that are fatal at load rather than reported (damaged declarations, and
  damage in a journal that declares nothing).
* The status header notes the amendment as a clarification of implemented behaviour.

No product behaviour changed as a result of this section.

---

## 5. Preservation

| Item | Result |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25`, unchanged |
| git index | empty |
| `git status --porcelain` | one new entry, `?? crates/tdb-qa/tests/evidence_discovery.rs`; everything else identical to R3 |
| real evidence `H:\c0-s2-int1\run1` | 4 sealed runs, 0 journals — unchanged |
| real evidence `H:\c0-s2-int1\run3` | 4 sealed runs, 0 journals — unchanged |
| `docs/audits/`, `anchors/` | untouched |

Both preserved roots still load and rebuild **byte-identically**:

```
check --root run1 --data out-r1      → ok: 2016 row(s), no validation failure, no drift   (exit 0)
check --root run3 --data out-r2-run3 → ok: 2175 row(s), no validation failure, no drift   (exit 0)
build --root run1 → diffed file-by-file against R2's published set: no differences
```

That the rewritten traversal produces identical digests over the real trees is the load-bearing
observation here: `manual_digest`, the migration digest and `collect_scope_contents` all changed
walker, and none of them changed a value.

---

## 6. Remaining limitations

1. **`EntryKind::Other` is now fatal inside an evidence tree.** A FIFO, socket or device node under
   `manual/` refuses the whole load rather than being skipped. That is the intended direction, but it
   is a behaviour change for anyone who had such an entry; none exists in either preserved root.
2. **The cycle guard is defensive, not exercised.** No test can create a cycle without a link, and
   links are refused first. `discovery-cycle` is therefore unreached code with a clear message.
3. **Permission-denied is still not covered end-to-end.** It cannot be produced reliably inside a
   temp tree the test owns; §3.2's helper case covers the code path with a stat failure of a
   different kind. On Linux CI (C0.2 R6, open) a real `chmod 000` case would be stronger.
4. **`load_runs` reads each run through `load_run`, which was not re-audited here.** The package
   scoped this to discovery; the per-run file reads inside a discovered directory are R2's territory
   and unchanged.
5. **These are check-then-act guards, not locks.** As `paths.rs` already documents, a sufficiently
   privileged concurrent writer can replace an entry between classification and reading. Nothing here
   claims otherwise.
6. **No new evidence was produced.** The four emitters were not re-run, so R1's and R2's published
   figures remain the most recent real measurements.

---

## 7. Files changed

| File | Change |
|---|---|
| `crates/tdb-qa/src/evidence/paths.rs` | `EntryKind`, `entry_kind`, `entry_kind_of`, `optional_dir_exists`, `read_dir_sorted`, `walk_files`, `child_dirs` |
| `crates/tdb-qa/src/evidence/store.rs` | `load_runs`, `walk_toml`, `walk_all`, `collect_scope_contents`; `PERMITTED_RUNS_ENTRIES` |
| `crates/tdb-qa/src/evidence/migrate.rs` | `walk` |
| `crates/tdb-qa/src/evidence/journal.rs` | reparse test in `read_entry_kind`; root check via `optional_dir_exists` |
| `crates/tdb-qa/tests/evidence_discovery.rs` | new, 13 cases |
| `docs/plans/qa-evidence-contract.md` | row 4b in §4.3; new §4.6; status header |
| `docs/receipts/c0-s2-int1-r4-evidence-discovery.md` | this receipt |

## 8. Unresolved questions for Codex

* Limitation 1 — is refusing an `EntryKind::Other` entry inside `manual/` the policy you want, or
  should a named exclusion list cover it the way `PERMITTED_RUNS_ENTRIES` does for files?
* R3's §9 question stands: `load_runs` is now corrected, but the same audit has not been extended
  outside `crates/tdb-qa/src/evidence/`.

**Stopping for Codex review.** Nothing staged, committed or pushed.

---

## Revision — C0-S2-INT1-R5 (2026-09-07): §1's audit-completeness wording overstated the scope

**Corrected claim.** §1 was headed *"Every filesystem read in `crates/tdb-qa/src/evidence/` was
inspected — `read_dir`, `is_dir()`, `is_file()`, `exists()` and `.flatten()`"*, and §1.2 presented
its list as the complete set of audited-and-unchanged entry points. Both overstate what was done.

The audit was of **directory discovery and digest traversal**. It did not cover per-file reads, and
two instances of the same defect class were in the crate at the time and were not caught:

```rust
// store::read_unmapped, store::read_facets
let Ok(text) = fs::read_to_string(path) else { return Ok(Vec::new()); };
```

The grep patterns that found the walkers do not match that form, and nothing in the R4 pass would
have. Read §1 as "every directory walker and digest traversal", not "every read".

**Closed in R5.** Both sites now read through `paths::read_optional_file`, which yields empty data
for `NotFound` alone; the per-file read inventory is completed in
[c0-s2-int1-r5-file-read-integrity.md](c0-s2-int1-r5-file-read-integrity.md) §2, including the
intentional fallbacks and their contract justification.

**Unaffected.** No figure or test result in this receipt changes; R4's discovery corrections stand as
recorded, and both preserved roots still rebuild byte-identically under R5.

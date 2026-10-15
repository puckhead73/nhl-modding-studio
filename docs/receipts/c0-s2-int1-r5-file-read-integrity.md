# C0-S2-INT1-R5 — finish evidence file-read semantics

**Package:** C0-S2-INT1-R5. **Result:** complete; the two confirmed defects corrected, the bounded
read audit finished, 9 new cases, three deliberate mutations.
**Branch:** `feat/phase5-asset-tooling`. **HEAD at start and at end:** `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` (unchanged).
**Not authorized and not done:** staging, commit, push, CI, live publication, migration installation.
**Scope held:** nothing outside `crates/tdb-qa/src/evidence/` was changed.

Architect decisions applied as given: non-file/non-directory entries under `manual/` remain errors;
no new exclusions were added; no repository-wide audit was performed.

---

## 1. The confirmed defects

`store::read_unmapped` and `store::read_facets` both opened with:

```rust
let Ok(text) = fs::read_to_string(path) else { return Ok(Vec::new()); };
```

The fallback exists for a real reason — a run written before these files existed is not malformed —
but `let Ok(…) else` extends that reason to **every** failure. An unreadable file, invalid UTF-8, a
directory or a link at the name all produced the same answer: *this run made no unmapped
observations and recorded no facets*. That is a positive claim about the run, manufactured from an
error, and it is exactly the shape R2 closed for journals and R4 closed for directories.

Both now read through a shared classifier:

```rust
let Some(text) = super::paths::read_optional_file(path, RUN_WRONG_TYPE, RUN_UNREADABLE)? else {
    return Ok(Vec::new());
};
```

`paths::read_optional_file` (new) is the file-level analogue of R4's `optional_dir_exists`:

| Condition | Result |
|---|---|
| `NotFound` | `Ok(None)` — the only reading of absence permitted |
| link, junction or reparse point at the name | `discovery-reparse-point` |
| directory or other non-regular entry | the caller's `wrong_type_rule` |
| not valid UTF-8 | the caller's `unreadable_rule` |
| any other I/O failure | `StoreError::Io` naming the path |

The type check is `symlink_metadata`, so no link is followed to decide what stands at an evidence
file's name. Rule tokens are parameters because each layer reports in its own vocabulary
(`run-unexpected-entry-type` / `run-unreadable-file` for runs); the *classification* is shared, which
is the point. `journal::read_optional` — R2's separate copy of the same logic, which lacked the
reparse check — now delegates to it and keeps its own tokens, so R2's and R3's assertions are
unaffected while the platform policy becomes uniform.

**Backward compatibility is preserved and proved on real data.** The preserved root
`H:\c0-s2-int1\run1` is genuinely an older-layout set: all four of its runs predate these files and
carry neither. It still loads, still validates and still rebuilds byte-identically (§5).

---

## 2. Completed read-path inventory

Every read in `crates/tdb-qa/src/evidence/` — per-run, per-record and their immediate callers, not
only directory walkers.

### 2.1 Reads that propagate every failure (correct, unchanged)

| Read | Caller |
|---|---|
| `store::read` → `manifest.json`, `keys-declared.txt`, `keys-executed.txt`, `observations.csv` | `load_run`; each is mandatory, and V4 digests them |
| `store::read` → each `.toml` | `parse_manual`, `parse_legacy` via `load_manual` / `load_legacy` |
| `fs::read` in `store::manual_digest`, `store::collect_scope_contents` | V11 fingerprint; scope freshness inputs |
| every `fs::read`/`read_to_string` in `migrate.rs` | frozen snapshots and migration digests |
| `publish::digest_file`, `digest_if_present`, `Journal::read`, `cleanup_owned_dir`, `build-inputs.json` | already `NotFound`-only, with the reasoning documented in place |
| `journal::read_optional` | R2; now delegating to the shared classifier |

### 2.2 Intentional fallbacks, with justification

| Fallback | Justification |
|---|---|
| `read_unmapped` / `read_facets` → empty on **`NotFound` only** | a run written before these files existed carries no such records. This is the schema-permitted optional file, and after R5 the fallback covers absence and nothing else |
| `state.json` unreadable → `interrupted` | **contract R-STATE**: "a run directory with no *readable* `state.json` is `interrupted`". Kept — see §2.3 |
| `emit::scope_digests` → empty map when `defs/qa/scopes.toml` cannot be read or parsed | can only make every scope digest `unknown`, i.e. **not fresh everywhere** (R-FRESH-UNKNOWN). It removes qualification and can never grant it, which is the opposite of the `read_unmapped` defect. It also returns a plain map, not a `Result`, because emitters must not panic while capturing identity |
| `emit::file_digest` → `None` when a fixture cannot be read | an unreadable fixture yields no digest rather than a digest-shaped placeholder; `unknown` identity never qualifies (R-FRESH-UNKNOWN) |
| `emit::from_env` → `repo_root.to_path_buf()` when canonicalization fails | affects only the recorded repo path in an emitter's own identity, never a qualification decision |
| `read_optional_file`'s second `NotFound` arm (after the type check passed) | the documented TOCTOU window: a concurrent writer removed the file between check and read. Absence is the honest answer, and `paths.rs` already states these are check-then-act guards, not locks |

Audited and **left alone as out of scope**: `qa_evidence.rs:197` reads `defs/qa/scopes.toml` with
`Err(_) => (None, "unknown")`. Same conservative direction as `scope_digests` (everything becomes
stale, nothing becomes fresh), and it lives in `src/bin/`, outside this package's boundary. Recorded
here so it is not mistaken for an unexamined path.

### 2.3 `state.json` — kept, but no longer misreported

The contract is explicit (§3, R-STATE): *"a run directory with no **readable** `state.json` is
`interrupted`"*. The downgrade therefore stays, and it is the one degradation in this module that is
safe by construction:

* an `interrupted` run is **never selectable** (§4.2), so the downgrade can only ever *remove*
  qualification, never grant it;
* all of its observations are retained and citable, so an executed failure inside it still triggers
  row 4a (`unreviewed-failure`) — the failure is not erased.

Both properties are now asserted by
`r5_an_unreadable_state_marker_cannot_erase_a_failure_or_qualify_anything`, which damages the marker
on a run holding an executed `fail` and checks that the observation survives, the key is not
`automated-qualified`, and `uncleared-executed-failure:` still appears in `unresolved`.

What R5 changes is the **reason**, which is a finding, not a classification. `Err(_)` reported every
cause as `"no state.json"`, so a directory, a link, a permission failure and invalid UTF-8 at that
name were all indexed as an absent terminal marker. Only one of those means "the emitter died before
sealing". The four causes are now distinct:

| Cause | `state` | `failure` |
|---|---|---|
| absent | `interrupted` | `no state.json` |
| present but unreadable (wrong type, link, I/O, invalid UTF-8) | `interrupted` | `state.json is present but unreadable: <rule and path>` |
| present, JSON did not parse | `interrupted` | `state.json did not parse: <detail>` |
| present, state string unrecognised | `interrupted` | `unknown terminal state "<value>"` |

No qualification outcome changes as a result; the diagnostic stops lying.

### 2.4 `walk_files_into` canonicalization

`fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())` now propagates. The fallback defeated
the guard it fed: two spellings of one directory would insert as two distinct entries and the cycle
check would never fire. The directory has just been proved to exist by `optional_dir_exists`, so a
failure here means something changed under the walk.

---

## 3. Regression tests

New file `crates/tdb-qa/tests/evidence_file_reads.rs`, **9 cases**, all over synthetic roots in the
system temp tree. Nothing reads or writes `docs/audits/`, `anchors/`, a game tree or a private
fixture.

| Case | Covers |
|---|---|
| `r5_an_older_run_without_the_optional_files_still_loads` | legitimate absence — both files removed; run loads `completed` with empty lists, observations intact, and `load_runs` sees it |
| `r5_valid_optional_contents_are_read_back` | valid contents — 1 unmapped record and 2 facets round-trip through a real emitter run |
| `r5_invalid_utf8_in_an_optional_file_is_an_error_not_an_empty_list` | invalid UTF-8 → `run-unreadable-file` |
| `r5_a_directory_at_an_optional_file_path_is_an_error` | directory at `facets.csv` → `run-unexpected-entry-type` |
| `r5_a_link_at_an_optional_file_path_is_refused` | a **real** junction at `unmapped.csv` → `discovery-reparse-point` |
| `r5_an_injected_read_failure_propagates_with_its_path` | injected read failure (§3.1); the error names the path, and the same run loads normally once the injection is released |
| `r5_only_a_genuine_absence_yields_empty_data` | the boundary itself, at the helper |
| `r5_an_absent_state_marker_and_an_unreadable_one_are_both_interrupted_but_distinguishable` | §2.3's first two rows |
| `r5_an_unreadable_state_marker_cannot_erase_a_failure_or_qualify_anything` | §2.3's two safety properties |

**Non-success reaches `check` and `build`.** Three of the failure cases call the real `qa-evidence`
binary (`CARGO_BIN_EXE_qa-evidence`) and assert: `check` exits **3** (the contract's code for a
malformed active input) with the rule token in stderr; `build` exits non-zero with the same token;
and the build's `--out` directory is left **empty** — a damaged input never becomes a published
matrix.

**Original bytes retained.** The UTF-8 and injected-failure cases hash every file in the run
directory before and after and assert equality; nothing in the read path writes.

### 3.1 The injected read failure

A first attempt used a path whose parent is a regular file. On Windows that resolves to
`ERROR_PATH_NOT_FOUND`, which std maps to `NotFound` — so it landed in the *absent* branch and
proved nothing about read failures. It was replaced with a genuine injection: the file is held open
with `share_mode(0)` (exclusive, no sharing) on Windows, and `chmod 000` on Unix. The entry exists,
classifies as a regular file, and then fails to read — the exact sequence the old `let Ok(text) = …
else` swallowed. Neither form needs elevation.

### 3.2 Proving the corrections are load-bearing

Each was reverted in source, the suite re-run, and the file restored from a byte-for-byte copy.

| Mutation | Result |
|---|---|
| `read_unmapped`/`read_facets` back to `let Ok(text) = … else` | **4 failed** — invalid UTF-8, wrong type, link, injected failure |
| `state.json`'s `Err(e)` arm back to `Err(_) => "no state.json"` | **1 failed** — the absent/unreadable distinction |
| `walk_files_into` canonicalization back to `unwrap_or_else` | **0 failed** — see limitation 2 |

### 3.3 Suite, lint, format

| Check | Result |
|---|---|
| `cargo test -p tdb-qa` | **215 passed, 0 failed, 0 ignored** (67 lib + 32 acceptance + 13 discovery + **9 file-reads** + 7 emit + 31 journal + 12 migration + 36 publication + 8 inventory) — 206 before, +9 |
| `cargo clippy -p tdb-qa --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | exit 0 |

No emitter was re-run, no full workspace suite and no broader audit, per the package.

---

## 4. Correction to R4's receipt

R4's §1 was headed *"Every filesystem read in `crates/tdb-qa/src/evidence/` was inspected"* and
listed the greps used. That wording overstated what was done: the inspection was **directory
discovery and digest traversal**, and the two `let Ok(text) = fs::read_to_string(path) else` sites in
`store::read_unmapped` / `store::read_facets` were in the crate at the time and were not caught — the
grep pattern that found the walkers (`read_dir`, `is_dir()`, `.flatten()`) does not match them. A
labelled revision has been added to `docs/receipts/c0-s2-int1-r4-evidence-discovery.md` saying so.
The per-file read audit is completed here, in §2.

---

## 5. Preservation

| Item | Result |
|---|---|
| HEAD | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25`, unchanged |
| git index | empty |
| `git status --porcelain` | one new entry, `?? crates/tdb-qa/tests/evidence_file_reads.rs`; everything else identical to R4 |
| `H:\c0-s2-int1\run1` | 4 sealed runs, **older layout** — no `unmapped.csv`, no `facets.csv` |
| `H:\c0-s2-int1\run3` | 4 sealed runs, all carrying both optional files |
| `docs/audits/`, `anchors/` | untouched |

```
check --root run1 --data out-r1      → ok: 2016 row(s), no validation failure, no drift   (exit 0)
check --root run3 --data out-r2-run3 → ok: 2175 row(s), no validation failure, no drift   (exit 0)
build --root run1 → diffed file-by-file against R2's published set: no differences
```

The two roots between them cover both sides of the boundary this package moved — `run1` is the older
layout the fallback exists for, `run3` exercises the corrected reader on real content — and neither
changed by a byte.

---

## 6. Remaining limitations

1. **Permission-denied is covered by injection, not by a permission bit.** The Windows case uses an
   exclusive share mode and the Unix case `chmod 000`; the latter is a no-op for root, so on a
   root CI runner that case would pass vacuously.
2. **The canonicalization guard is unreached by any test.** Like R4's cycle guard, it needs a
   canonicalize failure on a directory that was just stat'ed successfully, which no test can arrange.
   Mutating it fails nothing — recorded rather than papered over.
3. **`state.json` degradation remains lossy by design.** A `completed` run whose marker becomes
   unreadable is downgraded to `interrupted` and stops qualifying. That is the contract's rule and it
   errs safe, but it does mean an unreadable marker silently costs a legitimate qualification; the
   `failure` text is now the only place that says why.
4. **`qa_evidence.rs`'s scopes read is unchanged** (§2.2), being outside this package's boundary.
5. **These are still check-then-act guards, not locks**, and `read_optional_file` has an explicit
   TOCTOU arm to prove it.
6. **No new evidence was produced.** R1's and R2's published figures remain the most recent real
   measurements.

---

## 7. Files changed

| File | Change |
|---|---|
| `crates/tdb-qa/src/evidence/paths.rs` | `read_optional_file`; canonicalization propagates in `walk_files_into` |
| `crates/tdb-qa/src/evidence/store.rs` | `read_unmapped`, `read_facets`, `state.json` cause distinction; `RUN_WRONG_TYPE` / `RUN_UNREADABLE` |
| `crates/tdb-qa/src/evidence/journal.rs` | `read_optional` delegates to the shared classifier |
| `crates/tdb-qa/tests/evidence_file_reads.rs` | new, 9 cases |
| `docs/receipts/c0-s2-int1-r4-evidence-discovery.md` | labelled revision (§4) |
| `docs/receipts/c0-s2-int1-r5-file-read-integrity.md` | this receipt |

## 8. Unresolved questions for Codex

* Limitation 3 — should an unreadable (as opposed to absent) `state.json` be a hard error rather than
  a downgrade? It would be consistent with R2's fail-closed direction, but it contradicts R-STATE as
  written, so it was not done on my own judgment.
* `emit::scope_digests` returns a map, not a `Result`, so a scopes read failure can only surface as
  universal staleness. Is a louder signal wanted there?

**Stopping for Codex review.** Nothing staged, committed or pushed.

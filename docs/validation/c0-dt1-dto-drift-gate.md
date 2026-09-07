# C0-DT1 — Explicit DTO generation and a non-mutating drift gate

> **Packages:** C0-DT1, then the bounded corrections **C0-DT1-R1** (generator safety and focused
> tests) and **C0-DT1-R2** (recover the failing operation itself). This document records the
> combined, current implementation.
> **Execution dates:** 2026-09-07 (UTC). **Author role:** developer. Reviewer: Codex (architect).
> **Git:** one authorized commit across both packages — the C0-INT1 integration report, `bdcda0c`.
> **No source file was staged or committed. Nothing was pushed.** No dependency or lockfile change.

Conventions follow C0.2: **Passed / Failed / Blocked / Not run** describe a check that actually
executed. A case that returns early rather than exercising its subject is **skipped coverage**,
never a pass — §5.3 names the one such case.

---

## 1. Result

`cargo test` no longer writes the binding tree. The unconditional `export_typescript` test is gone;
in its place is a comparison-only assertion, and installation happens exclusively through a new
binary with validated arguments, preflighted paths and recoverable writes.

| | Before | After |
|---|---|---|
| `cargo test -p studio-api` | wrote all 124 files into `app/src/bindings` | **compares only**; asserts content **and** membership |
| Deliberate generation | none — a side effect of testing | `cargo run -p studio-api --bin export-bindings -- --write` |
| Drift detection | **none** (C0.2 R5) | `--check`, non-mutating, exits nonzero on drift |
| Export-root list | inline in a `#[cfg(test)]` module | one authoritative list shared by both paths |
| Failed installation | partial tree, no recovery | plan validated up front; best-effort rollback of the successful **and** the failing operation; retained recovery material on rollback failure |
| Argument handling | last flag silently won | exactly one mode required; every invalid form rejected before any work |
| `studio-api --lib` runtime | 67.62 s | **6.93 s** |

**The live binding set is byte-identical and membership-identical to its pre-package state**, at
every sampling point in both packages (§6.1).

The current working-tree DTOs and the current working-tree bindings **agree**: 70 export roots
render exactly the 124 files present, with no missing, changed, obsolete or irregular member. This
concerns the **working tree**, not HEAD — 10 binding paths remain modified or untracked relative to
HEAD, exactly as C0.2 recorded.

---

## 2. Implementation

### 2.1 Files changed

| Path | Change |
|---|---|
| `crates/studio-api/src/bindings.rs` | **New.** Export-root list, render/compare/install seams, recovery, and 31 tests. |
| `crates/studio-api/src/bin/export_bindings.rs` | **New.** The `export-bindings` CLI, with a total argument parser and 6 parser tests. |
| `crates/studio-api/src/lib.rs` | Removed the `#[cfg(test)] mod bindings` export test; declared `pub mod bindings`; crate doc note. |
| `crates/studio-api/Cargo.toml` | Registered the `export-bindings` `[[bin]]` target. |
| `CLAUDE.md` | Replaced the obsolete "unconditional export / no drift check" text with the real commands, rules, failure guarantees and CI status. |
| `docs/validation/c0-dt1-dto-drift-gate.md` | **New.** This record. |

No DTO field, TS name, import or IPC semantic changed. **No dependency was added** — `ts-rs 10.1.0`
was already a normal dependency of `studio-api`, so moving the export code out of `#[cfg(test)]`
needed no manifest change, and recovery is built from `std` alone rather than a transaction crate.

`Cargo.lock` is **unchanged, verified by hash** (not by mtime):

```
before  d8c6487f6917e92e0c20ee1c836d8df9a5ff9afc1b4f35bf9e664ad14d0b1622
after   d8c6487f6917e92e0c20ee1c836d8df9a5ff9afc1b4f35bf9e664ad14d0b1622
```

### 2.2 The ts-rs API actually used

Installed version: **`ts-rs 10.1.0`**. The previous test called `TS::export_all()`, which resolves
its target through the **process-global `TS_RS_EXPORT_DIR` environment variable** — the test set
that variable with `std::env::set_var`, which is exactly the process-global race to avoid.

This implementation uses **`TS::export_all_to(out_dir)`** instead. Its documented contract: *"Unlike
`TS::export_all`, this function disregards `TS_RS_EXPORT_DIR`, using the provided directory
instead."* That is a supported, explicit output-directory API, so **no environment variable is read
or written anywhere in the new code, and no child process is needed for isolation.**

No DTO carries `#[ts(export)]` or `#[ts(export_to = ...)]`, so ts-rs performs no automatic export of
its own; every write goes through the roots listed here.

### 2.3 The authoritative export-root list

The 70 roots previously enumerated by hand in the test live in a single `export_roots!` macro
invocation in `crates/studio-api/src/bindings.rs`. It expands to **both** consumers at once —
`render_roots(out_dir)` and `export_root_names()` — so generation and checking cannot disagree.
**All 70 pre-existing roots are preserved verbatim, in the original order**, and the 70 roots
produce 124 files because `export_all_to` exports each root with its whole dependency closure.

### 2.4 Seams

Three stages are separated, which is what makes most behaviour testable without a real render:

| Stage | Entry point | Property |
|---|---|---|
| render | `render_contract()` | the only stage that runs ts-rs; renders into a unique temp dir, reads it back, deletes it |
| compare | `compare(&Contract, dir)` | pure w.r.t. the destination — **never writes** |
| install | `install(&Contract, dir)` | the only stage that mutates the destination |

`check` = render + compare. `write` = render + install. A `Contract` is just `BTreeMap<String,
Vec<u8>>`, so tests can supply a three-file synthetic contract instead of the real 124-file one.

Scratch directories are named `studio-api-bindings-{tag}-{pid}-{atomic seq}-{unix nanos}`, so
concurrent threads and concurrent processes never collide.

---

## 3. Exact commands

```sh
# Check for drift. Renders to a temporary directory, compares, reports sorted
# missing / changed / obsolete / ambiguous / irregular paths. Repairs nothing.
cargo run -p studio-api --bin export-bindings -- --check

# Install the current DTO contract. The only deliberate write path.
cargo run -p studio-api --bin export-bindings -- --write

# Either mode against another directory.
cargo run -p studio-api --bin export-bindings -- --check --dir <path>

# Usage.
cargo run -p studio-api --bin export-bindings -- --help
```

### 3.1 Argument handling

Parsing is **total** and runs to completion before any render, scan or write. Exactly one mode is
required; there is no code path from an invalid invocation to the installer.

| Invocation | Result | Exit |
|---|---|---|
| `--check` in sync | root/file counts + `Bindings are in sync.` | **0** |
| `--check` with drift | each sorted category + the `--write` remedy | **1** |
| `--write` success | `wrote N, removed N, unchanged N, preserved N` plus `+`/`-` lines | 0 |
| **no arguments** | `exactly one of --check or --write is required` + usage | **2** |
| `--check --write` | `--check and --write are mutually exclusive` | 2 |
| `--check --check` | `--check was given more than once` | 2 |
| `--dir a --dir b` | `--dir was given more than once` | 2 |
| `--write --dir` | `--dir requires a path` | 2 |
| unrecognised / stray argument | `unrecognised argument \`x\`` | 2 |
| `--dir --check` (empty or option-looking value) | `--dir value \`--check\` looks like an option, not a path` | 2 |
| `--write` refused before mutation | render / ownership / preflight diagnosis | 2 |
| `--write` failed, rolled back | names the file and confirms restoration | 3 |
| `--write` failed, recovery failed | names uncertain files + retained recovery directory | 4 |

`--dir` values are validated before any work: an empty value, or one starting with `-`, is
rejected. Without that, `--write --dir --check` installed into a directory literally named
`--check`. A legitimate leading-dash directory is still expressible as `./--check`, which was
verified to work. Both mode orders are covered by test (`--write --dir --check` and
`--check --dir --write`).

---

## 4. Comparison, ownership and installation contract

### 4.1 Content comparison

Files are compared as **bytes**, with exactly one normalisation: `\r\n` collapses to `\n` on **both**
sides.

- **Why:** ts-rs always writes LF, but this repository runs with `core.autocrlf=true`, so a fresh
  checkout yields CRLF files that are not substantively different.
- **What it does not hide:** nothing else is normalised. Trailing whitespace, blank lines, a bare
  `\r`, indentation, BOMs and all real content differences survive and are reported. The test
  `crlf_checkout_is_not_reported_as_drift` asserts both halves — a fully CRLF-converted file is
  clean, and one extra newline in the same file is `changed`.
- **What it compares against:** the **current working-tree** DTO contract versus the current
  working-tree binding files. It never reads HEAD, `git show` or the index.

Comparison is by file name and the tree is flat. Only `*.ts` entries directly inside the directory
participate.

### 4.2 Generator ownership

A live `.ts` file is **generator-owned** if and only if its content starts with the ts-rs banner
(`// This file was generated by [ts-rs](…).`). All 124 live bindings carry it.

| Situation | `--check` reports | `--write` does |
|---|---|---|
| Rendered, live counterpart matches | — | nothing (`unchanged`) |
| Rendered, differs, banner present | `changed` | overwrites |
| Rendered, absent | `missing` | creates |
| Rendered, present **without** the banner | `ambiguous` | **refuses everything before writing** |
| Not rendered, banner present | `obsolete` | deletes |
| Not rendered, no banner | `unowned` (informational, **not** drift) | **preserves** |
| Any `.ts` entry that is a directory, symlink or other reparse point | `irregular` | **refuses everything before writing** |

Obsolete files are removed **only** where generator ownership is established. Ambiguous ownership is
rejected, not resolved. Entry types come from `DirEntry::file_type()`, which does not traverse
symlinks, so a linked binding is classified and rejected rather than **followed into another
location** — asserted by test, including that the link target's bytes are untouched.

### 4.3 Supported failure guarantees

Stated precisely, including the weaker claims:

- **Render failure, ownership refusal and preflight rejection occur before the first destination
  byte changes.** In those three classes the destination is genuinely unchanged, and only those
  three messages say so.
- **An I/O failure part-way through installation triggers a best-effort rollback** from recovery
  material written before the first mutation. Undo is per-op: a `Create` is deleted, an `Overwrite`
  or `Delete` is restored from its captured prior bytes — so **bytes and membership** both return.
  On success the recovery directory is discarded.
- **The failing operation is recovered too, not just the ones that succeeded.** A `before`-style
  rejection means the operation was never entered and its target is untouched; but once the
  filesystem operation itself is entered it may have truncated or half-written its target, so it
  joins the undo set. A `Create` that failed is removed (absence *is* the recovered state, and a
  file that was never created counts as already correct); an `Overwrite` or `Delete` that failed is
  rewritten from its captured prior bytes. This was the C0-DT1-R2 defect: the earlier code added an
  operation to the undo set only *after* it succeeded, so a truncated overwrite was left damaged
  while `Recovery::RolledBack` was reported. See §5.5.
- **If the rollback itself fails, the recovery directory is retained**, and the error names each
  uncertain file with its reason and prints the retained location. The message says
  `RECOVERY ALSO FAILED` and never claims the tree is intact.
- **No crash/power-loss atomicity is claimed or provided.** Rollback runs in-process; if the process
  dies mid-install, the retained recovery directory is the only remedy.
- **No protection against a concurrent external writer is claimed or provided.** The destination is
  scanned once; another process writing it concurrently can invalidate both plan and rollback.
- **Re-running `--write` does not necessarily repair a damaged tree.** A truncated or partially
  rewritten file can have lost its ts-rs banner; it is then no longer generator-owned, and a later
  `--write` correctly **refuses** with an ownership error rather than overwriting it. This is the
  ownership policy working as designed, not a regression. When rollback failed, the retained
  recovery directory is the source of truth: **copy the named files back by hand**, then re-run
  `--write`. This consequence is asserted by test, not merely documented.

No transaction framework and no dependency were introduced for this; it is a plan, a backup
directory and a reverse-order undo built on `std`.

---

## 5. Tests

The old `export_typescript` test is deleted. **37 tests** replace it: 31 in
`studio-api::bindings::tests` and 6 CLI parser tests in the binary. **Every drift mutation and every
injected fault happens inside a temporary fixture; no test writes to `app/src/bindings`.**

### 5.1 Fault injection

Installation takes a private `InstallHook` (production uses `NoHook`, which never fails), plus an
injectable recovery root. The hook has two distinct halves, which is what lets the two failure
shapes be told apart:

* `before(phase, action, name)` — a rejection **before** the filesystem is touched;
* `write_file(path, bytes)` / `remove_file(path)` — the operation itself, so a test can write a
  truncated prefix and *then* return `Err`, which is what a real short write or full disk leaves
  behind.

Rollback deliberately uses the real filesystem (gated only by `before`), so a test asserting that
prior bytes were restored is asserting about real bytes on disk. Faults are **deterministic and
positional** — no reliance on filesystem permissions, timing or global state.

### 5.2 Coverage

| Test | Asserts | Result |
|---|---|---|
| `live_bindings_match_the_dto_contract` | **the live gate** — bindings match the DTOs; bytes and membership unchanged by the check | **Passed** |
| `real_render_is_deterministic` | two independent **real** ts-rs renders are byte-identical | **Passed** |
| `export_root_list_is_unique_and_non_empty` | 70 roots, no duplicates | **Passed** |
| `synchronized_bindings_pass_without_writes` | in-sync fixture is clean; tree unchanged | **Passed** |
| `missing_binding_fails_without_writes` | `missing` reported; not recreated | **Passed** |
| `modified_binding_fails_without_writes` | `changed` reported; not repaired | **Passed** |
| `obsolete_owned_binding_fails_without_writes` | `obsolete` reported; not deleted | **Passed** |
| `crlf_checkout_is_not_reported_as_drift` | CRLF tolerated; one extra newline still drift | **Passed** |
| `unrelated_files_survive_generation` | a hand-written `.ts` and a `.txt` survive check and install; the `.ts` is `unowned`, not drift | **Passed** |
| `concurrent_checks_do_not_interfere` | five checks run **simultaneously** (`thread::scope` + `Barrier`); four clean, one drifted; no fixture disturbed | **Passed** |
| `explicit_generation_repairs_every_drift_condition` | install restores a fixture holding all three conditions | **Passed** |
| `repeated_installation_is_deterministic` | second install writes 0; an independent directory is byte-identical | **Passed** |
| `render_failure_never_reaches_the_destination` | injected render fault; bytes **and** membership unchanged | **Passed** |
| `ownership_refusal_leaves_prior_outputs_intact` | refusal mutates nothing | **Passed** |
| `path_collision_is_rejected_before_any_write` | a directory named `Gamma.ts` is rejected; not clobbered; reported `irregular`, not `missing` | **Passed** |
| `junction_reparse_point_is_rejected_and_not_followed` | a Windows junction is rejected; the target directory's contents are untouched | **Passed (executed)** |
| `symlinked_binding_is_rejected_and_not_followed` | as above for a file symlink | **Skipped here — see §5.3** |
| `failure_after_an_installed_file_rolls_back_bytes_and_membership` | fault after one installed file; prior bytes and membership restored; unrelated file intact | **Passed** |
| `failure_after_an_obsolete_removal_restores_the_removed_file` | fault after an obsolete deletion; the deleted file comes back | **Passed** |
| `rollback_failure_retains_recovery_material_and_reports_honestly` | rollback fault: exactly one uncertain file named, recovery directory retained and holding the true prior bytes, message says `RECOVERY ALSO FAILED` and never says "not touched" | **Passed** |
| `successful_rollback_discards_recovery_material` | a successful rollback leaves no recovery material (observed via an injected recovery root, not a scan of shared temp) | **Passed** |
| `partially_overwritten_file_is_restored_exactly` | a truncated overwrite is recovered: exact prior bytes, membership, an earlier successful overwrite undone, unrelated file intact | **Passed** |
| `partially_created_file_is_removed` | a half-written new file is removed; prior bytes and membership unchanged | **Passed** |
| `failed_recovery_of_a_partially_overwritten_file_is_reported_honestly` | damaged file reported `Failed`, never `RolledBack`; exactly one uncertain file named; the damage is real (10 bytes, banner gone); earlier op still undone; retained backup holds true prior bytes; a later `--write` then **refuses** on ownership | **Passed** |
| `error_messages_distinguish_the_failure_classes` | render / ownership / preflight say "destination not touched"; an installation failure never does | **Passed** |
| CLI `valid_invocations_select_exactly_one_mode` | both modes, `--dir`, order-independence | **Passed** |
| CLI `invalid_invocations_cannot_reach_the_writer` | 15 invalid forms each yield the right error and **no** `Invocation` | **Passed** |
| CLI `a_mode_flag_is_never_consumed_as_a_directory` | four invocations across **both** mode orders are rejected, none reaching the writer | **Passed** |
| CLI `dir_values_are_validated` | empty and `-`-leading values rejected; `./--check` still accepted | **Passed** |
| CLI `help_never_yields_a_runnable_invocation` | `--help` in any position | **Passed** |
| CLI `every_cli_error_has_a_message` | no empty diagnostics | **Passed** |

The negative tests assert both the expected report **and** a post-check snapshot equal to the
deliberately-broken state, so a check that "helpfully" repaired the tree would fail rather than pass.

### 5.3 The one skipped case, stated explicitly

`symlinked_binding_is_rejected_and_not_followed` **did not execute on this machine.** Creating a
file symlink on Windows requires Developer Mode or elevation; the attempt fails with
`ERROR_PRIVILEGE_NOT_HELD (os error 1314)`. The test prints an explicit `SKIPPED
(platform-dependent)` line, asserts that the failure reason really was a privilege problem (so an
unexpected failure still fails the test), and returns. **It reports `ok` while covering nothing —
skipped coverage, not a pass.**

That gap is largely closed by `junction_reparse_point_is_rejected_and_not_followed`, which **did**
execute: a Windows junction is a reparse point creatable without elevation, so the
rejection-and-non-traversal path is genuinely exercised here. The symlink case will execute on the
Linux CI runner, where symlinks need no privilege.

### 5.4 The R2 tests are load-bearing — verified, not asserted

The three partial-I/O tests were run against the **pre-fix** code by temporarily neutralising the
one-line change that adds the failing operation to the undo set. All three failed, and the failure
output showed exactly the reported defect:

```
partially_overwritten_file_is_restored_exactly ......... FAILED
partially_created_file_is_removed ...................... FAILED
failed_recovery_of_a_partially_overwritten_file_... .... FAILED

assertion `left == right` failed: prior bytes were not restored exactly
  left:  {"Beta.ts": [47,47,32,84,104,105,115,32,102,105], ...}    # 10 bytes, truncated
  right: {"Beta.ts": [.. full prior contents ..], ...}
```

`Beta.ts` was left truncated to 10 bytes while the old code returned `Recovery::RolledBack` — a
false claim of full restoration. The fix was then restored and all three pass. The temporary edit
was made to the working file and reverted from a byte-exact backup; no other file was involved.

### 5.5 Runtime

| | Before (C0-DT1) | After (C0-DT1-R1) |
|---|---|---|
| `studio-api` lib tests | 18 tests, **67.62 s** | 31 tests, **6.93 s** |
| Real ts-rs renders per run | ~15 | **3** |

The reduction comes from the render/compare/install split: all but three tests now drive small
synthetic three-file contracts. No assertion was weakened and no fixture directory is shared —
every test still owns a private scratch directory.

---

## 6. Verification

All runs on Windows 11, `--offline`, in the live working tree.

| # | Check | Result |
|---|---|---|
| 1 | `cargo build -p studio-api --all-targets` | **Passed** — no warnings |
| 2 | `cargo test -p studio-api` | **Passed** — 31 + 6 + 6 + 1 = **44 passed, 0 failed**, 1 pre-existing ignored doc-test |
| 3 | `cargo fmt --all -- --check` (repo-wide) | **Passed** — exit 0 |
| 4 | `cargo clippy -p studio-api --all-targets -- -D warnings` | **Passed** — exit 0, 0 diagnostics |
| 5 | Live `--check` | **Passed** — `70 export root(s). 124 binding file(s) rendered. Bindings are in sync.`, exit 0 |
| 6 | CLI rejection matrix (5 invalid forms) | **Passed** — each produced its specific message and exit 2 |
| 7 | Junction rejection actually executed | **Passed** — no skip line emitted |
| 8 | `--dir` rejection, both mode orders | **Passed** — `--write --dir --check` and `--check --dir --write` both rejected, exit 2, **no directory created** |
| 9 | `--dir ./--check` still accepted | **Passed** — a legitimate leading-dash path is still usable |
| 10 | R2 tests fail against pre-fix code | **Passed** — §5.4 |

The 31-minute workspace suite was **not** re-run, and no live `--write` was performed in this
revision. Nothing here touches any crate other than `studio-api`.

### 6.1 Binding preservation — content and membership

`app/src/bindings/*.ts` was hashed (SHA-256 per file, sorted manifest) before and after this
revision, and at every sampling point in the original package:

```
b238e1bc306b11c882e3b5e30c6418c1e447f879c2514568ed2d8088e7a9aaa4   (124 files)
```

| Aspect | Before | After |
|---|---|---|
| File count (membership) | **124** | **124** — same names, no additions, no removals |
| Per-file content | 124 hashes | **124 identical hashes** |
| `git status app/src/bindings` | 1 modified, 9 untracked (10) | **unchanged — 10** |

### 6.2 Source preservation

| Aspect | Result |
|---|---|
| Index at every boundary | **empty** |
| `git status` entries | **220 before and after this revision** — no additions, no removals |
| Pre-existing dirty/untracked work | **preserved** — including the mixed `lib.rs` and `CLAUDE.md` edits; no reset, checkout, stash, restore, clean or branch change |
| `Cargo.lock` | **unchanged, hash-verified** (§2.1) — its pre-existing modification is unrelated user work |
| Anchors, `defs/`, `docs/audits/data/**` | **untouched** |
| Overnight repair state (C0-INT1) | intact — repo-wide fmt and scoped strict clippy still pass |
| Scratch / recovery directories | **0 left behind**; retained recovery material in tests lives under an injected fixture root that is cleaned up |

---

## 7. Limitations and remaining work

1. **CI runs the test but nothing runs the CLI.** `.github/workflows/ci.yml` already invokes
   `cargo test --all`, so **the drift test will execute in CI as soon as this implementation is
   committed** — no workflow change is required for that. A separate CI step invoking
   `export-bindings --check` is **not** wired, and CI still runs on `ubuntu-latest` only; Windows CI
   remains C0.2 R6, open.
2. **The gate is working-tree-scoped, by design.** It proves the bindings on disk match the DTOs on
   disk. It says nothing about HEAD, and the 10 binding paths modified or untracked relative to HEAD
   are unchanged by this work.
3. **Ownership rests on the ts-rs banner.** If a future ts-rs release changes that banner, every live
   file would read as `ambiguous` and `--write` would refuse — loudly and safely — but the constant
   would need updating. No test pins the upstream literal itself.
4. **The file-symlink case does not execute on this machine** (§5.3). Junction coverage and the
   Linux CI runner mitigate it; full Windows symlink coverage needs Developer Mode or elevation.
5. **Comparison is name-based and flat.** Subdirectories under `app/src/bindings` are ignored.
   Nothing generates into subdirectories today (no DTO uses `#[ts(export_to = ...)]`), but a future
   DTO that did would be invisible to the check.
6. **Recovery is in-process and best-effort, not transactional.** See §4.3 for the precise
   guarantees and the three explicit non-guarantees: crash/power loss, concurrent external writers,
   and the fact that re-running `--write` does **not** necessarily repair a damaged tree.
7. **A retained-recovery failure requires manual restoration.** The tool cannot fix it on a later
   run, because a truncated file has typically lost its banner and is then correctly refused as
   unowned. The recovery directory holds the exact prior bytes; copying them back is a human step.
8. **Recovery material is written to the system temp directory** (unless a caller injects another
   root). If temp and the binding tree sit on different volumes, or temp itself is full, capturing
   the backups can fail — that failure happens before any mutation, so it is a clean refusal.
9. **Preflight is time-of-check.** A path validated as a regular file could be replaced between the
   scan and the write by an external actor; that is the concurrent-writer non-guarantee, not a
   separate defect.

## 8. What was not done

No QA generator was run and no `docs/audits/data/**` file was touched. No support status, capability
or evidence level was promoted. No fixture, anchor or game directory was read or written. The QA
evidence contract (C0.4) was **not** implemented. No dependency, lockfile, toolchain or CI change.
No `#[ignore]`d test was enabled, removed or weakened. No opportunistic source repair was made — B4,
B5, B6 and B7 from C0.2 remain exactly as recorded. No source file was staged or committed and
nothing was pushed.

# C0-BULK1 · Checkpoint 3 — Observation and safe publication (S2)

> **Package:** C0-BULK1, checkpoint 3 of 7. **Date:** 2026-09-07. **Author role:** developer.
> **Status: SUPERSEDED IN PART — see the review correction below.** Original status was "Passed";
> Codex review found blocking defects in the publication implementation this receipt describes.
> Nothing staged, committed or pushed. **Nothing was published into the live audit tree.**

---

## ⚠ Review correction (C0-BULK1-R1, 2026-09-07)

**S2 is `partial`, not complete.** Two things this receipt implied are corrected:

1. **The publication implementation described below was defective** and has been replaced. Codex
   review found: `prepared`-phase recovery assumed no file had moved although retention moves files
   before journalling; rollback never removed outputs that were absent beforehand; recovery trusted
   the `installed` boolean; the sole journal was truncated in place; identical output content
   collided with an earlier retained generation; generations were auto-pruned by digest order; and
   the boundary checks lived only in the CLI. All are repaired and regression-tested in
   [c0-bulk1-r1-publication-safety.md](c0-bulk1-r1-publication-safety.md). **Read that receipt for
   the current behaviour**; the "Implemented behavior" section below describes the superseded design.
2. **S2 was never fully validated.** Two of its stated prerequisites are still unmet:
   * the four `QA_WRITE=1` emitters have **not** been wired to `observe` (deliberate, Q-C), so no
     emitter produces a run directory in practice;
   * `qa-evidence check` has only ever run against an **empty** evidence tree. There has been **no
     validation against real, non-empty observations**, so the input contract is unproven end to end.

   Until both are done, S2 is **partial** and no CI gate may depend on it.

## Status

**Partial** (corrected from "Passed"). Contract stage **S2** is implemented and exercised against
isolated fixture copies, but its emitter wiring and real non-empty observation validation are
outstanding, and its publication half was replaced by C0-BULK1-R1. `docs/audits/data/` was **not
written to** at any point; every publication in this checkpoint targeted
`H:\c0-bulk1\sandbox-cp3\data` or a temp sandbox.

## Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/csv.rs` | **New.** Minimal RFC-4180 reader/writer. 4 unit tests. |
| `crates/tdb-qa/src/evidence/store.rs` | **New.** Run/manual/legacy load and run write; **V3**, **V4**, **V11**; scope-content collection. 6 unit tests. |
| `crates/tdb-qa/src/evidence/observe.rs` | **New.** Emitter `RunBuilder`. 3 unit tests. |
| `crates/tdb-qa/src/evidence/publish.rs` | **New.** Layer-C rendering, provenance sidecars, journalled publish, `recover`. |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | **New.** `qa-evidence check \| build \| publish \| publish --recover`. |
| `crates/tdb-qa/tests/evidence_publication.rs` | **New.** 9 cases (A2, A4, A29 ×4, A30, +2). |
| `crates/tdb-qa/src/evidence/mod.rs`, `vocab.rs` | Wired the new modules; added `catalog_universe()` (+1 test). |
| `crates/tdb-qa/Cargo.toml` | Registered the `qa-evidence` bin. |
| `defs/qa/scopes.toml` | **New.** Six versioned freshness scopes (contract §5.3, Q-D). |

## Implemented behavior

**Immutable runs.** `write_run` refuses an existing directory — a run is created, populated, sealed.
`state.json` is written **last**, so a crashed emitter leaves a directory that loads as
`interrupted`, never as a completed run. V3 and V4 are enforced on load; the key files are the
membership evidence and the manifest counts are a derived check over them.

**Declared vs. executed membership.** `RunBuilder::observe` *refuses* an undeclared key, so a caller
cannot violate V6. A skip records the key as declared but **not** executed.

**Terminal states.** `completed` / `failed` / `interrupted`, with an absent or unparseable
`state.json` classified `interrupted` (R-STATE) — an unreadable state file is never read as
completed.

**Non-publishing operations.** `build` refuses an `--out` inside the published tree. `check` writes
nothing at all and reports drift (exit 2) or malformed active input (exit 3). Running the binary with
no arguments prints usage and exits 4 — it never writes by accident.

**Journalled publication.** Prepare → **re-verify inputs** → retain → install one file at a time →
seal, with the journal rewritten at each step and after each installed file. The prior generation is
retained under `.qa-prev-<id>/`, and the last 3 generations are kept. `recover` acts per phase:
`prepared` → discard staging; `retained` → restore; `installing` → finish or roll back,
deterministically, from the per-file list; `completed` → no-op.

**R-NO-GIT.** No recovery path invokes git; the publication id is a digest, and pruning orders by
name, not by time.

**Manual records are never machine-written.** The only writer into layer B, `write_manual_record`, is
`pub(crate)` and has **no caller** until the approved migration install lands in checkpoint 4.

## Exact checks and results

| Check | Command | Result |
|---|---|---|
| S2 acceptance cases | `cargo test -p tdb-qa --test evidence_publication` | **Passed** — 9/9 |
| Unit tests (incl. store/observe/csv) | `cargo test -p tdb-qa --lib` | **Passed** — 47/47 |
| S1 acceptance cases (regression) | `cargo test -p tdb-qa --test evidence_acceptance` | **Passed** — 32/32 |
| Pre-existing generator regression | `cargo test -p tdb-qa` (`tests/inventory.rs`) | **Passed** — 8/8 |
| Formatting, scoped | `cargo fmt -p tdb-qa -- --check` | **Passed**, exit 0 |
| Strict clippy, scoped | `cargo clippy -p tdb-qa --all-targets -- -D warnings` | **Failed** — one `dead_code` error: `store::write_manual_record` has no caller until S3 lands. Deliberate; resolved in checkpoint 4, re-run there. |
| End-to-end CLI cycle | see below | **Passed** — but against an **empty** evidence tree; see the review correction |

### End-to-end CLI cycle — isolated, no live write

Root `H:\c0-bulk1\sandbox-cp3\root` (empty), data `…\sandbox-cp3\data`, out `…\sandbox-cp3\out`.
Freshness digests were computed from the **real repository** (read-only) via `defs/qa/scopes.toml`.

| Step | Result |
|---|---|
| `qa-evidence check` (before publishing) | reports 8 files differing from a fresh build, exit **2** |
| `qa-evidence build --out …` | built 8 files, exit 0 |
| `qa-evidence publish --from …` | published 8 files, id `ace6a236837e9f35`; prior generation retained |
| `qa-evidence check` (after) | `ok: 2016 row(s), no validation failure, no drift`, exit 0 |

### What the first real merge reports

**2016 rows** — the declared universe (28 units × 14 families × their non-conversion operations).
With no evidence present:

| Column | Distribution |
|---|---|
| `qualification = automated-qualified` | **0** |
| `qualification = game-verified` | **0** |
| `applicability = not-applicable` | **36** — every one on `nhl07-ps3`, each citing `research-only-unit` |
| everything else | `unqualified`, `reason = no-observation` |

This is the correct report for a tree with no observations, and it **promotes nothing**.

## Evidence / log locations

- `H:\c0-bulk1\logs\cp3-cli-cycle.log` — the full CLI cycle transcript.
- `H:\c0-bulk1\sandbox-cp3\` — the isolated evidence root, staged set and published set.
- Temp sandboxes for the acceptance cases are created and left under
  `%TEMP%\tdb-qa-evidence-pub\`.

## Preservation checks

| Check | Result |
|---|---|
| HEAD / index | `a2bbdc4…` / **empty** |
| `docs/audits/data/**` | **not written.** `git status --porcelain docs/audits/` shows only the pre-existing `README.md` and `assets/nhllegacy/xbox360/capability.csv` modifications recorded in C0.4a §7.1, plus the four pre-existing untracked audits |
| `docs/audits/evidence/` | **does not exist** in the repository; every run, record and publication in this checkpoint lived outside it |
| `anchors/**` | untouched; nothing in this checkpoint reads or writes an anchor |
| Game trees | not read and not written |
| Existing `tdb-qa` generator sources | unmodified |
| `Cargo.lock` | unchanged since checkpoint 2 (`12b777ff…`); no dependency added here |

## Transition boundaries respected (R-TRANSITION)

| Existing path | Status |
|---|---|
| `game-qa-matrix.csv` and the rest of `qa-inventory` | **unchanged and still generated as today.** No consumer was migrated, nothing was retired. |
| The four `QA_WRITE=1` emitters | **unchanged.** The parallel `observe` call is available (`RunBuilder`) but has deliberately **not** been wired into them: doing so is Q-C's bounded later package, and wiring it here would change what those tests write. |
| `schema/` delete-and-rebuild (E12) | recorded, **not** fixed — explicitly out of scope. |
| CI | **no CI change.** R-NO-CI: `check` has now run against real freshness inputs but against an *empty* evidence tree; a gate is proposed in checkpoint 7, not activated. |

## Unresolved decisions / blockers

| # | Item | Owner |
|---|---|---|
| Q-C wiring | when the four `QA_WRITE=1` emitters gain their parallel `observe` call. Not done here on purpose. | Codex |
| U3 | whether `docs/audits/evidence/` is committed. Until decided, the tree exists nowhere in the repo and every command takes `--root`. | Codex / user |
| `toolchain` identities | `scopes.toml` declares `rustc-version` / `target-triple`; the CLI resolves them from compile-time environment and returns `None` when unavailable, which correctly makes `shared-core` **unknown** rather than inventing a value. Wiring a real toolchain probe is ordinary later work. | developer |

## Prerequisites for the next checkpoint

Checkpoint 4 (S3 migration) needs: the frozen legacy sources (`game-qa-matrix.csv`,
`asset-capability-matrix.csv`, `assets/nhllegacy/xbox360/capability.csv`), an **isolated** candidate
output location, and the `write_manual_record` caller that clears the outstanding clippy lint.

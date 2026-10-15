# C0-BULK1 · Checkpoint 2 — Implement the evidence core (S1)

> **Package:** C0-BULK1, checkpoint 2 of 7. **Date:** 2026-09-07. **Author role:** developer.
> **Status: Passed.** Nothing staged, committed or pushed. No generator run, no fixture read, no
> `docs/audits/data/**` file touched, no capability promoted.

## Status

**Passed.** Contract stage **S1** is implemented and its acceptance cases execute. The core is pure:
no file is read or written by any function in it, no clock is consulted, and no timestamp field is
read by any decision (**R-NO-TIME**).

## Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/mod.rs` | **New.** Module root; layer separation documented. |
| `crates/tdb-qa/src/evidence/key.rs` | **New.** Coverage-key grammar, parse/serialize, conversion component, roll-up scope. 9 unit tests. |
| `crates/tdb-qa/src/evidence/vocab.rs` | **New.** Closed operation vocabulary, operation classes, **R-APPL-OPS** applicability resolution. 6 unit tests. |
| `crates/tdb-qa/src/evidence/model.rs` | **New.** Layer A/B/L types, closed vocabularies, per-key supersession. 3 unit tests. |
| `crates/tdb-qa/src/evidence/validate.rs` | **New.** V1, V2, V5–V8, V12–V15. 8 unit tests. |
| `crates/tdb-qa/src/evidence/merge.rs` | **New.** Selection, four axes, qualification table (incl. row 4a), roll-ups. |
| `crates/tdb-qa/src/evidence/scopes.rs` | **New.** `defs/qa/scopes.toml` schema, validation, freshness digest. 7 unit tests. |
| `crates/tdb-qa/src/evidence/testkit.rs` | **New.** Synthetic fixture builders (public, so unit and integration cases share one model). |
| `crates/tdb-qa/tests/evidence_acceptance.rs` | **New.** 32 acceptance cases. |
| `crates/tdb-qa/src/lib.rs` | Declared `pub mod evidence;` — one line. |
| `crates/tdb-qa/Cargo.toml` | Added `toml = "0.8"` (already in the workspace lock via `tdb-headmesh`). |
| `Cargo.lock` | See **Cargo.lock, recorded** below. |
| `docs/plans/qa-evidence-contract.md` | X16 / **R-CONFLICT-CLASS** (§12.9) — a contradiction the implementation exposed; see below. |

## Implemented behavior

**Coverage keys.** Full §2.2 grammar with the mandatory four-field `conversion` component; syntax and
vocabulary are checked separately so migration can hold a syntactically valid key whose family is not
yet resolved. `rollup_scope()` keeps conversion identities apart (**R-CONV-PAIR**).

**Independent axes.** `applicability` / `implementation` / `automated` / `manual` are stored
separately; `qualification` is derived by the §4.3 table and never stored. The `implementation` axis
is **declared input**, defaulting to `"unknown"` — it is never inferred from evidence.

**Selection.** Per key, not per run. A run is suppressed at a key only by a **completed** run
(R-SUPERSEDE-STATE) and only at explicitly covered keys (R-PARTIAL-KEYS). Disagreement is retained
with all citations and withholds (R-NOWIN).

**Executed failures.** `Outcome::is_executed()` splits executed from skipped/not-attempted;
`executed-unselected` is a distinct axis value; an uncleared executed failure withholds via row 4a
and is cited, whatever its run's terminal state (R-EXECUTED-FAIL).

**Preserved distinctions.** `nhl24` is a conversion source and is absent from the catalog, so it can
never be a coverage target (A22). Pair-specific conversion keys never collapse (A21, A44). The
`save-outer-package` requirement stands on all three console platforms (A10). Layer L structurally
cannot qualify (`LegacyClaim::qualifies()` is `false` by construction).

## A contradiction the implementation exposed — X16

Writing case A6 surfaced a ninth R1 contradiction: §3's decisions table resolves *manual pass +
current automated fail* to **`blocked`**, but §4.3 row 3 withholds on **any** conflict, so that case
could never reach row 4. Resolved by **R-CONFLICT-CLASS** (contract §12.9): a conflict *within* a
layer withholds; a cross-layer conflict whose resolution the contract states is recorded, published
with all citations, and resolved by its own row. Row 3 tests only the first class. Nothing is dropped
from the published `conflict` column either way.

## Exact checks and results

| Check | Command | Result |
|---|---|---|
| Unit tests | `cargo test -p tdb-qa --lib` | **Passed** — 33/33 |
| Acceptance cases | `cargo test -p tdb-qa --test evidence_acceptance` | **Passed** — 32/32 |
| Pre-existing generator regression | `cargo test -p tdb-qa` (incl. `tests/inventory.rs`) | **Passed** — 8/8, unchanged |
| Strict clippy, scoped | `cargo clippy -p tdb-qa --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, scoped | `cargo fmt -p tdb-qa -- --check` | **Passed**, exit 0 |
| Repo-wide fmt / clippy / full suite | — | **Not run** at this checkpoint; deferred to checkpoint 7 |

### Acceptance-case coverage

Contract §11 assigns A1, A3, A5–A10, A13–A24 to S1. Executed here, plus every R2 case:

| Case | Assertion | Result |
|---|---|---|
| A1, A3 | determinism; input order irrelevant | Passed |
| A5 | empty merge qualifies nothing; a present completed run is still selected | Passed |
| A6 | manual pass + current automated fail ⇒ `blocked`, both cited | Passed |
| A7 | stale digest withholds; identical bytes stay fresh (also `scopes.rs`) | Passed |
| A8 | `skipped-missing-fixture` never qualifies | Passed |
| A9 | PS3 NHL 07 `save-raw-db` — claim cited, never reproduced; R2 `not-applicable` | Passed |
| A10 | outer packaging never `not-applicable` on xbox360/ps3/ps4 | Passed |
| A13 / A14 | malformed **active** record is a hard error; malformed **legacy** claim is not | Passed |
| A15 | an operation pass never raises the family roll-up | Passed |
| A16 | interrupted run: never selected, always citable | Passed |
| A17 | two disagreeing completed runs: both cited, `withheld` | Passed |
| A18 / A19 | duplicate ids, cycles, missing targets, partial-without-mode | Passed (`validate.rs`) |
| A20 | two cases on one key coexist | Passed |
| A21 / A22 | pair keys distinct; `nhl24` source-only | Passed |
| A23 / A24 | install pass ≠ game-verified; unknown identity withholds | Passed |
| **A33–A45** | every R2 rule | Passed — 13/13 |

A2, A29, A30 belong to S2 (checkpoint 3); A11, A12, A25–A28, A31, A32 to S3 (checkpoint 4).
A4 (delete a fixture and re-run) needs a real emitter and is deferred to checkpoint 3.

## Evidence / log locations

- `H:\c0-bulk1\logs\cp2-tdb-qa-test.log` — full `cargo test -p tdb-qa` output, exit 0.
- `H:\c0-bulk1\logs\cp2-hashes.txt` — `Cargo.lock` and `crates/tdb-qa/Cargo.toml` SHA-256 after this
  checkpoint.

## Preservation checks

| Check | Result |
|---|---|
| HEAD | `a2bbdc4…` unchanged |
| Index | **empty** |
| Reset / restore / clean / stash / checkout | **none** |
| Existing `tdb-qa` sources (`catalog.rs`, `matrix.rs`, `manifest.rs`, `diff.rs`, `schema.rs`, `bin/qa_inventory.rs`) | **unmodified** — only `lib.rs` gained one `pub mod` line |
| `docs/audits/data/**` | **untouched**; no generator run |
| `defs/**`, `anchors/**` | untouched |
| Files written | the 9 new `evidence`/test files, `lib.rs`, `crates/tdb-qa/Cargo.toml`, `Cargo.lock`, the contract, this receipt |

### Cargo.lock, recorded

`Cargo.lock` changed: **42 insertions, 0 deletions**. Two causes, both recorded rather than hidden:

1. the new `toml` dependency edge for `tdb-qa` (`toml 0.8.2` was already in the lock via
   `tdb-headmesh`, so no new package version was resolved for it);
2. **pre-existing uncommitted `Cargo.toml` work that the lock had not yet caught up with** — the
   `blender-job` member and `zune-core`/`zune-jpeg` entries appear the first time the workspace is
   built. These are not this package's dependencies and were not chosen here.

| Point | SHA-256 |
|---|---|
| Before (pinned by [C0-FX1 §6](../validation/c0-fx1-explicit-fixture-tests.md)) | `d8c6487f6917e92e0c20ee1c836d8df9a5ff9afc1b4f35bf9e664ad14d0b1622` |
| After this checkpoint | `12b777ff811756b16adb0d642e90a3d0ecaeef065ca4b06748406bb08d4bdbc2` |

The change is purely additive; no locked version was altered or removed. C0-FX1's pin is therefore
superseded by this record, not contradicted.

## Unresolved decisions / blockers

| # | Item | Owner |
|---|---|---|
| X16 / R-CONFLICT-CLASS | a new R2 rule written during implementation; needs architect acceptance with the rest of R2 | Codex |
| `implementation` axis source | the contract never said where the `implementation` axis comes from. Implemented as a **declared register**, defaulting to `unknown`. Populating it for real targets is not evidence work and is not done here. | Codex |
| U2, U3 | unchanged from checkpoint 1 | Codex / user |

None blocks checkpoint 3.

## Prerequisites for the next checkpoint

S1 is green, so S2 may proceed (contract §11 gates S2 on S1's invariants holding). Checkpoint 3 needs
an isolated fixture root for publication/recovery testing — it must **not** publish into
`docs/audits/data/`.

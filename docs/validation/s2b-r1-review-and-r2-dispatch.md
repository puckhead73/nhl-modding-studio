# S2-B-R1 review and S2-B-R2 dispatch

Reviewer: Codex, Agent #1 task manager. Existing dirty branch preserved; no
application-source edits or live operations by this review. Index empty at review.

## Decision and independent checks

The original successive-commit and stale-handle reproducers now pass: recovery
retains both commits with supersession recorded, and a handle for an Aborted
transaction is rejected with StaleHandle. The revised adapters and persisted
manifest checks address the preceding architectural findings.

Independent workspace-locked checks, separate review target directory:

- `cargo test --locked --offline -p tdb-txn --target-dir local/reviews/s1-codec-target`:
  25 unit tests and 4 crash tests passed; zero failures; doc tests zero cases.
- `cargo test --locked --offline -p studio-domain --test txn_coordination --target-dir local/reviews/s1-codec-target`:
  3 passed, covering archive re-verification, loaded session identity and coordination.
- Reviewed containment changes and passing junction regressions. The old standalone
  junction scratch tree contains a pre-sequence journal and is now refused as
  unreadable history, so that particular rerun does not independently prove the
  junction correction. Do not report it as such.
- Other developer checks and the real copied-archive demo were not repeated here.

S2-B acceptance remains held for the concrete data-loss reproducer below. Do not
start Workspace/API integration yet. Keep the completed corrections; this is one
bounded mutation-boundary repair, not a transaction redesign.

## R2 — restore can overwrite an external edit made after inspection (P1)

In `engine::revert_all`, destination resolution and `inspect` happen before
`BeforeRevert`, the Reverting journal write, and the backup copy. Then the code
renames the backup over the old resolved path (or removes an originally absent
target) without checking whether it still holds the transaction's replacement.
Copying a large archive makes this more than a theoretical instruction-sized gap.

Independent public-API reproducer in `local/reviews/s2b-probe`:

1. Prepare/apply `a: old -> new` in a fresh scratch copied-target root.
2. Call restore with a Hooks implementation that writes `external edit` to `a`
   when `Event::BeforeRevert` fires.
3. Actual: restore returns `Ok`, state Restored, and `a` contains `old`. The
   foreign bytes were overwritten rather than preserved as a conflict.

Command:

`cargo run --offline --manifest-path local/reviews/s2b-probe/Cargo.toml --target-dir local/reviews/s1-codec-target`

This review-only harness uses its own dependency lock and prints observations;
its zero exit is not a passing regression result. All changed files are disposable
data under `local/reviews/s2b-probe-data-*`.

### Required correction

- Re-establish containment and current destination identity at the actual
  rename/remove boundary, after hooks, journal work and backup preparation. If
  foreign bytes appeared, retain them, retain recovery material and block/report
  the conflict. Do not return Restored/RolledBack for that set.
- Apply the rule to rollback and fresh-process recovery as well as explicit
  restore, and to expected-absent deletion as well as backup replacement.
- Audit `replace_all` too: containment currently precedes BeforeReplace, while
  identity precedes the phase journal write. A parent changed after those checks
  must not redirect the mutation. Existing earlier-phase junction tests are not
  sufficient for this boundary.
- Use supported file/parent identity and exclusion primitives where needed to
  protect the interval through mutation. State any remaining concurrent-writer
  limitation precisely; another early check alone cannot establish an absolute
  race-free or "never overwrites foreign bytes" guarantee.
- Add deterministic regressions for the exact BeforeRevert edit, absent-target
  deletion, edits after backup preparation, and late parent-junction replacement
  at apply/revert boundaries. Keep external data in test scratch directories.
  Assert foreign bytes/outside files remain unchanged and recovery stays blocked
  until the conflict is explicitly resolved. Preserve the passing history,
  stale-handle and crash tests.

## Scope and handoff

Active assignment: **S2-B-R2**. Transaction engine, focused tests and receipt
correction only, with narrow helper changes if required. No Workspace/API/UI,
live installs, staging, commits or pushes. Preserve unrelated work and branch.

Report exact mutation/exclusion ordering, per-reproducer results, checks actually
run and remaining Windows guarantees. Distinguish process interruption, external
concurrency and power-loss durability. The latter remains unqualified; no destructive
power-loss experiment is requested. Stop for Codex review.

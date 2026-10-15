# S2-B review and correction dispatch

Reviewer: Codex, Agent #1 task manager. Existing dirty branch preserved; index empty
at review. No application implementation edits or live installations by this review.

## Decision

S2-B is not accepted yet. Retain its shared engine and full-backup approach, but
correct the findings below before adding Workspace/API consumers. Active package:
**S2-B-R1**. No UI or live write migration is authorized.

Independent baseline command:

`cargo test --locked --offline -p tdb-txn --target-dir local/reviews/s1-codec-target`

Result: 12 unit tests and 4 crash-test harness tests passed, zero failures;
doc tests zero cases. Domain/API tests and real archive demo remain developer-reported
in this review. Passing existing tests does not cover the reproduced failures below.

Review-only external harness: `local/reviews/s2b-probe`. It uses the current path
dependency and its own lockfile; its dependency resolution differs from the workspace
lock. All probe data is under `local/reviews`, including the junction's destination.
The harness prints observed behavior; a zero exit is not an acceptance result.

Reviewed SHA-256:

| File | SHA-256 |
|---|---|
| `tdb-txn/src/engine.rs` | `e0fb27f71af2fb2e12b9514625b6d3698a3472acfde7d64b1a00824bd201b0fd` |
| `tdb-txn/src/journal.rs` | `7b5584bb637b7f0173d9cb36d63b3b8be3b5dae8aa2eedac7045c0ea75a7bdc3` |
| `tdb-txn/src/changeset.rs` | `93191899afe61f101ac1188ae64dc50dba7722a8b9a8cc24ca7263e5f0c87220` |
| `studio-domain/src/txn.rs` | `771987182cee1ed8d4d0fac39f9de32531c45dc931e5d9382ee00a6507bf4086` |

## R1 — enforce filesystem containment, not just relative spelling (P1)

`engine.rs:247` validates components, then returns `root.join(destination)`.
It does not resolve/check destination parents for junctions or other reparse points.
Likewise recovery paths are checked lexically. The recovery volume check compares
joined path prefixes rather than proving the actual resolved destinations.

Reproduced on Windows: create a marked copy root, then a directory junction
`copy/escape -> outside` (both scratch directories). Prepare/apply for `escape/a`
commits successfully and changes `outside/a` from `old` to `new`. Thus the marker
does not constrain writes to the copied target as claimed.

Correct destination and recovery resolution for prepare/apply/recover/restore,
including the lock, marker, journal and temporary-file paths. A conservative refusal
of reparse points in writable paths is acceptable for this bounded package. Check
again at the mutation boundary; do not rely solely on prepare-time resolution.
Refuse unsupported cross-volume layouts based on resolved filesystem identity.

Reserve engine-owned namespaces (`.tdb-txn`, copied-target marker) from artifact
destinations. Validate transaction IDs before joining them to a path. Handle Windows
alternate-stream/device spellings explicitly. Domain staging paths must not be
derived unchecked from artifact keys (`../...`, separators or absolute keys), and
must not overwrite arbitrary existing files.

Regressions: destination-parent junction escape; recovery-directory junction escape;
reserved destinations; escaping transaction IDs and staging keys. Assert outside
bytes remain unchanged. Use scratch roots only; no live-tree experiments.

## R2 — recognize legitimate transaction history (P1)

`recover` (`engine.rs:993`) verifies *every* historical Committed journal against
the current destinations. It does not account for later transactions superseding
the same destination.

Reproduced through public APIs: commit `a: old -> one`, then commit
`a: one -> two`, then call recover. The first transaction becomes Blocked; the
second remains Committed, and ACTIVE blocks the root. No external edit occurred.

Define persisted per-destination lineage/ownership, or another explicit supported
history policy, so routine successive edits remain recoverable. Recovery must
distinguish a later authorized transaction from foreign bytes. Define restore
ordering: refusing out-of-order restore without poisoning valid history is fine;
silently overwriting a newer transaction is not. Cover overlapping and disjoint
sets, a crash during the later apply, later rollback, reverse-order restore,
repeated recovery, and actual foreign edits. Preserve prior backups.

## R3 — revalidate handles and authoritative journal state under lock (P1)

`apply` (`engine.rs:554`) trusts the caller's public, cloneable Transaction and its
in-memory Prepared state. It does not reload authoritative state, check ACTIVE
ownership, or validate/bind the supplied change set to the recorded change-set ID.

Reproduced: prepare -> recover marks that transaction Aborted -> apply using the
original handle. Apply succeeds and changes the file, resurrecting an aborted set.
A root lock only serializes calls; it does not make stale handles authoritative.

Reload and validate persisted state and change-set identity while holding the lock.
Reject stale/mutated handles and mismatched change sets before any staging or writes.
Restore must respect another active/blocked transaction rather than replacing its
ACTIVE marker. Keep invalid calls from clearing another transaction's marker.

Validate journal semantic invariants as well as JSON/schema/path syntax. Inspection
also finds that Present + backup=None passes Journal::read but reaches an
`unreachable!` in revert_all; missing change-set keys can reach an `expect` in
stage_all. Corrupt records must produce a retained, actionable error, never panic.
Bind recoverable records to the persisted reviewed manifest; check duplicate keys,
destinations, required backup identities and legal states/phases.

Regressions: the stale-handle sequence; a second transaction prepared before stale
apply; a changed change set; restore while another set is active; valid JSON with
missing backup/duplicate records. Assert zero unintended target changes and correct
ACTIVE ownership.

## R4 — complete artifact validation and original-source binding (P1)

Two adapter issues found by source inspection, not independently executed here:

- `archive_artifact` (`studio-domain/src/txn.rs:100`) accepts a mutable verification
  report when its output hash matches and entries_reparsed is merely nonzero. It
  does not establish full structural verification or bind the report to the actual
  plan. An invented report for arbitrary bytes can satisfy these conditions. This
  does not meet the dispatch's requirement to validate reports as input, rather
  than treating cached counters as proof. Re-run the actual bound verification at
  the adapter boundary, or introduce a controlled verified-artifact construction
  that callers cannot fabricate/mutate and which binds source, plan and output.
- `session_artifact` (`txn.rs:163`) hashes the destination *at staging time* to
  establish expected_prior. If it changed after the session was opened, stale
  session bytes can then overwrite that change without a conflict. Carry the
  original session/source identity into the artifact and compare it with the
  copied target; do not silently adopt newer disk bytes as the expected baseline.

Add regressions for forged/inconsistent archive reports, changed output, and a
destination edited after session opening but before artifact staging. Preserve the
successful coordinated archive/session/companion demonstration. Keep qualification
unresolved, and scope qualification notes to this artifact/evidence; avoid blanket
claims about all titles' runtime history.

## Durability qualification remains open

The receipt correctly distinguishes process interruption from power loss. Retain
that distinction. Current Windows rename-metadata durability assumptions do not
close the dispatch's durable-install gate; this review neither requires a destructive
power-loss experiment nor waives the unresolved ordering/platform guarantee.
Report the supported guarantee explicitly after correctness repairs. The next API
integration remains parked until review; no claim of live eligibility is authorized.

## Handoff

Complete R1-R4 within the existing transaction/domain boundary and focused tests.
Preserve unrelated work, branch, source archives and receipts' evidence attribution.
No staging, commits, pushes, UI, new live commands, broad discovery or format work.

Return per-finding outcomes, reproduction/regression results, actual checks,
transaction-history and restore policy, containment policy, and remaining durability
limitations. Stop for Codex review. Do not start the proposed Workspace/API package.

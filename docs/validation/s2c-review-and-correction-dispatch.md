# S2-C review and correction dispatch

Reviewer: Codex, Agent #1 task manager. Existing branch and unrelated dirty work
preserved. Index empty at review. No application implementation changes or live
operations performed by this review.

## Decision

S2-C needs the bounded corrections below before acceptance. Keep the shared
transaction integration; do not begin Texture Lab UI. Active package: **S2-C-R1**.

Independent command:

`cargo test --locked --offline -p studio-api --test copied_target --target-dir local/reviews/s1-codec-target`

Result: 5 passed, zero failures, including the interruption child case. Other
developer checks (full API/domain suites, exporter, shell and TypeScript checks)
were not repeated in this review.

Independent public-API harness: `local/reviews/s2c-probe`, run with
`cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target`.
Its separate lockfile was seeded from the workspace lock. All test targets are
new disposable copies under `local/reviews/s2c-probe-data-*`; no originals changed.
The harness prints observations, so a zero exit is not a passing acceptance result.

Reviewed SHA-256:

- `studio-api/src/registry/copied_target.rs`:
  `30eb6af81678430a9c43914315e00a4ad4ea18d310d71f419b233b938d674477`
- `studio-domain/src/copied_target.rs`:
  `9ac2feff6e90fb0e34d22730864bf8ce062d7f74948c1311d4f6c7d8b8445766`

## R1 — bind existing headers as reviewed dependencies (P1)

`stage_workspace`, `copied_target.rs:219`, combines Present with NotIndexed and
discards the existing header path. Only templates for newly generated headers
become bound inputs. Consequently an existing header is not rechecked at apply.

Reproduced: provision the indexed copied-tree fixture with its header already
present; edit the workspace; review reports zero bound inputs; delete the header;
apply returns Committed and the header is still absent. The prepared save no longer
has the companion that justified omitting a header artifact.

Required: record a contained, identity-bound dependency for Present headers and
check it in apply preflight. Deletion, modification, unreadability or redirect
invalidates the review before target mutation. Preserve the existing file exactly;
do not silently regenerate it after review or commit. Include the dependency in
review DTOs. Do not treat directories or redirected outside-root headers as valid
regular companion files. Add delete/change and unchanged-header regressions.

## R2 — reserve affected files, not only the initiating project ID (P1)

`copied_target_apply_with_hooks` marks only `busy.insert(project_id)` at line 633.
Another open workspace can refer to the same primary/roster and still reach
save_workspace through its own unblocked project ID.

Reproduced: open the same copied files twice in one Registry; review/apply through
the first project; at AfterReplaced(primary), invoke save_workspace on the second.
That competing save returns Ok(2), writing both files during the transaction. In
this run it also creates the pending header and causes the transaction to fail and
roll back. Such a write must be refused by the API reservation, not left for the
engine to discover after the competing writer has already changed files.

There is also a same-project race: preflight runs through with_workspace, releases
the mutex, then a separate lock inserts busy. An edit can occur between revision
validation and the reservation.

Required:

- Reserve canonical affected destination identities/root and every aliasing open
  workspace for the operation. Establish preflight validation and reservation in
  one synchronized transition; no edit window between them. Use a guard for cleanup.
- Block conflicting operations through another project ID, including legacy saves,
  new opens targeting reserved files, and close/reload while its state is needed.
  This is a narrow shared reservation check, not migration of live writers into
  the transaction engine. Preserve unrelated-project availability.
- Apply the same rule to recovery/restore and overlapping roots; a snapshot of
  currently open projects must not allow a newly opened alias to bypass the guard.
- On completion, rebind the initiating workspace and refresh/invalidate affected
  aliases appropriately; preserve their unsaved edits. Release all reservations on
  success, refusal, cancellation and error. Status must identify affected busy work.
- Regressions: the exact alias-save probe; an alias edit/open during apply; competing
  same-project edit at preflight/reservation boundary; recovery/restore contention;
  unrelated-project operations still work. Require refusal before any competing write.

## R3 — surface refresh failures and retain a safe session state (P2)

Source inspection: refresh_and_release uses `ws.refresh_baselines().ok()` at line
779. baseline_dto maps None to `unchanged`. A missing/unreadable/malformed file or
failed alignment analysis is therefore hidden as a successful unchanged refresh.
Workspace refresh/reload also mutates the primary before attempting the roster,
so an error can follow a partial refresh.

Required: return an explicit per-project refresh failure/action, retain edits and
mark the workspace unusable for another copied save until resolved. Never report
unchanged when refresh failed. Prepare refresh/reload changes before adopting them
as a coordinated workspace, or explicitly preserve/report partial outcomes and
block further edits that rely on invalid alignment/history. Avoid silently dropping
the error while releasing busy state. Add missing/corrupt roster and primary refresh
regressions, including preservation of dirty edits and retry/reload behavior.

## Decisions accepted within the current scope

- Content-based revision is appropriate: undo to identical content may reuse review.
- Busy may conservatively block reads of affected workspaces; unrelated work must
  remain available. Scope the reservation to actual conflicts.
- OS-temp staging, server-owned handles, explicit re-review supersession and the
  internal archive extension seam are reasonable for this package.
- The six DTO export roots/count update are expected, subject to drift checks.
- No root-provisioning UI, PS4 expansion or live eligibility is required here.

## Handoff

Complete R1-R3 in domain/API planning, reservations/session lifecycle and focused
tests; update the receipt to match tested behavior. Generated DTO changes through
the exporter are authorized if needed for refresh errors. Run the affected tests
and binding drift check. Do not repeat the full corpus or expand format support.

Return per-finding repro/regression results, reservation scope/order, failure-state
DTO examples, checks actually run and remaining blockers. Preserve branch and
unrelated work. No staging, commits, pushes, frontend UI or live installation.
Stop for Codex review; Texture Lab UI remains next after this boundary is accepted.

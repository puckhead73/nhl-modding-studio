# S2-B-R2 acceptance and S2-C dispatch

Reviewer: Codex, Agent #1 task manager. Existing dirty branch preserved; index
empty at review. No commits or live installations authorized.

## Decision

S2-B-R2 passes bounded review for the **isolated copied-target transaction
milestone**. The engine can now support the next copied-target domain/API slice.
This does not close C3/Q06 production durability or authorize live writes.

Independent checks:

- `cargo test --locked --offline -p tdb-txn --target-dir local/reviews/s1-codec-target`:
  32 unit tests, 4 crash tests passed; doc tests zero cases.
- Rebuilt and ran `local/reviews/s2b-probe`: successive commits remain Committed
  with supersession recorded; stale apply is refused; the exact BeforeRevert
  foreign-edit case now returns Blocked and retains `external edit` byte-for-byte.
  The review-only probe has its own lockfile; workspace tests use the workspace lock.
- Domain coordination's 3 tests passed independently in the preceding R1 review;
  the R2 developer's domain/API checks and full-archive demo were not repeated here.

Reviewed `engine.rs` SHA-256:
`640237b3243cb85849b169764f35b39e67b3a47cbdaf1f3dddc9bebf74d54f58`.

The final path/hash check narrows, but does not eliminate, the concurrent-writer
race. A Windows handle that omits FILE_SHARE_DELETE prevents replacement while
held; it does not protect against all writers or a closed handle followed by our
rename. Use dedicated disposable roots with no game/external editor accessing them.
Keep external-concurrency and power-loss qualification open before live integration.
Scope claims of process recovery to the tested protocol and these assumptions;
avoid unconditional "never overwrites unowned bytes" claims across failure classes.

## Active assignment: S2-C — copied-target Workspace and API integration

Objective: expose the shared review/prepare/apply/recover/restore workflow to a
real Workspace consumer, including content-header companions, without changing
existing live save/install behavior. One prepared change-set identity must survive
preview, apply, recovery and restore. No Project prerequisite for Standard use.

### Scope

- `studio-domain`: Workspace staging, pure content-header planning, transaction
  adapters, focused tests/examples.
- `studio-api`: narrow copied-target registry/command surface, typed DTOs/conversion,
  command registration, API tests. Generated bindings for these DTOs are now
  authorized through the existing exporter; check drift, do not hand-edit them.
- Narrow transaction helper fixes justified by this consumer and one compact receipt.
- Codex owns this dispatch. No frontend pages, Texture Lab UI, live writer migration,
  registry-schema expansion, LZX, converter work, broad discovery, staging, commits
  or pushes. Preserve the existing branch and unrelated dirty work.

### Required behavior

1. Add a Workspace staging sibling to save_in_place that packages the primary and
   attached roster sessions through the existing serializers/reparse checks.
   Destinations must be in an explicitly selected, marked copied-target root and
   match each session's loaded baseline. Preserve all dirty edits on refusal/failure.
2. Refactor content_header discovery/byte production into a pure planner shared with
   the existing ensure helper where practical. Before prepare, include every newly
   required header in the same change set, with its expected absence and dependency.
   Preserve existing headers byte-for-byte. If relying on an existing header/template,
   bind that input and invalidate preparation if it changes. No sidecar writes or
   discovery after commit. Keep existing non-header-indexed-tree behavior.
3. A header path outside the selected copy root is a refusal, never an implicit
   root expansion. Refuse missing required parent directories with a useful message
   in this package; directory-creation transactions are not required. Tests/demo
   can provision the copied-tree directory structure explicitly beforehand.
4. Expose review/prepare and separate apply, plus status/recover/restore, using a
   server-owned prepared handle/ID. Review reports the actual destinations, prior
   and output identities, dependencies, qualification and recovery state. The API
   must not accept frontend-supplied validation booleans, arbitrary manifests,
   mutable Transaction objects or unchecked destination overrides as authority.
5. Bind prepared work to the selected root and the Workspace revision. Edits or
   target changes after preview require re-prepare. Reject a handle from another
   workspace/root, duplicate apply, stale handles and altered artifacts. Keep the
   engine's authoritative journal checks. Prevent concurrent Studio operations
   from independently mutating affected sessions/targets during apply or recovery.
6. Recovery inspection is a prerequisite to another copied-target apply. Return
   actionable Blocked status and retained recovery paths without deleting evidence
   or retrying into foreign bytes. Persisted transaction IDs remain inspectable
   after registry/process restart; reconstruction must not depend on old handles.
7. Define post-commit session lifecycle explicitly: after a successful save the
   next edit/save must use the newly installed baseline. Reopen/rebind on success
   or require an explicit reload through the API; never silently reuse the original
   loaded hash or clear dirty state on failure. Restore/recovery must invalidate or
   refresh affected sessions and pending reviews. Do not discard unsaved edits.
8. Use the same prepared-artifact service for archive artifacts from the controlled
   StagedArchive adapter; no separate archive installation command or alternate
   filesystem writer. A focused domain/API test can demonstrate a combined set
   without building an archive browser in this package.
9. Carry unqualified status and the copied-target restriction through DTOs/results.
   There is no force-live option. Keep progress/cancellation and failures accessible
   without holding unrelated global registry locks through long archive operations.

### Acceptance

- Real Workspace-level copied-target demo/test: primary plus roster where applicable,
  missing header planned before apply, prepare/review -> apply -> reopen -> exact
  restore. Use known valid fixtures; no invented game semantics.
- Existing header preserved; ordinary non-indexed save needs no header; out-of-root
  sidecar refused; missing required parent reported before target mutation.
- Two consecutive edit/save cycles through the copied API, then ordered restore,
  proving loaded-baseline/session lifecycle and transaction ownership work together.
- Session edit after preview, external file/header change, stale/cross-root handle,
  concurrent request, missing staged output and cancellation all refuse or recover
  without losing user edits or writing unexpected paths.
- A restarted registry discovers/reports an interrupted transaction through the
  same recovery API; blocked recovery prevents another apply. Reuse engine crash
  machinery rather than duplicating the full crash matrix.
- Focused tests demonstrate archives and DB/save/header artifacts use the same
  prepared service. Run affected domain/API tests, transaction regressions and DTO
  drift checks; frontend type/build checks only as relevant to generated contracts.

## Handoff

Return runnable commands and outcomes, API contract/review examples, session
revision/baseline lifecycle, companion planning rules, actual checks and remaining
blockers. Stop for Codex review. Texture Lab UI is the next integration candidate
after this shared boundary is accepted; it is not authorized in this package.

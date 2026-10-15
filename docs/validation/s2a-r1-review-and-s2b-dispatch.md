# S2-A-R1 acceptance and S2-B dispatch

Reviewer: Codex, Agent #1 task manager. Branch remains `feat/phase5-asset-tooling`
with existing uncommitted work. No commit or live-install authorization.

## Acceptance

S2-A-R1 passes this bounded review. S2-A is accepted as a structurally validated
staging building block, not as game-qualified output or completed C3/C3a.

Independent checks:

- `cargo test --locked --offline -p tdb-archive-codec -p tdb-archive-big --target-dir local/reviews/s1-codec-target`:
  archive 52 passed / 1 ignored; codec 45 passed / 6 ignored; doc tests passed
  with zero cases. Ignored cases are not passes.
- The independent `local/reviews/s2a-probe` reproducer now returns
  `TablesOverlap { file_table_end: 64, name_table_offset: 32 }` without panic.
- Reviewed shared table validation, bind/verify use, and qualification propagation.
  The planner always carries unresolved native-loader acceptance; staging does
  not promote qualification.
- Domain/canonical checks, clippy, formatting and the new full-archive demo remain
  developer-reported for this round. Private codec fixtures were independently
  executed in the preceding review; not repeated for this parser-only correction.
- Index empty at review.

Reviewed SHA-256:

- `stage.rs`: `268cbafeee60ae5891f6a3a84e2e06c5c6625eb44c75f525957e462217ae8ae7`
- `eb.rs`: `8fc41cbc4f5b141cff3b0b6be30eee1e3070070782e16f4567cc36a1905f8c37`

The arithmetic-overflow branch need not be forced through an impossible serialized
header to close this package. Keep its defensive checks. No additional S2-A
paperwork or broad corpus rerun is required.

## Active assignment: S2-B — shared copied-target transaction and recovery

Objective: one reusable transaction engine coordinating validated archive and
DB/save artifacts, demonstrated on explicitly created disposable copied targets.
Implement prepare/review, apply, restart recovery and verified restore. No Project
prerequisite. No application command may start writing live targets in this package.

This is a bounded foundation for C3/Q06, not migration of every existing writer.

### Existing boundaries to reuse

- `tdb-archive-big::stage`: source/output hashes, entry/range edits, structural
  verification and unresolved qualification.
- `EditSession::save`: existing DB/save serialization and container packaging.
  `EditSession::save_to` and `Workspace::save_in_place` currently write directly;
  the latter also creates content-header sidecars separately.
- `studio-api/src/hockey_shop.rs::atomic_write_with_backup` currently uses
  `std::fs::write`; its name is not an atomicity or durability guarantee.
- `tdb-headmesh::install` is a pure planning boundary, not a filesystem journal.

Use these facts to avoid another independent asset-only swap path. Do not broadly
rewrite existing callers now. Demonstrate the new domain entry point directly.

### Write ownership

Agent #1 owns the implementation. Add the shared module in `studio-domain`, or a
small dependency-neutral filesystem crate if dependency direction justifies it.
Narrow adapters in existing archive/session/workspace code, focused tests, a copied
target demo and a compact receipt are authorized. Necessary dependency/lock changes
are authorized. Keep archive codecs independent of domain orchestration.

No UI, generated DTOs, registry-schema changes, LZX, head conversion, format discovery,
evidence activation, live command migration, staging, commits or pushes. Preserve
the assigned branch and unrelated dirty work. Codex owns this dispatch record.

### Required behavior

1. One versioned change-set identity binds every destination, expected prior
   identity (including expected absence), staged output identity, validation result,
   qualification and dependencies. Resolve duplicate/conflicting destinations
   before apply. Coalesce same-archive edits through S2-A. Keep companion writes
   in the same change set; do not discover and write a sidecar after commit.
2. Prepare all outputs and validate all inputs before target mutation. Recheck
   source/destination and staged-output hashes at apply. Persist review identity;
   changed artifacts invalidate the prepared operation. Treat public/mutable S2-A
   reports as input to validate, not an unforgeable proof or cached boolean.
3. Default to full intact backups plus validated replacement files in the
   destination filesystem. Do not add in-place range patching in this package.
   Stream archive copies/hashes. Full backups preserve fit-in-slot pre-images and
   original EOF; a range list or overwritten source does not.
4. Persist a versioned journal with enough identities, paths and state to recover
   without in-memory Plan values. Specify the durable ordering of backups, staged
   files, journal transitions, destination replacement, verification and commit.
   Never overwrite a prior transaction's backup. Validate journal paths on recovery
   and keep writes inside the explicit copied-target/recovery roots.
5. Implement replacement and flush behavior for Windows, the primary host. Do not
   label remove-then-rename atomic or equate flush with durability. Refuse unsupported
   filesystem/volume arrangements explicitly. Account for staging/backup space,
   propagate actual disk-full/flush/rename errors, and preserve recovery material.
6. Define one deterministic recovery policy. A concrete acceptable policy is:
   without a durable committed record, recover all targets to their pre-images;
   after committed, verify the new set and retain backups for explicit restore.
   A crash between replacement and journal update must be recoverable by inspecting
   old/new hashes. Unexpected third-party bytes are a conflict: stop without
   overwriting them and retain the journal/backups.
7. Multi-file replacement is not instantaneously atomic. Block competing Studio
   operations on the affected targets and leave an interrupted set marked as
   requiring recovery. Report success only when the entire set is verified and
   committed. Startup/reopen recovery must resolve or explicitly block the set
   before another apply is allowed. Never claim filesystem-wide atomic visibility.
8. Restore is itself restart-recoverable and verifies exact original hashes and
   lengths. For a target originally absent, remove it only when it still matches
   the installed artifact. Refuse to clobber later external edits. Recovery and
   restore must be idempotent on repeated invocation.
9. Support cancellation before the mutation/commit phase and progress/errors that
   identify the failing target and retained recovery directory. Once mutation has
   begun, cancellation must reach a verified consistent outcome through the journal
   policy; it cannot just abandon a partially applied set.
10. Explicit copied-target mode permits structurally valid Unqualified S2-A
    artifacts and records that fact. Constrain this to a newly created, explicitly
    selected test-copy root; do not add an unrestricted force flag to live paths.
    Reject structural blockers. Preserve qualification items without resolving them.
    No live-install operation or qualification-evidence system is required here.

### Observable acceptance

- A runnable copied-tree demonstration replaces one EB archive with both fit and
  append changes, then reopens/verifies and restores exact original hash and EOF.
- A coordinated change set also includes a DB/save artifact serialized and reparsed
  through existing code (a synthetic valid fixture is sufficient), and an
  originally absent companion test file. They use the same journal and engine.
  No new asset-ID semantics are needed merely to demonstrate coordination.
- Exercise source/output tampering, duplicate/aliased destinations, conflicting
  plans, locked files, insufficient space/write/flush/rename failures, cancellation,
  corrupt/truncated journals, missing/corrupt backups and external edits before
  apply/recovery/restore. Test refusal of escaping recovery paths.
- Inject interruption around every durable transition, including immediately
  before/after each destination replacement and the committed record. At least
  one fresh-process recovery test must reconstruct state using only on-disk files;
  tests returning an error through normal cleanup are not crash simulation.
- Demonstrate recovery for a failure after the first target changed but before
  the second, plus interrupted restore and repeated recovery. Require exact hashes
  for all old/new targets as appropriate, not only successful parsing.
- Run focused transaction tests and affected archive/domain consumers. Report
  Windows behavior actually tested and distinguish process-interruption simulation
  from real power-loss or hardware durability qualification.

If a platform durability primitive cannot be implemented or verified, isolate that
specific blocker and complete independent engine/test work. Do not silently weaken
the protocol or claim durable completion.

## Handoff and next boundary

Return the implementation/interface, journal state transitions and Windows
durability assumptions; demonstration commands/results; files changed; actual checks;
failure/recovery coverage; remaining blockers; and the smallest next integration.
Stop for Codex review. Do not begin Texture Lab UI or migrate live save/install
commands automatically. Those are subsequent packages after transaction acceptance.

S2-A's unknown header meanings, native encoding acceptance, PS4 20-byte rows and
runtime precedence remain explicitly unresolved; none blocks copied-target recovery
engineering or is resolved by its success.

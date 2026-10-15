# S2-C — copied-target Workspace and API integration

> **Package:** S2-C of [s2b-r2-review-and-s2c-dispatch](../validation/s2b-r2-review-and-s2c-dispatch.md),
> corrected by **S2-C-R1** ([s2c-review-and-correction-dispatch](../validation/s2c-review-and-correction-dispatch.md)) — section 9.
> **Date:** 2026-09-17. **Author role:** developer. Reviewer: Codex (architect).
> **Branch:** `feat/phase5-asset-tooling` at `8d60a6d`; the pre-existing dirty working tree is preserved.
> **Nothing staged, committed or pushed. No live installation.** No frontend pages, no Texture Lab UI,
> no migration of `save_workspace` / `save_project` / `hockey_shop_install_apply`, no registry-schema
> expansion, no LZX, no converter work. Generated bindings were installed through the existing
> exporter (`--write`), never by hand.

---

## 1. Result in one paragraph

A real [`Workspace`] (primary `nhlng.db` plus the attached roster save) can now be saved into an
explicitly selected, marked **copied-target root** through the shared `tdb-txn` engine: the
sessions are packaged through the existing serializers and re-parse checks, every content-header
sidecar the copied tree newly requires is planned by a **pure planner** (refactored out of
`ensure_content_header`, which now consumes it) and joins the **same change set** — expected
absent, depending on the save it indexes, with any template it was cloned from recorded as a
**bound input** — and the `studio-api` registry exposes *review/prepare → apply* behind a
**server-owned handle**, plus *status / recover / restore / discard / cancel / reload*. The handle is
bound to `(project, canonical root, workspace content revision)`; an edit after review, a changed
destination, header, template or staged output, a missing parent directory, a stale, duplicate or
foreign handle, and a busy or blocked root are all refused **without touching a destination and
without losing an edit** (the engine's prepared set is aborted so the root is free again). On
commit the sessions **rebind** to the installed files (verified by re-reading them) so the next
edit/save uses the new baseline; restore and recovery **refresh** every open workspace under the
root (clean → reloaded, dirty → edits kept and marked stale until an explicit reload). A fresh
process discovers an interrupted apply through the same status/recovery API and a blocked root
refuses another save. An archive staged through the controlled S2-A adapter joins the same
prepared service (no separate archive installer). Every artifact stays **unqualified**, the mode is
copied-target only, and there is no force/live option.

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-txn/src/engine.rs`, `lib.rs` | **Narrow helpers:** `copied_target_root(path)` (the engine's own root check, exported so the consumer's containment agrees with the engine's) and `abort(&Transaction, hooks)` (Prepared/Staged → Aborted under the lock with the handle checked against the journal; frees the `ACTIVE` marker; retains the recovery dir). |
| `crates/tdb-txn/src/tests_r2.rs` | `abort_discards_a_prepared_set_and_refuses_everything_else` (stale handle, idempotence not offered, committed set refused, root free afterwards). |
| `crates/studio-domain/src/content_header.rs` | `plan_content_header(save_file) -> HeaderPlan { NotIndexed \| Present(path) \| Required { header_path, bytes, template: Option<TemplateInput{path, identity}> } }` — read-only; `ensure_content_header` is now plan + write. Non-indexed-tree behaviour unchanged (existing 3 tests + 2 unit tests pass unmodified). |
| `crates/studio-domain/src/copied_target.rs` (new) | `stage_workspace(ws, root, staging_dir, title) -> CopiedTargetPlan`, `verify_plan_inputs(&plan)`, `CopiedTargetPlan::add_archive(key, dest_path, &StagedArchive)`, `ArtifactRole`, `PlannedArtifact`, `BoundInput`, `KEY_PRIMARY`/`KEY_ROSTER`. |
| `crates/studio-domain/src/project.rs` | `Project::rebind_to_file(file_bytes, identity)` (crate-private; re-detects source, replaces the inner baseline, container template and loaded identity). |
| `crates/studio-domain/src/session.rs` | `EditSession::rebind_saved_baseline(path, &identity)` (re-reads, verifies identity **and** that the inner TDB equals the working bytes before adopting), `reload_from_disk(path)`, `baseline_matches_disk(path)`. |
| `crates/studio-domain/src/workspace.rs` | `revision()` → `WorkspaceRevision` (content identities of every session), `roster_session()`, `roster_path()`, `stale_baseline()`, `rebind_after_copied_save(primary, roster)`, `refresh_baselines() -> BaselineRefresh`, `reload_from_disk()`; `BaselineAction`, `BaselineRefresh`. `save_in_place` unchanged. |
| `crates/studio-domain/src/lib.rs` | exports above; `pub use tdb_txn;` so `studio-api` reaches engine types through the domain crate. |
| `crates/studio-domain/tests/copied_target.rs` (new) | 4 tests (section 6). |
| `crates/studio-api/src/registry.rs` | `Inner.reservations` (files + every workspace on them held during engine work — R1 §9.2; `with_workspace`, `register`, `close_project` and `save` consult it), `Registry.copied: Mutex<CopiedState>`, child module. |
| `crates/studio-api/src/registry/copied_target.rs` (new) | the service (section 3). |
| `crates/studio-api/src/dto.rs`, `bindings.rs` | 14 DTOs, 6 export roots (`CopiedTargetReviewDto`, `CopiedTargetApplyDto`, `CopiedTargetStatusDto`, `CopiedTargetRestoreDto`, `CopiedTargetRecoveryDto`, `ProjectBaselineDto`); the deliberate root-count guard updated 70 → 76. |
| `crates/studio-api/src/error.rs` | `ApiError::CopiedTarget(String)`, `ApiError::Transaction(TxnError)`. |
| `crates/studio-api/src/commands.rs`, `lib.rs` | 8 commands registered in `command_handler!` (section 3). Existing commands untouched. |
| `crates/studio-api/Cargo.toml` | dev-deps `tdb-archive-big`, `tdb-archive-codec` (the combined archive test only). |
| `crates/studio-api/tests/copied_target.rs` (new) | 5 tests incl. a fresh-process interruption child (section 6). |
| `app/src/bindings/*.ts` (+14, generated) | installed by `export-bindings -- --write`; `--check` reports in sync; the `live_bindings_match_the_dto_contract` gate passes. |

## 3. API contract

```text
copied_target_review (project_id, root)            -> CopiedTargetReviewDto   prepare; mints a handle
copied_target_apply  (project_id, handle)          -> CopiedTargetApplyDto    consumes the handle
copied_target_cancel (handle)                      -> bool                    prepared or in flight
copied_target_discard(handle)                      -> CopiedTargetStatusDto   aborts the prepared set
copied_target_status (root)                        -> CopiedTargetStatusDto   from journals; no handle needed
copied_target_recover(root)                        -> CopiedTargetRecoveryDto engine recovery + session refresh
copied_target_restore(root, txn_id)                -> CopiedTargetRestoreDto  exact originals + session refresh
copied_target_reload (project_id, discard_edits)   -> ProjectBaselineDto      explicit; refuses dirty unless told
```

**Authority.** The frontend supplies a project id, a root path, a handle and (for restore) a
transaction id — nothing else. Destinations, prior/output identities, validation, qualification,
dependencies and bound inputs come from the sessions, the header planner and the S2-B adapters;
the engine re-validates the persisted change set and journal under its own lock at apply. There is
no manifest, validation boolean, destination override, force or live option in any DTO.
`live_install_allowed` is always `false`; `mode` is always `copied-target`.

**Review example** (nhl14 pair in the header-indexed fixture layout, header absent):

```text
handle 0  txn 000001-<millis>-<changeset prefix>  root H:\…\copy  revision <16 hex>+<16 hex>
mode copied-target  qualification unqualified  live_install_allowed=false
  primary        database        Main database (nhlng.db)
                 USRDIR/nhlng.db   prior 2 820 656 / 84e1bdff…  output <same length> / <new sha256>
                 studio-domain::session  unqualified: "native acceptance of this saved database/save … unresolved"
  roster         roster-save     Roster save (ROSTER 20260723000649)
                 B13EBABEBABEBABE/454109EC/00000001/ROSTER 20260723000649/ROSTER 20260723000649
                 prior 2 215 156 / e54e9621…  output 2 215 156 / <new sha256>
  roster.header  content-header  Save index header for ROSTER 20260723000649 (B13EBABEBABEBABE/454109EC/Headers/00000001/ROSTER 20260723000649.header)
                 prior: absent  output 328 / <sha256>  depends on [roster]
                 studio-domain::content_header  unqualified: "synthesized from the verified 0x148-byte layout … not game-verified"
  bound_inputs   []      (a cloned template appears here with its path + identity)
  status         pending_handles [0]  apply_allowed=false  guidance "review handle 0 is prepared for transaction …; apply it or discard it before starting another"
```

(The prior lengths/hashes are the committed nhl14 anchors; the output hashes depend on the edit and
are asserted by the tests rather than transcribed here.)

**Apply result:** `state committed`, `recovery_dir <root>/.tdb-txn/<txn>`, per-destination
identities, `applied_unqualified true`, `project { primary: rebound, roster: rebound, dirty: false }`,
the engine event log, and the root status (`apply_allowed true`, the transaction `restorable`).

**Status guidance** is one sentence naming the next action: a pending handle to apply/discard, an
interrupted set to run recovery on, a blocked set to resolve by hand (recovery material retained,
nothing deleted), or "the root accepts a new copied-target review".

**Refusals** (all `Err`, all leave the session's edits and every destination untouched; the
engine's prepared set is aborted so the root is free): edited after review (content revision);
stale baseline; destination not the loaded identity; planned-absent header now present; bound
template changed; staged output missing or changed; parent directory missing (named); unknown /
already-used / discarded / superseded handle; handle of another project; project busy (apply,
recovery or restore in progress); root busy/blocked/interrupted (with the guidance text); prepared
handle on the root when restoring (discard first); dirty reload without consent.

**Locks.** `inner` is held only for packaging, the preflight + reservation and the rebind/refresh;
the engine runs under a **reservation** (§9.2) of the affected files and every workspace on them,
so other projects, status and cancel remain available while the engine runs. A reserved project
refuses every `with_workspace` access (reads included) — a deliberate, conservative choice.

## 4. Session lifecycle

* **Review** packages the sessions (`EditSession::save` → `Project::reparse_packaged` → bound to
  the loaded identity) and records the workspace **content revision** (identity of each session's
  working bytes). Nothing in the session changes; dirty stays dirty.
* **Apply preflight** requires the same revision (so an edit-then-undo back to identical bytes is
  still valid, any real change is not), no stale baseline, and `verify_plan_inputs` (priors, absent
  headers, parents, bound inputs, staged outputs) — then locks the project.
* **Commit → rebind.** Each session re-reads its destination, verifies the engine's installed
  identity, re-detects the source (a repacked container becomes the template for future repacks),
  requires the inner TDB to equal its working bytes, and adopts it as the baseline: `is_dirty()`
  is false, `loaded_identity` is the installed identity, the next review's expected prior is that
  identity, the undo stack is kept (undoing past the save correctly makes the session dirty against
  the new baseline). A rebind that fails to verify leaves the session untouched and reports that
  the transaction is committed and restorable.
* **Failure / cancel / refusal** → the session is untouched (dirty, same baseline); the engine's
  outcome (Aborted / RolledBack / Blocked) is in the error text and in status.
* **Restore / recovery → refresh** for every open workspace whose files lie under the root:
  identity unchanged → `unchanged`; changed and clean → `reloaded` (working copy + history
  discarded, mirror alignment re-derived); changed and dirty → `stale-kept-edits` (edits kept,
  `stale_baseline` set with the reason; review refuses until `copied_target_reload`, which refuses a
  dirty workspace unless `discard_edits` is set). Nothing is ever silently discarded.

## 5. Companion (content-header) planning rules

* Planned on the destination as it sits in the copied tree; requires the verified content shape
  `<content>/<PROFILE16>/<TITLE8>/<TYPEDIR>/<NAME>/<file>` **and** an existing `Headers/<TYPEDIR>`
  index — otherwise `NotIndexed` and the save is an ordinary non-indexed save (unchanged behaviour).
* `Present` → never staged, never rewritten (asserted byte-for-byte across a second save), and
  **bound** as a `content-header` input (R1 §9.1): a regular file inside the root with its
  identity; gone, changed, a directory or a redirect at apply → refusal, never regeneration.
* `Required` → one artifact, `expected_prior: Absent`, `dependencies: [<save key>]`, producer
  `studio-domain::content_header`, unqualified with a note saying whether it was cloned or
  synthesized. A cloned template is a **bound input** (path + identity) re-verified at apply; a
  changed template invalidates the plan. The template file itself is never written.
* A header whose parent resolves outside the root is a refusal naming the header, the save and the
  root; the root is never widened. A missing parent directory is refused by name at review and at
  apply preflight (and the engine independently refuses it as `MissingParent`); no directory is
  created.
* No sidecar write or discovery happens after commit: the header's bytes are fixed at review and
  installed by the engine like any other artifact.

## 6. Checks actually run (this host, Windows 11, `H:` temp)

```sh
cargo test -p tdb-txn                                  # 33 unit (incl. new abort test) + 4 crash: ok
cargo test -p studio-domain --test copied_target --test txn_coordination --test content_header \
      --test workspace --test session --test save_matrix --lib
                                                       # 148 lib, 3, 4, 3, 10, 11, 1: all ok
cargo test -p studio-api                               # 31 lib (drift gate incl.), 6 export-bindings,
                                                       # copied_target 5, registry 6, roster_builder 1: ok
cargo run -p studio-api --bin export-bindings -- --check   # "Bindings are in sync." (76 roots, 138 files)
cargo check -p nhl-studio                              # Tauri shell compiles with the 8 new commands
npx tsc --noEmit -p app                                # exit 0
cargo clippy -p tdb-txn -p studio-domain -p studio-api --tests   # no warnings in the new/changed code
rustfmt --check on the four new files                  # clean
```

**Domain tests** (`crates/studio-domain/tests/copied_target.rs`, nhl14 PS3 pair on temp copies,
header-indexed fixture layout):
`workspace_saves_twice_through_one_journal_and_restores_in_order` (primary + roster + absent header
planned; dependency order; apply; rebind → clean, reload equals working bytes; second save with the
header preserved byte-for-byte and the installed identity as prior; `RestoreOutOfOrder` on the
first; restore second → clean workspace reloads to cycle 1; restore first → exact originals, header
gone) · `a_dirty_workspace_keeps_its_edits_when_the_files_change_underneath_it` · 
`template_headers_are_bound_inputs_and_existing_headers_are_never_rewritten` · 
`refusals_name_the_root_the_directory_and_the_change` (roster outside root; unmarked root; missing
`Headers/00000001` named by the plan check and by the engine; non-indexed tree → no header;
destination changed after open → conflict, never a new baseline).

**API tests** (`crates/studio-api/tests/copied_target.rs`):
`two_saves_reopen_and_ordered_restore_through_the_api` (labels/kinds/qualification/prior/output
DTOs; apply → rebound; reopen in a second registry shows the edit; second save; superseded flags;
ordered restore with session refresh) · `every_refusal_keeps_the_edits_and_frees_the_root` (edit
after review; content-based revision; external db change; header appeared; missing parent; missing
staged output; foreign-project handle; superseded handle; cancel before mutation → aborted; cancel
after first replacement → rolled-back; **concurrent** edit/save/review/apply/recover during an
in-flight apply refused with the project busy and the handle in progress; restore under dirty edits
→ stale-kept-edits, review refused, reload refused without consent, reload with consent) · 
`a_restarted_registry_discovers_and_recovers_an_interrupted_apply` (child process exits at
`AfterReplaced("primary")`; a fresh registry reports `applying`, blocked, refuses review; recovery →
rolled-back, stale-kept-edits for the dirty session opened on the half-applied tree; then a normal
save; a foreign edit after commit → `blocked` on recovery, foreign bytes retained, review refused,
backups retained) · `a_staged_archive_and_the_workspace_share_one_prepared_service` (synthetic EB
archive with fit + append through `stage_archive`, added through `CopiedTargetPlan::add_archive`
inside the same review; archive outside the root refused at the same seam; apply → reopened via
`BoundSource`; restore → exact original archive, db and no header).

Not run: the repo-wide `cargo fmt --check` / `clippy -D warnings` (known C0.2 baseline failures,
untouched), the full `cargo test --all`, and any real game tree or live install.

## 7. Decisions and limits worth review

1. **Workspace "revision" is content-based** (SHA-256 of each session's working bytes), not a
   counter: an edit that is undone back to the reviewed bytes still applies; any byte change
   refuses. The DTO shows it as a short digest string.
2. **A reserved project refuses reads too.** Every `with_workspace` access refuses a project held
   by a reservation. Cheap and unambiguous; a read-only path could be split out later.
3. **Staging directory** lives under the OS temp dir (`nhl-modding-studio/copied-target-staging/
   <pid>-<seq>-<handle>`), never inside the root (the engine treats every `.tdb-txn/*` directory
   as a transaction). Removed on apply/discard/recovery; the engine copies staged bytes into its own
   recovery directory at stage time, so nothing after prepare depends on it except the "missing
   staged output" refusal.
4. **Re-review supersedes** a project's previous prepared handle (its engine set is aborted first);
   a root with any other non-terminal/blocked set refuses review with the guidance text — recovery
   is never run implicitly.
5. **`tdb_txn::abort`** is the one engine addition with behaviour: without it a consumer would
   have to run root-wide recovery to free a root from its own abandoned preparation. It is confined
   to Prepared/Staged and checked like `apply`.
6. **The archive seam** is `Registry::copied_target_review_with` (`#[doc(hidden)]`, not a command):
   only a Studio-internal producer holding a `StagedArchive` (constructible solely through
   `stage_archive`) can extend a plan. No archive command, no alternate writer.
7. **Fixture layout caveat.** The header-indexed tree in the tests holds the PS3 nhl14 anchors under
   a Recomp-style content path. That exercises the planner and the transaction; it is not a claim
   that any game reads that tree (the tests' own docs say so).
8. `display_path` strips Windows' verbatim `\\?\` prefix from user-facing paths; engine-internal
   paths are untouched.

## 8. Remaining blockers / open items

* **Live integration** stays out of scope: `save_workspace` and the hockey-shop installer are
  unchanged; C3/Q06 production durability, external-concurrency and power-loss qualification are
  still open as recorded in the S2-B-R2 review.
* **No copied-target root provisioning command.** Tests and demos create the root and copy the tree
  themselves (`create_copied_target_root` + explicit directory structure), as the dispatch allows.
  A Studio workflow for "make a copied target from this install" is a later package.
* **Localization DB and PS4 sessions** go through the same path in principle (the primary role
  label covers them) but were not exercised here; the tests cover the nhl14 PS3 pair and a synthetic
  EB archive only.
* Frontend: no page consumes the new bindings yet (Texture Lab UI is the next candidate, not
  authorized here).

## 9. S2-C-R1 corrections (2026-09-17)

Reviewer's probe (`local/reviews/s2c-probe`, run unchanged after the corrections):
`EXISTING HEADER BOUND INPUTS: 1` · `APPLY AFTER HEADER DELETION: Err(… content-header … is gone
… re-prepare (the file is not regenerated silently) …)` · `HEADER EXISTS: false` · `ALIAS SAVE
DURING APPLY: Err(… project 1 is locked by a copied-target operation in progress (apply of review
handle 1 (project 0)) …)` · `APPLY WITH ALIAS WRITER: Ok("committed")`.

### 9.1 R1 — existing headers are bound dependencies

* `stage_workspace`: `HeaderPlan::Present(path)` now produces a `BoundInput { kind:
  ContentHeader, path (canonical), identity }` after `regular_file_in_root`: `symlink_metadata`
  must be a regular file (a symlink/junction → "redirected header is not treated as the save's
  companion"; a directory → "not a regular file"), the canonical path must lie inside the root.
  Template inputs carry `kind: HeaderTemplate`. `BoundInputDto.kind` exposes both.
* `verify_plan_inputs` (staging and apply preflight): a bound input that is gone → "… is gone; it
  was present when the plan was made … (the file is not regenerated silently)"; not a regular
  file / redirected / outside the root → the same refusals as above; identity differs → "changed
  after planning — re-prepare". Refusal happens before any mutation; the existing file is never
  written; edits are kept; the prepared set is aborted so the root is free.
* Regressions: `r1_existing_headers_are_bound_dependencies` (API: unchanged header → commit with
  the header byte-identical; deleted → refused, not regenerated; modified → refused, modified
  bytes left alone; directory in its place → review refused) ·
  `a_directory_in_the_headers_place_is_refused_not_bound` (domain) ·
  `template_headers_are_bound_inputs_and_existing_headers_are_never_rewritten` now asserts the
  Present header is bound with its identity.

### 9.2 R2 — reservations over files and aliasing workspaces

**Scope of a reservation.** `Reservation { holder, root, files: BTreeSet<canonical path>,
whole_root, projects: BTreeSet<u32> }` in `Inner`:

* apply → `files` = every planned destination **and** every bound input (existing/template
  headers); `projects` = the initiator plus every open workspace whose canonical primary or roster
  path is one of those files;
* recovery / restore → `whole_root` = the canonical root; `projects` = every open workspace with
  a file under it.

**Ordering (apply).** (1) the handle is taken out of `prepared` into `in_flight` (a second apply
finds nothing); (2) **one `inner` critical section**: project not reserved → workspace found →
content revision unchanged → no stale baseline → `verify_plan_inputs` → `reserve(...)` (refused
if any existing reservation overlaps on a project, a file, or an enclosing/enclosed whole root);
(3) the lock is released and a `ReservationGuard` is armed; (4) the engine runs; (5) under
`inner` again: on commit the initiator rebinds and every alias is refreshed (§9.3); on any other
outcome no session changes; (6) the guard releases the reservation, the handle is finished. A
preflight refusal never reserves (the set is aborted first). Recovery/restore: `reserve_root`
(one critical section) → guard → engine → refresh every reserved project → release. New opens
(`register`) canonicalize the workspace's files and refuse if a reservation covers any of them,
so a snapshot of open projects cannot be bypassed by an alias opened later; `close_project`
returns `false` for a reserved project; `save` (Save-As) refuses a reserved project or a target
path covered by a reservation; `with_workspace` refuses reserved projects (legacy
`save_workspace`, edits, reload).

**Release.** The guard drops on success, refusal, cancellation, engine error and panic; status
lists `reservations` (holder, root, whole_root, projects, files) and `busy_projects` (the union).

* Regressions: `r2_reservations_cover_aliasing_workspaces` (the exact alias-save probe inside
  `AfterReplaced("primary")`: alias `save_workspace`/`set_field`/`reload` refused, new alias open
  refused, `close_project` of both `false`, Save-As from an unrelated project into a held file
  refused while that project's own edit and Save-As elsewhere succeed, recovery/restore refused,
  status shows the reservation; after commit the clean alias is `reloaded`; a dirty alias is
  `stale-kept-edits` with its edits intact) · `r2_same_project_edits_cannot_slip_between_preflight_and_reservation`
  (4 rounds, fresh tree each: a 40-edit burst racing the apply ends either "refused: edited after
  the review" with ≥1 accepted edit and nothing written, or "committed" with **zero** accepted
  edits; the gated rounds force the second branch deterministically — observed 2/2) ·
  `r2_overlapping_roots_contend` (an apply inside a nested copied root blocks restore/recovery of
  the enclosing root with "overlaps …"; the enclosing project is not held; afterwards the outer
  restore reserves both projects and refreshes them) · `every_refusal_keeps_the_edits_and_frees_the_root`
  (concurrent same-project requests, unchanged).

### 9.3 R3 — refresh failures are reported; sessions stay consistent

* `EditSession::prepare_reload(path) -> PreparedReload` (read + detect + parse + endian, no
  mutation) and `adopt_reload(PreparedReload)` (infallible). `Workspace::refresh_baselines` and
  `reload_from_disk` are **two-phase**: every decision, every reload and the re-derived mirror
  alignment (`analyze_bytes` on the prepared bytes) are computed first; only then is anything
  adopted. An error leaves both sessions exactly as they were, keeps the edits and sets
  `stale_baseline` ("could not be re-read …" / "could not be reloaded …") so copied-target saves
  refuse until a reload succeeds.
* `refresh_workspace` (registry) never maps an error to `unchanged`: `ProjectBaselineDto {
  primary/roster: "refresh-failed", refresh_error: Some(<error>), stale_baseline: Some(<reason>),
  dirty }`. `copied_target_reload(id, false)` on a dirty workspace first re-checks the files: if
  they carry the loaded identities again the mark clears with the edits kept; otherwise it refuses
  (edits kept) unless `discard_edits` is set; a reload that fails changes nothing.
* Regressions: `r3_refresh_failures_are_reported_and_leave_the_alias_untouched` (API: clean alias
  with the roster missing at commit → `refresh-failed`, primary not adopted, review refused,
  reload refused while missing, reload succeeds once the file is back; clean alias with a corrupt
  roster at restore → `refresh-failed`, nothing adopted, initiator `reloaded`; dirty alias with the
  roster missing → edits survive the failed refresh and the failed reload, reload without consent
  refused once the file is back, reload with consent works) ·
  `a_failed_refresh_leaves_the_workspace_untouched_and_marked` (domain: corrupt primary on a clean
  workspace → error, bytes/identity/dirty unchanged, stale set, cleared when the file returns;
  missing roster under a dirty workspace with a changed primary → error, primary **not** adopted,
  failed reload changes nothing, reload succeeds and re-derives alignment once the file is back).

**Failure-state DTO examples** (values as the tests assert them; paths shortened):

```text
ProjectBaselineDto { project_id: 1, primary: "refresh-failed", roster: "refresh-failed", dirty: false,
  stale_baseline: "the workspace's files could not be re-read after a restore or recovery (io error on
    …\00000001\ROSTER 20260723000649\ROSTER 20260723000649: The system cannot find the file specified.);
    the unsaved edits are kept. Put the file back and reload the workspace before saving again",
  refresh_error: "io error on …\ROSTER 20260723000649: The system cannot find the file specified." }

ProjectBaselineDto { project_id: 2, primary: "stale-kept-edits", roster: "stale-kept-edits", dirty: true,
  stale_baseline: "the main database and the roster save changed on disk (a restore or recovery) while this
    workspace had unsaved edits; the edits were kept. Reload the workspace to continue saving, or export
    the edits first", refresh_error: null }

CopiedTargetStatusDto.reservations[0] = { holder: "apply of review handle 3 (project 0)", root: "H:\…\copy",
  whole_root: false, projects: [0, 1],
  files: ["…\USRDIR\nhlng.db", "…\00000001\ROSTER …\ROSTER …", "…\Headers\00000001\ROSTER ….header"] }
CopiedTargetStatusDto.busy_projects = [0, 1]

BoundInputDto { kind: "content-header", purpose: "existing save index header for ROSTER 20260723000649
  (preserved as-is; no header is written because it is present)", path: "…\Headers\00000001\ROSTER ….header",
  identity: { len: 328, sha256: "…" } }
```

### 9.4 Checks actually run (R1)

```sh
cargo test -p tdb-txn                                   # 33 + 4 crash: ok (unchanged)
cargo test -p studio-domain --test copied_target --test txn_coordination --test content_header       --test workspace --test session --lib             # 6, 3, 3, 10, 11, 148 lib: ok
cargo test -p studio-api                                # 31 lib (drift gate), 6, copied_target 10, registry 6, roster_builder 1: ok
cargo run -p studio-api --bin export-bindings -- --write / --check   # +ReservationDto, 4 changed; "in sync"
cargo check -p nhl-studio                               # ok
npx tsc --noEmit -p app                                 # exit 0
cargo clippy -p tdb-txn -p studio-domain -p studio-api --tests   # 0 warnings
rustfmt on the changed files                            # clean
cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target
                                                        # observations quoted at the top of §9
```

### 9.5 Remaining blockers / notes

* Reservations are registry-scoped: they coordinate Studio's own operations in one process. An
  external writer (another process, the game) is outside them — as before, the engine's identity
  re-checks at the mutation boundary are the only defence, with the residual window recorded in
  S2-B-R2.
* The symlink/junction branch of `regular_file_in_root` is exercised only by the directory case in
  tests (creating junctions needs privileges the test runner may not have); the check itself is
  `symlink_metadata`-based like the engine's containment.
* Everything else from section 8 stands.

Stopped for Codex review. Nothing staged, committed or pushed.

### 9.6 Closeout re-verification (2026-09-17, later session)

No code changed in this pass. The S2-C-R1 corrections above were found already complete; the
named regressions exist in the tree and the focused checks were **re-executed** on this host:

```sh
cargo test -p tdb-txn                                   # 33 + 4 (crash children): ok
cargo test -p studio-domain --test copied_target --test txn_coordination --test content_header --test workspace --test session --lib
                                                        # lib 148 (1 ignored), 3, 6, 11, 3, 10: ok
cargo test -p studio-api                                # lib 31, 6, copied_target 10, registry 6, roster_builder 1 (+1 ignored): ok
cargo run -p studio-api --bin export-bindings -- --check   # 76 roots, 139 files, "Bindings are in sync."
```

Temporary artifacts: the API tests' `%TEMP%\nhlms-s2c-api-*` trees are removed by the tests
themselves (verified absent after the run). 17 orphaned staging directories
(`H:\TEMP\nhl-modding-studio\copied-target-staging\<pid>-<seq>-<handle>`, ~85 MB, all from
2026-09-17 S2-C/S2-C-R1 test runs whose copied-target trees were already gone) were removed by
exact literal path after confirming each was a plain directory inside that root, contained no
reparse points, and had no live owning process. Nothing else was deleted. Observation for review:
a handle still *prepared* at process exit leaves its staging directory behind (staging is only
removed on apply/discard/cancel/supersede); harmless in size, but not self-cleaning.

## 10. S2-C-R2 corrections (2026-09-17)

Dispatch: [s2c-r1-review-and-r2-dispatch](../validation/s2c-r1-review-and-r2-dispatch.md).
Reviewer's harness (`local/reviews/s2c-probe`, source `2a6d19e5…6b91fc`, **not modified**), run
after the corrections:
`UNRELATED SAVE AS UPPERCASE RESERVED FILE: Err(… nhlng.db is held by a copied-target operation in
progress (apply of review handle 1 (project 0)); it cannot be written until that finishes)` ·
`APPLY WITH ALIAS WRITER: Ok("committed")` (the earlier four observations unchanged).

### 10.1 R1 — reservation identity (Windows filename case, redirected final component)

* `registry/copied_target.rs::reservation_key(path)` is now the **single** producer of every
  reservation key: canonical parent (directory links resolved, on-disk spelling) + leaf. An
  existing final component is resolved first (`symlink_metadata` then `canonicalize`), so a
  symlink/junction in last position reserves and checks its target. The leaf is lower-cased when
  `directory_folds_case(parent)` says the directory matches names case-insensitively — decided by
  the file system (an existing entry looked up under a case-swapped spelling must resolve to the
  same file), with the platform default (Windows/macOS fold) only when the directory holds no
  entry with a letter. A path with no resolvable parent or no proper leaf is an **error**.
* Consumers: apply computes the keys **inside** the preflight critical section (after
  `verify_plan_inputs`) for every destination and bound input; `Reservation.files` is
  `BTreeMap<key, planned path>` (display keeps the planned spelling); `workspace_files` (open /
  `register`, alias discovery, whole-root membership) and `Registry::save` (Save-As) use the same
  key. `save` refuses a target whose identity cannot be established instead of writing it. The
  whole-root guard (recovery/restore) compares keys against the canonical root, so an equivalent
  spelling under the root is covered too. Refusal text uses `display_path` (no `\\?\`).
* Regression `r2_save_as_under_an_equivalent_spelling_is_refused_before_any_byte_changes`
  (`tests/copied_target.rs`): the exact reproduction (unrelated workspace, `Save As` to
  `…\USRDIR\NHLNG.DB` inside `AfterReplaced("primary")`) → refused, bytes of `nhlng.db`
  byte-identical before/after the call; the planned *absent* header under `…\ROSTER ….HEADER` →
  refused, still absent; opening a workspace on `NHLNG.DB` → refused; a target in a missing
  directory → refused "could not be resolved"; the unrelated workspace edits and saves elsewhere
  during the apply; the apply commits, the initiator rebinds, no aliases, reservations empty, the
  unrelated workspace is still dirty and saves again; then inside a restore (`BeforeRevert`,
  through the new hidden `copied_target_restore_with_hooks` seam) the same Save-As is refused by
  the whole-root guard; afterwards the spelling is writable again. The test asserts the directory
  folds case on Windows (so it cannot silently take the case-sensitive branch); on a
  case-sensitive directory it asserts the distinct file is written and the held file untouched.
  **Verified to fail against the previous `save` logic** (`expected a refusal` at the first
  Save-As) before the fix was restored.

### 10.2 R2 — prepared staging ownership and normal teardown

* `CopiedState::drop_staging` replaces the error-ignoring `remove_staging`: a removal that fails
  (other than not-found) becomes a **tracked leftover** (`leftovers: BTreeMap<path, reason>`),
  retried at every later teardown/recovery and exposed as `CopiedTargetStatusDto.staging_leftovers`
  (new nested `StagingLeftoverDto { path, error }`; bindings regenerated with the exporter, 140
  files, in sync; export-root count unchanged at 76).
* Teardown paths: **`close_project`** discards the project's prepared handles first (abort with
  the engine, then staging; a hard abort failure keeps the project open, returns `false`, and
  retains the staging as a leftover whose reason says to recover the root); **`Registry::
  teardown_copied_target()`** (new, not a command) aborts every prepared handle, removes staging,
  retries leftovers and returns what is still left `(path, reason)` — the Tauri shell calls it on
  `RunEvent::Exit` and prints anything left; **`impl Drop for CopiedState`** is the backstop for a
  normally dropped registry (abort + remove, nothing reported, nothing promised for forced
  termination). `discard` now aborts **before** removing staging (the set references the staged
  files until the engine lets go); on a hard abort failure the staging is retained as a leftover
  and the error returned. Apply's `in_flight` bookkeeping and staging removal are a drop guard
  (`Finish`), so a panic unwinding out of the engine (a hook's assertion) no longer strands them.
* Forced termination (`process::exit`/kill inside the engine) is **not** covered and is
  documented on `Drop`: the prepared/applying set stays on disk for the engine's recovery, which
  aborts/rolls it back from its own recovery directory; the staging directory under
  `%TEMP%\nhl-modding-studio\copied-target-staging\<pid>-<seq>-<handle>` is then unreferenced
  scratch whose exact path is the `replacement` source recorded in that transaction's
  `changeset.json`. No orphan sweeper was added. The `a_restarted_registry_…` test — the only
  producer of that case in the suite — now removes its killed child's staging by exactly that
  recorded path after recovery, so a full test run leaves **zero** staging directories (verified:
  `ls` of the staging root before/after the suite, no new entries).
* Regression `r2_prepared_staging_is_torn_down_on_normal_disposal_and_leftovers_are_reported`:
  a second registry's prepared staging is untouched throughout and still applies at the end;
  `close_project` → staging gone, handle gone, root free (set aborted), no leftovers; explicit
  teardown → staging gone, the handle is refused, the session keeps its edits; a staged file
  pinned without delete sharing → `discard` succeeds (set aborted) and reports **one** leftover
  naming that directory, cleared by the next teardown once the pin is released (non-Windows: no
  leftover expected, asserted); a normally dropped registry with a prepared handle → staging
  gone and a fresh registry sees the set `aborted`. Hidden seam `copied_target_staging_dir(handle)`.

### 10.3 Checks actually run (R2, this host, after every change)

```sh
cargo test -p studio-api                                # lib 31 (drift gate), 6, copied_target 12, registry 6, roster_builder 1, 1 ignored: ok
cargo test -p studio-domain --test copied_target --test txn_coordination --test content_header --test workspace --test session   # 6, 3, 3, 10, 11: ok
cargo test -p tdb-txn                                   # 33 + 4: ok (unchanged crate)
cargo run -p studio-api --bin export-bindings -- --write   # +CopiedTargetStatusDto.ts, +StagingLeftoverDto.ts (140 files)
cargo run -p studio-api --bin export-bindings -- --check   # "Bindings are in sync."
cargo check -p nhl-studio                               # ok (shell exit hook)
cargo clippy -p studio-api -p nhl-studio --tests        # 0 warnings
rustfmt --check on the five changed Rust files          # clean (hunks were confined to new code)
npx tsc --noEmit -p app                                 # exit 0
cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target   # observations quoted above
```

Not run: full workspace clippy/fmt (known C0.2 baseline defects), corpus campaigns, runtime UI.
The junction/symlink branch of `reservation_key` is exercised only through `canonicalize` on
plain files (creating links needs privileges the runner may lack); the case-fold probe was
exercised on NTFS (folding) only — the case-sensitive branch is asserted in code but untested on
this host.

### 10.4 Limits

* Reservations remain in-process; an external writer is still only caught by the engine's
  identity re-checks.
* `directory_folds_case` folds ASCII letters via Unicode `to_lowercase`; NTFS's upcase table and
  Unicode simple folding differ for a few non-ASCII letters — an equivalent spelling that differs
  only there could still miss the guard (it would be caught by the engine's identity checks
  after the fact, as before). Windows per-directory case sensitivity is honoured by the probe.
* Leftover retry happens on teardown events, not on a timer.

### 10.5 Temporary artifacts (S2-C-R2)

* **Removed (developer-owned, this package):** `H:\TEMP\nhl-modding-studio\copied-target-staging\{10116-0-0, 34544-0-0, 36128-0-0, 68368-0-0, 77696-0-0}` — staging left by my pre-fix and deliberately failing test runs and by the forced-exit child before the test cleanup existed; each verified in-root, non-reparse, owner process dead, removed by literal path. Test trees `%TEMP%\nhlms-s2c-api-*` are removed by the tests (verified absent).
* **Retained, reviewer-owned, untouched:** `H:\TEMP\nhl-modding-studio\copied-target-staging\45068-0-0` (Codex's recorded observation; disposable in my judgement now that the lifecycle is fixed, but its deletion is Codex's call); `local/reviews/s2c-probe`, `s2c-probe-data-{48452,56128,58772,70568}`, `local/reviews/s1-codec-target`.
* **New, developer-generated by running Codex's harness:** `E:\Repositories\nhl-modding-studio\local\reviews\s2c-probe-data-15100` (~10 MB copied tree after the post-fix harness run; retained for Codex to inspect; disposable at closeout).
* Scratchpad (session-owned): `H:\TEMP\claude\…\scratchpad\{r2_edit.py, r2_tests.rs, receipt_s10.md}` — helper scripts, no durable value.
* No spawned processes remain.

Stopped for Codex review. Nothing staged, committed or pushed; no frontend change beyond
generated bindings; the Tauri shell change is the one-line exit hook.

## 11. S2-C-R3 corrections (2026-09-17)

Dispatch: [s2c-r2-review-and-r3-dispatch](../validation/s2c-r2-review-and-r3-dispatch.md).
Reviewer's updated harness (`local/reviews/s2c-probe`, source `b361b137…cc275`, **not
modified**), run after the corrections: `ABORT FAILURE STAGING BEFORE: true` ·
`TEARDOWN WITH PINNED JOURNAL: [StagingLeftoverDto { path: …\79836-2-2, kind:
"unresolved-transaction", txn_id: Some("000003-…"), error: "retained: prepared set 000003-… in
… still references this staging — it could not be aborted (…) …" }]` ·
`ABORT FAILURE STAGING AFTER: true` · `JOURNAL AFTER FAILED ABORT: … "state": "prepared"`; after
the harness released the pin its registry's Drop aborted the set (journal now `aborted`) and
removed the staging — nothing left behind. The earlier R1/R2 observations are unchanged.

### 11.1 Retention is decided by the engine's journal, not by a message

* `Obligation` (registry/copied_target.rs) replaces the string map: `RemovalFailed { error }`
  (the set is resolved; only `remove_dir_all` failed — retried by a plain removal) and
  `Unresolved { txn: Box<Transaction>, reason }` (the persisted set still names the staged files
  as its replacement sources — **retained** together with the engine handle needed to retry the
  abort). The two are never confused; a retry never deletes an `Unresolved` entry outright.
* `CopiedState::settle(staging, txn)` is the single lifecycle path (discard, close-project
  discard, teardown, Drop, recovery of stale handles, retry of obligations): it reads the
  persisted state through the public `tdb_txn::status(root)`; `Prepared`/`Staged` → `abort` (the
  only states the engine's `abort` accepts) and release on success; journal gone, `Aborted`,
  `Committed`, `RolledBack`, `Restored` → release (the engine holds its own copies of everything
  it still needs); any other state (`Applying`, `Restoring`, `Blocked`), an unreadable journal or
  a failed abort → retain with the reason. `StaleHandle`/`WrongState` from `abort` are not taken
  as proof of anything: the state is re-read and decides.
* `discard_prepared_for_project` and `teardown_copied_target` settle **every** drained handle;
  the first failure is returned afterwards (close) or reported (teardown), never by early return.
  `teardown_copied_target()` now returns `Vec<StagingLeftoverDto>` (what is still on disk, with
  kind/txn_id/reason); calling it again is safe. `Drop` settles the same way, retains what it
  cannot resolve, and prints each retained path/kind/reason to stderr (the only reporting
  possible there). Forced termination remains uncovered, as documented in §10.2.
* `StagingLeftoverDto` gains `kind` (`removal-failed` | `unresolved-transaction`) and `txn_id`;
  bindings regenerated with the exporter (140 files, in sync; roots unchanged). It also derives
  `Debug` so the reviewer's harness compiles unchanged. The shell's exit hook prints kind+reason.

### 11.2 Regression

`r3_abort_failure_retains_staging_until_the_set_is_resolved` (`tests/copied_target.rs`), two
projects/roots in one registry, project A's journal pinned with `share_mode(1)`: teardown settles
B regardless of drain order (its set `aborted`, staging gone) and retains A (staging present,
persisted state `prepared`, one `unresolved-transaction` leftover naming A's txn and path, root
still held, no pending handle); two repeated teardowns keep it; a re-review is refused and
leaves it alone; freeing the pin → the next teardown aborts the set (`aborted`) and only then
removes the staging (no leftovers, session edits intact); the same scenario resolved by
`copied_target_recover` instead of a retry; a normal `Drop` with a pinned journal retains the
staging and leaves the journal `prepared`, after which a fresh registry's recovery aborts the
set and the test removes the now-unreferenced directory by the path the change set records. On
non-Windows the pin does not block and the test asserts both sets settle at once. **Verified to
fail against the previous retry behaviour** (temporarily deleting unresolved entries → panics at
"retained: the set still references it"), then restored.

### 11.3 Checks actually run (R3, this host, after every change)

```sh
cargo test -p studio-api                                # lib 31, 6, copied_target 13, registry 6, roster_builder 1, 1 ignored: ok
cargo test -p studio-domain --test copied_target --test txn_coordination   # 6, 3: ok
cargo test -p tdb-txn                                   # 33 + 4: ok (unchanged crate)
cargo run -p studio-api --bin export-bindings -- --write / --check   # StagingLeftoverDto.ts rewritten; "Bindings are in sync."
cargo clippy -p studio-api -p nhl-studio --tests        # 0 warnings (one variant-size warning fixed by boxing)
cargo check -p nhl-studio                               # ok
rustfmt --check on the changed Rust files              # clean
npx tsc --noEmit -p app                                 # exit 0
cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target   # quoted above
```

Staging root audited before/after the full suite and the harness: no new directories remain.
Not run: full-workspace fmt/clippy (C0.2 baseline), corpus campaigns, runtime UI.

### 11.4 Limits

* `settle` runs the engine's `abort` while holding the registry's `copied` mutex (short; the
  abort fails fast on a pinned journal rather than waiting). Status calls wait for it.
* An obligation lives in the registry that created it; after that registry is gone (normal Drop
  with an unresolved set, or a kill) a later registry does not know the staging path — it is
  recoverable from the transaction's `changeset.json` (`replacement` sources), and is documented,
  but no automatic sweep exists by design.
* Only `Prepared`/`Staged` sets are aborted by settle; `Applying`/`Restoring`/`Blocked` are left
  to recovery and reported.

### 11.5 Temporary artifacts (S2-C-R3)

* **Developer-generated by running Codex's harness:** `E:\Repositories\nhl-modding-studio\local\reviews\s2c-probe-data-79836` (post-fix reproduction root; its `000003-…` set is `aborted`, staging removed; retained for comparison, disposable at closeout). The earlier `s2c-probe-data-15100` (R2 run) likewise.
* **Reviewer-owned, untouched:** `local/reviews/s2c-probe`, `s1-codec-target`, `s2c-probe-data-{48452,56128,58772,70568,75772,66720}` — `66720/copy` still holds the unresolved `000003-1789637260077-034563920ae8` set in state `prepared` (its staging was deleted by the R2 defect; resolving it is a closeout step: `copied_target_recover` on that root will abort it); `H:\TEMP\nhl-modding-studio\copied-target-staging\45068-0-0` — the only entry left in the staging root.
* **Removed this package:** nothing on disk needed removing; every staging directory my runs created was settled by the corrected lifecycle (audited). Scratchpad helpers only (`r3_edit.py`, `r3_test.rs`, `harness.out`, `receipt_s11.md`).
* No spawned processes remain.

Stopped for Codex review. Nothing staged, committed or pushed.

## 12. S2-C-R4 corrections (2026-09-17)

Dispatch: [s2c-r3-review-and-r4-dispatch](../validation/s2c-r3-review-and-r4-dispatch.md).
Reviewer's harness (`local/reviews/s2c-probe`, source `8f794922…66ed6`, **not modified**), run
after the corrections: `OUTER HANDLE AFTER REFUSED RECOVERY: true` · `TEARDOWN AFTER REFUSED
RECOVERY: []` · `OUTER STAGING AFTER TEARDOWN: false` · `APPLY REFUSAL WITH PINNED JOURNAL: Err(…
edited after the review … the prepared set could not be aborted cleanly and its staging is
retained (retained: prepared set 000005-… still references this staging — it could not be aborted
…))` · `APPLY REFUSAL STAGING RETAINED: true`; after the harness released its pins, its
registry's Drop settled every set (all five journals in `s2c-probe-data-76384` are `aborted` or
`committed`) and no staging was left behind. The R1–R3 observations are unchanged.

### 12.1 Finding 1 — recovery reserves before it takes any handle

`copied_target_recover` now orders: in-flight fast check → `reserve_root` (one `inner` critical
section; refused on any overlap **before any handle is touched**) → take the root's prepared
handles under `copied` → engine recovery → refresh → settle each taken handle against its journal
→ retry obligations. Once the whole root is reserved no apply can move one of those handles into
flight (its own reservation would overlap), so the take is an owned transition; a refused
reservation leaves every handle listed, applicable and its staged bytes untouched.

### 12.2 Finding 2 — apply's exit guard settles, it does not delete

`Finish` (the apply guard) now owns `(staging_dir, snapshot of the prepared Transaction)` and on
every exit — explicit settle before the status read, `?`, or unwind — removes the handle from
`in_flight` and calls `CopiedState::settle` with the snapshot: the journal decides (`Prepared`/
`Staged` → abort; resolved → release; `Applying`/`Blocked`/unreadable/failed abort → retain as
`unresolved-transaction`). The preflight-refusal path settles immediately so the error says
whether the set was aborted ("the root is free") or retained ("its staging is retained (…)").
Nothing in apply deletes staging on the assumption that the abort or the engine's staging
succeeded.

### 12.3 Exit-path table (every `prepared.remove/drain`, `in_flight` transition and deletion)

| Path | Handle ownership | Staging disposition | What proves deletion safe |
|---|---|---|---|
| review: `stage_workspace`/`prepare` fails | none exists yet | `drop_staging` (failure tracked as `removal-failed`) | `prepare` returned `Err` → no engine set names these files |
| review: previous handle for the project | `prepared.remove` → `discard_prepared` → `settle` | per journal | journal `Aborted`/resolved after `abort` |
| apply: unknown handle | nothing taken | — | — |
| apply: handle bound to another project | `prepared.remove` then re-`insert` | untouched | — |
| apply: taken → `in_flight.insert`; **preflight/reservation refusal** | `Finish.settle()` now: `in_flight.remove` | settle with snapshot | journal `Aborted` after `abort`; else **retained** + reported |
| apply: engine `Ok` (committed / aborted / rolled back) | explicit `Finish.settle()` before status | released | journal state is resolved (engine holds its own copies) |
| apply: engine `Err` (failed part-way, cancelled) | explicit `Finish.settle()` | released if journal `Aborted`/`RolledBack`; **retained** if `Applying`/`Blocked`/unreadable | journal state |
| apply: committed but session gone / rebind failed | `?`/return → `Finish` drop → settle | released | journal `Committed` |
| apply: panic in a hook before mutation | unwind → `ReservationGuard` releases, `Finish` drop → settle | released | snapshot still matches disk → `abort` → `Aborted` |
| apply: panic after mutation began | same | **retained** (`unresolved-transaction`) | journal `Applying`; recovery rolls back → later settle releases |
| discard | `prepared.remove` → settle | per journal | as above |
| close_project | `take_prepared_for_project` → settle **every** handle; first error returned after | per journal | as above |
| teardown | `prepared.drain` → settle every handle → retry obligations | per journal; report returned | as above |
| Drop (normal) | `prepared.drain` → settle → retry; retained ones printed to stderr | per journal | as above |
| recover: reservation refused | **nothing taken** (fixed) | untouched | — |
| recover: reserved | `prepared.remove` under the root reservation → engine recover → settle each | per journal | recovery aborted the set → `Aborted` |
| restore | prepared handle on the root → refused before anything | untouched | — |
| forced termination | lost with the process | left on disk (documented; path in `changeset.json`) | not claimed |

### 12.4 Regressions

* `r4_refused_recovery_keeps_the_prepared_handle_and_its_staging`: nested root inside the outer
  root; in the nested apply's `AfterReplaced("primary")` the outer recovery is refused
  ("overlaps"); the outer handle is still listed with the same staging path and byte-identical
  staged files, no leftovers; the nested apply commits; round 1 applies the outer handle normally
  (committed, destinations changed), round 2 discards it (edits kept); staging gone with the
  handle; teardown empty. **Fails against the old ordering** ("the refusal consumed nothing").
* `r4_apply_refusal_with_a_failed_abort_retains_staging`: ordinary refusal (abort succeeds →
  released, journal `aborted`); pinned-journal refusal (destinations, roster and header untouched;
  edits and position kept; handle consumed; staging retained byte-identical; journal `prepared`;
  root held; one `unresolved-transaction` leftover naming the txn; two more teardowns keep it;
  unpin → teardown aborts then removes; the same resolved via `copied_target_recover`); a hook
  panic before mutation (`BeforeStageReplacement`) → set aborted, staging released, reservation
  released, edits kept; a hook panic after mutation (`AfterReplaced`) → journal `applying`,
  staging **retained** and reported, recovery rolls back to the original bytes and the staging is
  then released. **Fails against the old guard** ("retained"). Non-Windows branches assert the
  non-blocking outcome.

### 12.5 Checks actually run (R4, this host, after the last edit)

```sh
cargo test -p studio-api                                # lib 31, 6, copied_target 15, registry 6, roster_builder 1, 1 ignored: ok
cargo test -p studio-domain --test copied_target --test txn_coordination   # 6, 3: ok
cargo test -p tdb-txn                                   # 33 + 4: ok (unchanged crate)
cargo clippy -p studio-api -p nhl-studio --tests        # 0 warnings
cargo run -p studio-api --bin export-bindings -- --check   # "Bindings are in sync." (no DTO change)
cargo check -p nhl-studio                               # ok (shell unchanged)
rustfmt --check on the changed Rust files              # clean
cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target   # quoted above
```

Staging root audited before/after the suite and the harness: no new entries. Not run:
full-workspace fmt/clippy (C0.2 baseline), `tsc` (no binding change), corpus, runtime UI.

### 12.6 Limits

* Settle still runs the engine's `abort` under the `copied` mutex (fails fast on a pinned file).
* The apply guard's snapshot is the *prepared* journal; after the engine advanced the journal the
  snapshot is stale by design — `settle` then relies on the persisted state alone (never on the
  stale handle), which is the intended contract.
* Forced termination remains uncovered (§10.2).

### 12.7 Temporary artifacts (S2-C-R4)

* **Removed (developer-owned):** `H:\TEMP\nhl-modding-studio\copied-target-staging\77280-1-0` —
  leaked by my deliberately reverted-code regression run (the exact Finding-1 leak; its temp tree
  was already gone); verified in-root, non-reparse, owner dead; removed by literal path.
* **New, developer-generated by running Codex's harness:** `local/reviews/s2c-probe-data-76384`
  (all five sets resolved; retained for comparison, disposable at closeout), alongside `-15100`,
  `-79836`.
* **Reviewer-owned, untouched:** `local/reviews/s2c-probe`, `s1-codec-target`,
  `s2c-probe-data-{48452,56128,58772,70568,75772,66720,77572,66316,68400}`;
  `H:\TEMP\nhl-modding-studio\copied-target-staging\{45068-0-0, 66316-3-3, 68400-3-3}` — the
  only entries left in the staging root. `66720` and `66316` still hold unresolved `prepared`
  sets; `68400`'s final set is `prepared` with its staging already gone. Closeout order for each:
  `copied_target_recover` on the root (aborts the set), then remove the exact staging path.
* Scratchpad helpers only (`r4_edit.py`, `r4_tests.rs`, `ct_fixed.rs`, `harness_r4.out`,
  `staging_before.txt`, `receipt_s12.md`). No processes remain.

Stopped for Codex review. Nothing staged, committed or pushed.

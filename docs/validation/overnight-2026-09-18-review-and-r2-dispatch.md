# Overnight developer review — correction dispatch R2

Disposition: substantial implementation delivered; Phase 1 closure and new image-write runtime clearance withheld pending the findings below. Continue the same developer session if available. Do not restart the overnight feature queue before correcting these issues. NHL21-E1 remains accepted and closed.

## Independent checks

Codex read receipts 01–07/status/morning cards as relevant to the reviewed paths, inspected the in-place save, image staging/patch/API/UI paths, localization writer and Phase 1 scope evidence. This is a focused review, not independent acceptance of every overnight change.

Executed `npx vitest run src/components/DictionaryRenamePanel.test.tsx src/lib/project.test.tsx`: 2 files, 6 tests passed. Also compiled and ran a temporary `studio-domain` integration probe against the current implementation. It deliberately supplied a mismatching expected output identity to enter the post-rename verification-error branch of `in_place_save::place`; this is fault-injection/branch evidence, not a physical-disk failure reproduction. Input destination `OLD`, planned bytes `NEW`, expected identity for `BAD`: `publish` returned `Outcome::RolledBack`, `nothing_changed() == true`, while destination bytes were `NEW`. Probe fixture/source/build outputs were scheduled for removal at review closeout; no installed game, image or anchor was changed. The developer's broader Rust/format/real-source results have not been independently rerun here.

## F1 — P1: both writers omit the failing, potentially mutated destination from rollback

`crates/studio-domain/src/in_place_save.rs`, `place`/`publish`: the destination is renamed before read-back verification. `published.push(w)` occurs only after `place` returns success. A read-back I/O error or identity mismatch therefore excludes the already-mutated destination from rollback and classification. The reproduced outcome above contradicts the user-facing claim that nothing changed. Existing before-write and after-success hooks do not cover this boundary.

`crates/studio-domain/src/iso_patch.rs`, `apply`, and `iso_staging.rs`, `apply_into`, repeat this error: range mutation, sync/read-back, and final record update precede `Ok(record)`; only successful records enter `done`. A partial write, read-back error, or final record persistence failure can leave the attempted range changed while the enclosing code reports `nothing had been written` or `the image is as it was`. Restore refuses a partially written range as unknown; the header claim that it handles every state is false. Preserving a backup is useful but is not a completed recovery workflow.

Correct both writers to track mutation intent and the currently attempted destination before publication. Classify/recover it and all earlier writes on every failure. Only claim unchanged after verifying the full affected set. Add fault injection immediately after rename/range mutation, on flush/read-back, and on final record update, including a second-file failure. For partial range writes, provide a bounded recovery path tied to the correct image/range and verified backup; unrelated external bytes must not be silently overwritten. If recovery cannot complete, persist/expose an accurate interrupted/split outcome and usable recovery information. Surface records on failed applies too. Do not claim restart detection for in-place splits without durable evidence supporting it; the current in-memory priors disappear on crash.

## F2 — P1: the reviewed image plan is not the plan applied; operations bypass workspace coordination

`app/src/components/DiscImagePanel.tsx` displays `iso_plan`, but `Yes, write` calls `iso_apply` with only a staging path. `Registry::iso_apply` calls `iso_staging::apply`, which rereads/replans the staged files. A file saved or externally changed after review can be written without reviewing its new bytes. Once confirmation opens, its button only checks `busy`, so a newly dirty workspace is not refused there. The backend accepts no project/revision/review identity and does not enforce the UI dirty check.

The image commands also operate outside the workspace's reservation/close lifecycle: no Registry-level ownership connects this write/restore to the open project's save, close or another session targeting the same image. UI disabling in one mounted component is insufficient.

Bind image review to a server-held immutable snapshot (or equivalent exact reviewed identities), project/workspace revision, staging binding and validated image/member mapping. Reject stale reviews before mutation. Serialize conflicting apply/restore/save operations by actual destination identity, including path aliases and independent staging roots; keep close/status/cancel behavior explicit and responsive. At the mutation boundary enforce applicable running-emulator exclusion for the live source, reusing existing platform detection where possible. Validate reused staging game/platform/metadata and manifest/record paths rather than relying solely on matching the image pathname. Do not silently apply a newly calculated plan in place of the reviewed one.

Regressions: staged bytes change after review; edit/save after confirmation; two roots/projects target the same image; apply races restore; close during an active operation; delayed UI results after switch; running-emulator refusal. Synthetic/disposable sources only. Exercise the new panels with deferred component tests; they currently have none. Do not mutate real images to test this correction.

## F3 — P2: Phase 1 implementation closure exceeds the delivered scope

The status and morning preamble label the phase implementation complete, but their own detailed receipts show otherwise:

- 1.2 provides schedule browse/validation; the roadmap includes supported source schedule editing/import/generation. No finished schedule editor/import/generation workflow is claimed. Other competition/rule/mode-initialization items need explicit coverage or evidence-based blocker entries.
- 1.3 free text is a useful bounded in-place operation, not complete growth/new-key/locale coverage. List the supported operations and pending requirements rather than treating one writer as full localization closure.
- 1.4 has one real Legacy XPR2 replacement round-trip. RW4, PS3 DDS and PS4 GNF league-file replacements remain unexercised, and compressed logo archive placement is unfinished. A loose-file replace with unproven loading does not close archive packaging parity. Inventory non-logo league imagery/variants too.
- 1.5 research establishes no league-named member among the inspected Xbox audio paths. The same handoff says league presentation may be inside generic banks; PS4 is explicitly unknown. That supports `no separately named league bank found`, not absence of league-related audio or phase closure on all targets.

Correct STATUS, MORNING-REVIEW, relevant receipt summaries and the canonical roadmap to an operation-by-target coverage matrix: implemented + tested, implemented + test missing, not implemented, research unknown, evidence-backed not applicable, and user review pending. Keep meaningful partial achievements; do not rewrite historical evidence. Restore active development priority to unfinished Phase 1. Preserve the Phase 2.3 opening slice but do not expand Phase 2 until the Phase 1 gate is honestly satisfied or specifically externally blocked under the campaign's rule. Attach precise remaining work packages and reuse the research lane's handoffs.

The corrected morning cards must not direct live use of the new image writer until F1/F2 are reviewed. Existing accepted NHL21-E1 remains closed; do not request a repeat. Separate existing candidate identity tests from new membership/asset/localization operations when planning user review.

## Execute next

Package: OVERNIGHT-DEV-R2. Fix F1/F2 first, with the required failure/concurrency regressions; then reconcile F3 and refresh the now-stale batch helper/test checksum inventory. No unrelated feature work, live deployment/image writes, game launches, quarantine, staging, commits or pushes. Preserve the installed sources and current runtime backups. Use existing scratch/build ownership rules; remove obsolete owned fixtures and retain only named review dependencies.

One receipt: plain-language summary first, findings dispositions, actual test results, changed-source hashes, corrected per-target readiness and cleanup. Stop for Codex review before resuming Phase 1 implementation. The prior overnight authorization allowed continuing without review during that run; this new correction dispatch defines the current stop point.

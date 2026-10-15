# Developer continuation after receipt 60 — recovery correction first, then PS4 heads

Continue the same developer session. Read `docs/validation/development-20260928-receipt60-review.md` first. This dispatch supersedes completed tasks in the after-receipt-45 queue; its overnight continuation, ownership, source protection and no publication/commit/app-launch/game-launch/listening rules remain. New work is not independently accepted merely because a receipt is ready.

## 1. F1 — preserve the original if Hockey Shop rollback fails

The coordinator reproduced `WriteTxn::rollback` returning an error while deleting the backup containing the original. At reviewed lines 658–670 it removes `bak` regardless of the restoration result. Reproduction source: `docs/validation/evidence/coordinator-r60-rollback.rs` (exact extracted transaction/path/backup code plus a minimal error shim). Use owned scratch only.

Fix this before new features. Preserve a newly created backup when its destination restoration fails; include the usable backup path and failed target in the error/recovery report. Do not delete an unowned directory or file to force restoration. Retire transaction-created backup/temp artifacts only when safe; report failed cleanup without claiming complete restoration. Inspect partial temp writes, deterministic temp-name collisions and create-mode rollback within this correction, using the existing ownership/recovery contract. Never overwrite an unowned temporary file by assuming its name makes it yours.

Add meaningful fault tests: restoration fails after a successful overwrite/backup; a pre-existing backup survives; ordinary successful rollback; create-mode removal; and interrupted/failed temp writing as appropriate. Assert original bytes remain recoverable on disk and paths are reported. Re-run the applied NHL 12 copied create/overwrite regression. Do not call in-memory best-effort rollback crash-safe or concurrent-writer-safe. Publish the correction receipt and continue independent authorized work after your tests pass; Codex acceptance remains pending.

## 2. Recompiled public edit regression

Receipt 59's alias correction and encoding guard pass the coordinator's focused tests. Strengthen coverage through the real catalogue/project id `legacyrecomp`, not only direct `encoding_conflict`. On an owned database copy from the identity-checked registered source, exercise representative weight, rating, birth-date and contract-dollar fields through the public project/API path: display, review, apply, undo/redo, copied save/reopen; include the applicable import seam. Test a wrong declared title refusing before mutation. Keep the accepted alias only for compatibility, not as the sole test input.

Do not infer Recompiled roster-save ceilings, save sidecar compatibility, archive precedence or missing feature admissions from identical database bytes. Use existing source identity and observe the actual registered root/game-subfolder resolution. Keep this bounded; do not repeat every already-green audio vector.

## 3. PS4 head reassignment — bounded existing-head operation

Proceed with the proposed existing-head reassignment using R124/R15 and the current vault reader. Resolve the target against the **open title's identity-checked vault**, with proper bounds/type checks and an unambiguous `ge_nhl_skaterhead` / `SkaterNameAttribute` match. Do not accept a head merely because a different title's catalogue has it. Existing bespoke/pool classification must be explicit; disclose duplicate bespoke faces.

For PS4 NHL 17–21, plan `headid` and `headtexture` together as one reviewed change, preserving the paired stock contract. Verify both fields' declared ranges and presence; fail without partial mutation on incompatible layouts. Do not apply the paired rule to titles without that field or infer a new relationship from coincidence. Preserve unrelated appearance fields. NHL 15's known absent target (10391) is a refusal case; existing malformed/exceptional source rows need a scoped warning rather than silent normalization.

Use genuine NHL 20 plus another measured PS4 title on disposable copies. Cover valid shared/bespoke target, absent target, foreign/stale vault or source, missing paired field, review invalidation, one-step undo/redo and save/reopen of both values. Keep runtime rendering and user P34-1 acceptance separate. No mesh conversion or external head-converter work.

## 4. Release inputs and artwork — await only the actual missing answers

MIT is already approved. Read the latest recorded copyright-holder answer; if none exists, retain the single missing value and continue independent work without another repeated question or a placeholder LICENSE.

**User decision, 28 September:** keep the existing artwork bundled for internal research/team builds, and replace it before external release. Do not start replacement work or ask the artwork-choice question again during this internal-build queue. Preserve the original assets and record their current unrecorded provenance honestly; permission to bundle internally is not evidence of external distribution rights. Retain the external-release blocker and a per-group replacement backlog. Internal candidate assembly should continue to work, while external release mode remains refused until replacement or separately documented rights resolve the entries. No actual upload or sending to the team is authorized by this dispatch.
Keep archive mode explicitly internal-review until its actual prerequisites are met. Packaging eligibility must not imply all application features/tests/user observations are accepted. The release checker must continue to refuse missing licence/artwork evidence. Once corrections merit a new bundle, preserve r59 and earlier copies, record actual dirty source identities and hashes, and update build-specific review cards. No upload, publication or commit.

## 5. Continue and report the whole beta scope

After corrections and PS4 heads, consume the highest-impact implementation-ready League/Team/Player/Equipment work that is not actively research-owned. Board replacement, archive re-key and Austrian phased generation remain withheld pending their separate coordinator contract decisions; do not silently treat them as excluded beta scope or as satisfied by a gallery/refusal. League/team creation/cloning/deletion and executable patching remain explicitly outside beta; Projects/Builds is deferred.

Keep live source/fixture files read-only; copy for write tests. No new agents/chats, messaging other sessions or runtime launches. Claim each package before editing. Respect actual tool restrictions. Use focused meaningful tests with `NHL_STUDIO_FIXTURES` where required and `CARGO_INCREMENTAL=0` as needed; check disk before builds. Exclude `zz_diag` and unrelated fmt/clippy churn. Clean only owned scratch with checked paths/reparse behavior; leave the unowned H:\TEMP folders alone. Do not turn every independent blocker into a stop or repeat expensive sweeps without a new reason.

Consolidated receipt: lead with user-visible results, then F1 recovery evidence, Recompiled/public path and PS4-head coverage, exact tests/failures, new bundle inclusion, complete remaining beta scope and release-input blockers, cleanup and one exact next task. Distinguish developer tests, coordinator review, desktop/listening and game acceptance. No blanket release-ready statement from a green archive checker.

## Coordinator decision, 28 September — NHL 11–13 style labels (S3)

Keep companion style values numeric where no accepted numeric-to-label mapping exists. Research continuity/dictionary vocabulary does not establish the complete mapping. Treat this as a closed presentation decision, not a user question or automatic beta blocker. Preserve already accepted title-specific labels; do not remap stored values. See `docs/validation/research-20260928-receipt5-review.md`. This addendum does not lift the board writer or Austrian phase withholding.

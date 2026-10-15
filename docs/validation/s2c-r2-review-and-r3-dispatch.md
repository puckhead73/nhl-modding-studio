# S2-C-R2 review and S2-C-R3 dispatch

Claude session: continue the same session. A bounded cleanup correction remains in the code just implemented.

Reviewed by Codex, 17 September 2026. Acceptance withheld. The Windows uppercase Save As reproduction is fixed; normal cleanup tests pass, but abort-failure cleanup violates the claimed retention behavior.

## Independent evidence

The submitted hashes matched for registry/copied_target.rs, registry.rs, tests/copied_target.rs and src-tauri/src/lib.rs. The candidate copied_target.rs SHA-256 is f501ee1f3390fa872183447cb83be4ba9b0cdfe108536c12de59bd1f13e3b6ad.

- `cargo test --locked --offline -p studio-api --test copied_target --target-dir local/reviews/s1-codec-target`: 12 passed.
- Original reviewer harness rerun: deleted bound header refused; alias save refused; unrelated workspace Save As to NHLNG.DB refused; initiating transaction committed.
- Extended that review-only harness to pin a newly prepared transaction's journal.json with Windows OpenOptionsExt::share_mode(1), allowing reads but preventing journal replacement. Called teardown_copied_target while the journal remained pinned; released the file afterward and read the persisted journal.

Observed:

```text
ABORT FAILURE STAGING BEFORE: true
TEARDOWN WITH PINNED JOURNAL: Err(... writing journal ... PermissionDenied ...)
ABORT FAILURE STAGING AFTER: false
persisted journal state: prepared
```

Reproduction root: E:/Repositories/nhl-modding-studio/local/reviews/s2c-probe-data-66720/copy. Transaction: 000003-1789637260077-034563920ae8. Deleted staging path: H:/TEMP/nhl-modding-studio/copied-target-staging/66720-2-2. Updated reviewer harness SHA-256: b361b1374d5daae4d360d02c8bb66777c09f19884f6127445a625031920cc275.

No application code changed by Codex. No staging/commit/push. Broader developer checks were not independently repeated. Symlink/junction and case-sensitive-directory coverage remains a disclosed limitation; this review does not qualify those branches.

## Blocking finding — cleanup retries erase abort-retained staging (P2)

In crates/studio-api/src/registry/copied_target.rs, discard_prepared places hard abort failures into the same leftovers map used for deletion failures. teardown_copied_target immediately calls retry_leftovers, which unconditionally deletes every entry. Thus material explicitly reported as retained until recovery is removed in the same call. CopiedState::drop also ignores abort errors and removes staging unconditionally. Repeating teardown or dropping the registry must not bypass retention.

This demonstrates a lifecycle/retention-contract failure, not observed live-game corruption or proof that the engine's own recovery copies are missing. Preserve the distinction between API staging and engine-owned transaction backups.

Related code inspection: discard_prepared_for_project drains all prepared handles into a vector, then returns on the first error. Remaining drained handles can lose cleanup ownership without processing. Address this within the same lifecycle correction, with a multiple-handle regression if supported.

## Executable prompt for Claude

Continue as sole implementation developer on S2-C-R3. Read this review and docs/plans/coordinator-developer-working-agreement.md. Preserve the dirty tree and the accepted reservation behavior.

Correct only the prepared-staging cleanup lifecycle:

1. Separate staging safe to delete after a failed removal from staging retained because transaction abort/resolution failed. Retain enough root/transaction ownership to retry resolution safely; do not infer deletion eligibility from a string message. Repeated teardown and Drop must preserve unresolved material and report actionable paths/reasons where reporting is possible.
2. Once abort or verified recovery resolves the dependency, remove the owned staging and clear its obligation. Do not treat every stale/wrong-state error as proof of safe deletion without checking the relevant engine contract.
3. Ensure project-close and teardown process or retain every drained handle even if one abort fails. Do not lose later handles through early return. Preserve other registries' staging and in-flight operations.
4. Add a Windows locked-journal regression reproducing the evidence above. Assert staging remains on abort failure, persisted transaction state remains accurate, repeated teardown and normal Drop cannot delete unresolved staging, and unlock plus successful resolution permits eventual cleanup. Exercise multiple handles/failure ordering where supported. Keep normal-close, deletion-failure reporting and reservation regressions passing.
5. Run focused API/transaction checks appropriate to changed paths. Run binding generation/check and shell checks if their contracts change. Distinguish checks actually run from earlier receipt results.
6. Start the receipt with a short layperson summary; include changed hashes, checks, limitations, and exact scratch ownership/disposition. Clean your obsolete package-owned files safely. Keep reviewer fixtures only for this active correction and identify their dependency; do not delete the unresolved review transaction or broad temp/cache directories. Stop for Codex review. No new feature work, frontend expansion, live install, commit, staging or push.

## Temporary-artifact disposition

This correction remains active; package closeout is not claimed. Reviewer-owned local/reviews/s2c-probe and local/reviews/s1-codec-target remain the reproduction harness and its active compiled dependencies. Existing reproduction directories 48452, 56128, 58772, 70568, developer-run 15100, and this review's 75772 and 66720 remain under local/reviews/s2c-probe-data-<id> for correction comparison/closeout. The 66720 root specifically preserves the unresolved prepared journal; resolve it before disposal. Previously recorded H:/TEMP/nhl-modding-studio/copied-target-staging/45068-0-0 remains a closeout obligation. No manual cleanup performed in this review. Probe process finished and its journal pin was released.

## Roadmap status

S2-C-R2 reservation correction passes the reproduced failure; S2-C overall remains pending S2-C-R3 lifecycle correction and verified cleanup. After acceptance, use a fresh Claude session for L1.1-A league identity mapping/assessment, working across supported titles/platforms. Teams, players, equipment and miscellaneous follow the entity-editing roadmap. Projects/Builds remain final optional stretch work.

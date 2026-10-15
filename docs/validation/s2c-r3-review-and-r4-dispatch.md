# S2-C-R3 review and S2-C-R4 dispatch

Claude session: continue the same session. The new settlement logic works on the tested teardown path, but two other paths still bypass ownership/retention. Acceptance withheld.

Reviewed by Codex, 17 September 2026.

## Independent verification

Submitted SHA-256 hashes match for registry/copied_target.rs (8581aa8fdbc83cf91745e6b3e3bcf2aa8c941f77d21595d9c42b337bd01fe23e), dto.rs, tests/copied_target.rs and src-tauri/src/lib.rs.

`cargo test --locked --offline -p studio-api --test copied_target --target-dir local/reviews/s1-codec-target`: 13 passed. The original reviewer harness independently confirms locked-journal teardown retains staging, reports an unresolved transaction, and normal Drop after unlocking resolves it. Header and filename-case reservation refusals continue to work; initiating apply commits.

Then extended the reviewer-only harness to exercise recovery reservation refusal and apply preflight refusal. Command: `cargo run --offline --manifest-path local/reviews/s2c-probe/Cargo.toml --target-dir local/reviews/s1-codec-target`. Current harness SHA-256: 8f794922d6f27e9920dcaa1b1b42246a2f24524fc5f690bbf7ef540537f66ed6. No application code modified by Codex; no staging, commits or pushes. Broader developer checks, UI runtime, symlink/junction and case-sensitive-directory qualification were not independently repeated.

## Finding 1 — refused recovery loses prepared ownership (P2)

`copied_target_recover`, around lines 1383–1402, removes matching prepared handles into stale_handles before reserve_root(...)? succeeds. When an overlapping operation prevents the reservation, the local vector is dropped with no restoration or settlement.

Reproduction: prepare the outer copied root; prepare and apply a distinct DB in a nested copied root. In the nested apply's AfterReplaced(primary) hook, request recovery of the outer root. Recovery correctly refuses overlap, but incorrectly consumes the outer handle. Once the nested apply completes, teardown reports no leftovers while the outer staging still exists.

```text
OUTER HANDLE BEFORE REFUSED RECOVERY: true
RECOVERY DURING NESTED APPLY: Err(... overlaps apply of review handle 4 ...)
OUTER HANDLE AFTER REFUSED RECOVERY: false
TEARDOWN AFTER REFUSED RECOVERY: []
OUTER STAGING AFTER TEARDOWN: true
```

Observed in reviewer roots s2c-probe-data-66316 and -68400. This is an untracked cleanup obligation and unusable review handle on a refused operation. Keep prepared ownership unchanged on reservation refusal, or retain it through an explicit guarded ownership transition; do not attempt unreserved cleanup of an overlapping operation as the fix.

## Finding 2 — apply refusal still deletes unresolved staging (P2)

`copied_target_apply_with_hooks`, around lines 1071–1105, installs a Finish guard that always calls drop_staging. Its abort_and_fail closure can fail to abort the transaction, but Finish still removes staging. The new settle contract is bypassed.

Reproduction: prepare a review, pin its journal with Windows read access and share_mode(1), edit the workspace after review, then apply. Preflight refuses the changed revision and abort fails to replace the pinned journal. The persisted state remains prepared but staging is gone:

```text
APPLY REFUSAL WITH PINNED JOURNAL: Err(... edited after the review ... could not be aborted cleanly ... Access is denied ...)
APPLY REFUSAL STAGING RETAINED: false
persisted state: prepared
```

Root: E:/Repositories/nhl-modding-studio/local/reviews/s2c-probe-data-68400/copy. Transaction: 000005-1789638208870-dbf5f34315ce. This demonstrates the same retention-contract defect through apply refusal; it is not a claim of observed live-game corruption or missing engine-owned recovery copies.

## Executable prompt for Claude

Continue as sole Studio implementation developer on **S2-C-R4** in E:/Repositories/nhl-modding-studio. Read this review and docs/plans/coordinator-developer-working-agreement.md. Preserve unrelated dirty work and the tested R3 settlement/reservation behavior.

1. Fix recovery's reservation-refusal path so it cannot drop prepared handles or their cleanup ownership. Maintain synchronization and reservation guarantees; a refused overlapping recovery must leave the original review usable and its staged bytes untouched.
2. Bring apply cleanup under the same state-aware ownership contract as discard/teardown. A failed abort must retain staging and a reportable/retryable obligation. Preserve in-flight cleanup, reservations, session edits and panic handling; a guard must not delete staging based solely on an assumption that abort or engine staging succeeded.
3. Audit every prepared.remove/drain, in-flight transition and drop_staging call in this service. Provide a compact exit-path table covering success, reservation/preflight refusal, engine failure, abort failure and panic: who owns each handle/staging path and what proves deletion safe. Keep this bounded to copied-target lifecycle; no general infrastructure rewrite.
4. Add regressions for both exact reproductions. For refused overlapping recovery, verify the handle and staged bytes remain intact, nested apply completes, then explicit discard/apply resolves the original review normally. For apply preflight refusal plus failed abort, verify unchanged destinations, retained edits/staging, accurate prepared journal and reported obligation; repeated teardown/Drop preserves unresolved staging, and unlock plus settlement/recovery permits cleanup. Exercise the guard's early error/panic paths as appropriate, preserving engine recovery material when needed.
5. Run focused API/domain/transaction checks appropriate to changes; generate/check bindings and check shell if contracts change. Report fresh commands/results and untested limits separately.
6. Start the receipt with a short layperson summary. Include changed hashes, the exit-path table, regression evidence and exact scratch/process ownership/disposition. Clean obsolete developer-owned temporary artifacts safely. Preserve the reviewer evidence listed below for this active correction; no blanket deletion or unresolved recovery disposal. Stop for Codex review. No frontend/new feature work, Project/Build work, live install, staging, commits or pushes.

## Active reviewer artifacts

The prior manifest in s2c-r2-review-and-r3-dispatch.md remains active for correction comparison and closeout; the harness and s1-codec-target build dependencies remain required for independent R4 replay. Add developer-run s2c-probe-data-79836 and reviewer runs -77572, -66316, -68400 under E:/Repositories/nhl-modding-studio/local/reviews.

- -77572: original harness only; normal Drop resolved its transaction/staging.
- -66316: recovery-refusal evidence; outer prepared transaction remains unresolved. Its staging is H:/TEMP/nhl-modding-studio/copied-target-staging/66316-3-3.
- -68400: both reproductions. The earlier outer transaction was subsequently recovered by the probe, but its untracked staging remains at H:/TEMP/nhl-modding-studio/copied-target-staging/68400-3-3. The final apply-refusal transaction remains prepared with staging already deleted by the defect.
- Prior -66720 unresolved transaction and H:/TEMP/nhl-modding-studio/copied-target-staging/45068-0-0 remain closeout obligations.

All probe processes ended and journal pins were released. No manual cleanup performed; package closeout remains pending. Resolve recovery dependencies and preserve durable evidence before deleting exact owned paths at acceptance.

## Roadmap

S2-C-R3 teardown fix verified; S2-C overall awaits R4 lifecycle completion and cleanup. Next remains a fresh Claude session for L1.1-A league identity mapping and existing-workflow assessment. Phases 1–5 follow the entity-editing roadmap; Projects/Builds remain final optional stretch work.

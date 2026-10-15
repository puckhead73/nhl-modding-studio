# LEAGUE-R1 review and LEAGUE-R2 dispatch

Claude session: continue the same session. One small but substantive correction remains in the referential gate just implemented.

Reviewed by Codex, 17 September 2026. Archive snapshot correction and the covered frontend lifecycle regressions pass review at source/API/component level. Acceptance of the complete correction remains withheld for the reproduced F2 gap below. No desktop or game qualification is implied.

## Independent evidence

Receipt hash matches 79272a0c924819dcd3f4e4c9ece70a3de80a89ad2058dfc2bdb6a63cb02794d9; all 16 changed-file hash prefixes listed in the receipt match the current files.

Fresh checks using local/runs/league-overnight-2026-09-17/target:

- `cargo test --locked --offline -p studio-api --test league_copied_target --test copied_target`: 6 + 15 passed.
- From app: `npm exec -- vitest run src/components/CopiedTargetPanel.test.tsx`: 6 passed. An additional direct local vitest.cmd invocation also passed the same six tests; this is one set of six cases, not twelve distinct regressions. Vite emitted deprecation warnings, not test failures.
- Reviewer probe rerun: the original clean NHL 21 dangling-reference edit is refused. The subsequent committed transaction in that original probe is a no-op; it is not evidence of an invalid save passing.
- Extended reviewer probe with an NHL 18 copied fixture to test substitution of a pre-existing invalid reference: new failure reproduced below.

Source review confirms archive bytes now come from the plan's staged primary artifact under the same review lock, with identity verification. The previous double-read race is removed. Frontend tests exercise frozen configuration, revision-based staleness, late response disposal, project-switch update protection and cancellation. Desktop interaction remains unexecuted; no application source changed by Codex, no live writes, no staging/commit/push.

## Blocking finding — equal-count dangling substitutions bypass the gate (P1)

crates/studio-domain/src/session.rs:545:

```rust
if post.dangling.len() <= pre.dangling.len() {
    continue;
}
```

This shortcut runs before comparison of the actual `(record, value)` violations. An operation can introduce a new dangling reference while keeping the count constant, or while repairing more old violations than it creates. Both bypass the promised delta gate.

Reproduced with a copied NHL 18 main DB: stockleagues has no rows for codes 30 or 31. The harness seeds team record 0 with code 31 and recomputes CRCs before opening the fixture, establishing a pre-existing violation. It then uses the normal Registry::set_field API to set that record to 30.

```text
NHL18 LEAGUE 30 EXISTS: false
CHANGE INVALID 31 TO INVALID 30: Ok(())
TEAM 0 AFTER: Some(30)
```

The existing bad value should remain browsable and unrelated edits allowed, but replacing it with a different invalid target introduces a new violation. The current implementation accepts it. This observation is an edit-path reproduction; the extended NHL 18 case did not execute apply/save. No claim of runtime corruption is made.

Harness: E:/Repositories/nhl-modding-studio/local/reviews/league-overnight-probe/src/main.rs; SHA-256 d87e71a8eecc23a5d092819029869ffc0b1e5c057e3ed0497dbdde68a2fcbf48. Command: `cargo run --offline --manifest-path local/reviews/league-overnight-probe/Cargo.toml --target-dir local/runs/league-overnight-2026-09-17/target`. Fixture: local/reviews/league-invalid-reference-74364/nhl18.db. The harness initially had a borrow-check compilation error while constructing the fixture; it was corrected using an immutable source clone before the successful run. No application changes were needed to reproduce the defect.

## Executable prompt — LEAGUE-R2

Continue as sole Studio developer in E:/Repositories/nhl-modding-studio. Read this review and the working agreement. Preserve the existing dirty tree and the reviewed snapshot/frontend fixes.

1. Correct refuse_new_dangling_references: remove the violation-count shortcut and reject newly introduced dangling-reference identities even when total counts are equal or lower. Use the actual pre/post violation sets with an identity appropriate to the operation. Keep existing unchanged violations tolerated and avoid false positives when supported structural operations legitimately move record indices; inspect that case rather than silently weakening the rule.
2. Add the exact NHL 18 invalid-31→invalid-30 regression at the API/domain seams. Assert refusal preserves working bytes, undo/redo, pending state and destination bytes. Keep unrelated edits on pre-existing-invalid rows accepted, valid repair accepted, and older-game exemption intact.
3. Where an existing operation can update multiple rows in one transaction, test repairing an old violation while introducing a different one (equal count and lower count). Do not add a production multi-row API solely for testing; a focused validator test can exercise the relevant before/after states. Also verify unchanged pre-existing violations and legitimate supported row movement do not become blanket blockers.
4. Rerun focused domain/API referential and copied-target regressions; keep snapshot and component tests green if touched. No new feature or phase work. Correct the manual smoke launch command in the receipt: app/package.json has no `tauri` npm script, so `npm run tauri dev` is not executable as written. Verify the actual repository command/config and document it, without claiming a desktop run occurred.
5. Return a short layperson summary first, then changed hashes, exact checks and limitations. No intermediate updates or preference questions needed. Retain only active correction/review scratch; report exact disposition. Stop after this correction for Codex review. No staging, commits, pushes or live installation.

Keep this correction confined to reference validation and its regressions.

## Scratch and roadmap

Retain local/runs/league-overnight-2026-09-17/target as the active LEAGUE-R2 validation cache and local/reviews/league-overnight-probe as the reviewer regression harness. Prior local/reviews/league-invalid-reference-52016 remains under the previous correction manifest; add -74364 for this reproduction. Its NHL 21 no-op transaction is terminal; NHL 18 edits exist only in memory and the fixture contains the deliberately seeded pre-existing bad value. No active recovery dependency was created for that NHL 18 case. Probe and test processes finished; staging root was empty. Package closeout cleanup remains pending acceptance; unowned nhlms-p3-op-24084 remains untouched.

Next after correction acceptance: desktop smoke using actual app commands on disposable copied fixtures, then a separately authorized in-game test plan. Phase 1.1 remains open. No advance to Phase 1.2 or new localization work. Projects/Builds remain final optional stretch.

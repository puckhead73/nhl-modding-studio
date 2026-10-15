# NHL21-E1-PREP-R1 review — R2 required

**Claude session: continue the same session.** The original interruption/rollback defects are substantially corrected; two reproduced qualification/reporting defects remain. No live placement or game launch is cleared.

## Fresh checks

Ran the actual deployment helper and supplied test runner from identical copies under reviewer-owned `local/reviews/e1-prep-r1`, using isolated 4 KB fixture payloads. **23 passed, 1 failed** (test 12). The unreadable-directory scenario did not create an effective enumeration denial here: install printed mods_folder empty PASS and succeeded. The test's icacls invocation does not assert its exit status or verify that enumeration actually fails before testing the helper. This does not independently establish a defect in the helper's enumeration catch; it does mean the claimed negative-path evidence is not portable/reproduced. Other interruption/rollback cases passed.

Reran `verify_db_diff.py` using the bundled Python against the original anchor and retained X DB. **PASS**, 232 CRC words verified per file, zero mismatches; two differing ranges correctly accounted for (fsLK#8 and mkrU.prior_crc). Candidate bytes remain accepted; no regeneration needed.

## Reproduced findings

**F1: resumed install bypasses qualification.** Construct S2 with the helper's interruption hook after rename1. Add a nonempty mods folder and change config. `status` reports `install qualification: FAIL`, but `install` exits 0, renames Incoming to Live and reports INSTALLED (resumed). The S2 branch returns before InstallChecks. Recovery by restoring original should remain available, but completing a modified install must meet the applicable qualification gates or explicitly refuse and direct rollback. Reuse the already verified incoming candidate without unnecessary source-artifact or free-space requirements; do not weaken environment gates just because the process restarted.

**F2: status does not predict the action.** With stock Live and an unknown Incoming, `status` prints `install qualification: PASS` and `install would: copy candidate -> incoming ...`; `install` then exits 2 preserving/refusing the unknown file. State selection ignores this sidecar condition. The existing S7 status also says both modes refuse while rollback intentionally succeeds as a no-op. Derive readiness/action/reason from shared transition logic, distinguishing environment qualification from disk-state readiness. Do not claim all modes are ready merely because environment checks pass.

These were reproduced only on disposable fixtures. No live action was attempted. Reproduction source and results are preserved in `docs/validation/evidence/league-e1-prep-r1-review/` before reviewer scratch cleanup.

## Executable prompt — NHL21-E1-PREP-R2

Continue as primary developer in E:/Repositories/nhl-modding-studio. Read this review. Fix only the two findings and the negative-test setup; retain accepted candidate bytes and the working interruption recovery.

1. Make S2 install completion honor applicable current environment/executable/config/overlay gates. If they fail, preserve all files and name rollback as the recovery action. Do not require a new copy, extra space or an external candidate artifact where a verified incoming candidate already suffices. Keep restoring original independent of install qualification.
2. Make `status` accurately predict install/rollback transitions and blockers for all supported states, including unknown incoming, duplicate original, missing secondary, interrupted install and fallback. Share decision logic with mutation modes rather than maintaining inconsistent descriptions. Distinguish a harmless already-complete no-op from permission to mutate.
3. Add regressions for S2 with changed config/nonempty overlay, and assert status/action agreement for the refusal/no-op cases above. Repair test 12: check ACL setup success and independently confirm the denied enumeration before treating it as injected. Use a fixture-only deterministic enumeration-error seam if needed, and label that as injected evidence. If real ACL denial cannot be established, report that coverage unavailable rather than treating a readable folder as an unreadable-folder test. Keep fixture mode isolated from live paths and test seams disabled in live mode.
4. Rerun the helper tests and a read-only live status with the exact expected emulator hash. Update manifest, hashes and docs/receipts/league-nhl21-e1-prep-r2.md (plain-language summary first). No broad refactor, candidate rebuild, live replacement, emulator/game launch, staging, commit or push. Clean owned fixtures including ACL changes; retain the existing candidate/rollback sources only for the named E1/E5 dependency. Stop for Codex review. Automated game navigation remains off.

## Roadmap

Stock G0 and candidate identities remain accepted. Live E1 is pending this final qualification-path correction; the outstanding stock restart/re-entry check still precedes modified placement. No new feature track is opened.

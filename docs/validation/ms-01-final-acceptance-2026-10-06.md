# MS-01 NHL07 final acceptance

**Result: PASSED — user sign-off: “MS-01 passes.”** All final acceptance checks below are accepted for the initial NHL07 Xbox 360 milestone. No MS-01 acceptance checks remain outstanding within that scope. Next focus is MS-02 NHL07 onboarding; documented deferred scope and legacy test debt remain tracked separately.

Scope: NHL07 Xbox 360, game-default database authoring, current Project Overview, developer sharing, builds and Mod Manager. Save Editor and cross-platform native qualification are not included.

## User-confirmed

- Developer-package import restores the project state successfully (user confirmation after adding Import Project Package to the Mod Studio start screen). Further editing/rebuilding of the imported copy remains part of the final acceptance pass.

- Save/lifecycle behaviors from the MS-01 review are confirmed functional by the user: manual save, timed autosave, save-and-close, discard, cancel, reopening, and closing behavior. No further routine save test is requested.
- QATest was built, installed through Mod Manager and restored successfully.
- Earlier NHL07 default-database edits were verified in game using Restore Default Rosters.

## Implemented in this pass

- Load Project imports `.nhlproject` directly. It previews the title/platform, suggests the organized destination, accepts a different destination, and optionally links the recipient's game source folder.
- Import validates ZIP paths, duplicate names, special files, size bounds, supported project schema, internal manifest paths, baseline/source identities and working database readability. Extraction is private until validation completes; existing destinations are refused. Interrupted-save archives are refused. Limits: 100,000 entries, 128 GB uncompressed, 4 MB manifest.
- A shared project's author installation links and live deployment list are cleared. Supplied local sources are used instead. Sources can be linked later in Project Overview. Original files and the archive are unchanged.
- Interrupted project saves have a conditional **Recover interrupted save** card in Overview. It does not offer recovery for an active operation or while there are unsaved edits. Recovery errors remain visible. An unreadable journal can still require investigation; recovery is not a guarantee against arbitrary disk damage.
- Overview shows net loose-file and materialized-archive differences. Returning either to its original bytes removes the entry. Archive history is not counted as repeated net changes. Unknown ownership remains under Shared assets, with navigation to the file browser.

## Automated evidence

- `project_release`: five tests covering release/archive round trip, direct import, author/version retention, further editing and rebuilding, source relinking, collision refusal, invalid source, damaged ZIP, unsafe archive paths and changed baseline rejection; preserved originals; NHL15 generation separation; blocked unsupported content.
- `project_overview`: cumulative scalar/transfer/jersey changes survive save/reopen and disappear on reversion; loose-file/archive net differences and navigation.
- `project_resume`: five tests covering reopen, relocation/missing sources, schema validation/migration, concurrent project locks and interrupted-save detection without automatic writes.
- `nhl07_default_roster`: scalar admission/rollback guards and database-only build/install/restore. Injected publication failure rolls back; a failed rollback is recoverable using a fresh registry; failed restoration can be retried. Roster saves remain untouched.
- Actual QATest package, tested against a temporary recipient only: 657 changes prepared in roughly 0.7 seconds; installation/restoration and saved-project Overview checks passed. The preceding performance fix was also compared against the former installer's output on the same pre-install backup and produced the identical SHA-256.
- `mod_package`: four tests passed for package compatibility, malformed/tampered package rejection, conflicts, external-change refusal and restoration.
- UI: 18 tests passed across import, recovery, Mod Manager and Overview. TypeScript checking passed.
- Import commands use the off-main-thread command adapter. Native GUI/emulator acceptance remains with the user.

### Existing test debt discovered

The broader `onb003_corrections` suite has eight failures tied to its retired paired/roster-only authoring assumptions: `r2_both_edits…`, three `r3_…` recovery/deployment tests expecting roster/header writes, and four `r72_r1_…` roster-only package tests. They were not hidden, disabled or counted as passing. Six other tests in that suite passed. Current database-only recovery is tested separately above. Migrate those old cases with the separate Save Editor work; they do not establish a failure in the current NHL07 database-only path. This repository-wide suite is not green.

## Final checks for the user

1. **Developer sharing:** open QATest, Package Dev Project Files, then Load Project → Import developer project. Pick the resulting `.nhlproject`, change the final folder name to `QATest Shared Review` to avoid the original, optionally supply the NHL07 source folder, then Import and open. Confirm name/version/author/description and saved edits remain. Make one small edit, save and BUILD. Confirm the original QATest remains unchanged.
2. **Overview review:** in that imported copy, inspect a few historically imported players, a transfer and a rating/name edit. Confirm grouping, original/current values and navigation. Revert one small edit to its vanilla value and confirm it disappears from the net-change count. Existing loose assets appear under Shared assets when ownership is not established; archive files are summarized at file level.
3. **Native acceptance:** with NHL07 closed, deactivate any active test build, activate the newly built copy, and confirm the Installed badges and short preparation time. Launch NHL07, Restore Default Rosters, and verify the chosen historical players/team changes. Close the game, Deactivate, then reopen and Restore Default Rosters to verify originals again.

Do not deliberately crash the application or damage the live installation for this pass. Failure/recovery checks were performed using disposable test folders. If an actual interrupted save appears, use the new recovery card before further edits and report any refusal.

After these checks pass, close **MS-01 initial NHL07 project/build milestone** and move to **MS-02 NHL07 onboarding acceptance**. Revisit the full ISO → extracted source → vanilla/region check → asset preparation → project flow and the existing-dump route, carrying forward previously confirmed extraction results. Cross-platform output, future editor summaries, full archive rebuilding, precise asset ownership/member focus and Save Editor remain explicitly deferred rather than silently marked complete.

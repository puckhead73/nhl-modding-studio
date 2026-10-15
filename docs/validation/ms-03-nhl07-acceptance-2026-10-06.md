# MS-03 — NHL 07 Xbox 360

Status: PASSED — user sign-off, 2026-10-07. All six desktop verification checks passed, including rejection and corrected retry in checks 3 and 5. MS-03 is closed for NHL 07 Xbox 360 Mod Studio. Save Editor integration and qualification are separate work; NHL 07 has no PS3 release.

## Agent verification follow-up

At the user's request, all six scenarios below were executed through the production Registry API in `ms03_six_step_verification_with_prepared_inputs`. **All six passed.** The test recreates the prepared input cases in an isolated temporary directory; it does not depend on local-only fixtures or change QATest, anchors or the installation.

| Scenario | Agent result |
| --- | --- |
| 1. Bundled metadata, optional inputs omitted, Players/Teams | PASS — production API |
| 2. Demitra 38 → 90 → save/reopen → 38 → save/reopen | PASS — production API |
| 3. Wrong metadata rejection, matching sibling retry | PASS — production API |
| 4. Invalid database rejection and corrected input | PASS — production API |
| 5. Invalid dictionary rejection, clearing it permits import | PASS — production API |
| 6. Valid dictionary copy, save/reopen and Players/Teams | PASS — production API; dictionary bytes preserved |

Frontend component verification: all 29 `OpenProject.test.tsx` tests passed, including three new NHL07 error/retry cases for metadata, database and dictionary. They verify visible error text, disabled Next while invalid, no premature creation, and successful submission after correcting/clearing the input. Backend commands are mocked in these frontend tests; this is not native end-to-end evidence.

Native UI automation was attempted using the computer-use skill. Initialization failed with `failed to write kernel assets: The system cannot find the path specified. (os error 3)` both initially and after resetting the kernel. No native UI clicks were performed, and no native visual acceptance is claimed. The user subsequently completed all six desktop checks and confirmed PASS on 2026-10-07. This supplies the native UI acceptance that automation could not provide. The instructions below are retained as the completed verification procedure.

## Checklist

1. **PASS, automated:** bundled game database, game metadata and English dictionary match the inventory and independently recorded USA original-disc manifest. Regression: `nhl07_anchor_identity_and_database_roundtrip` in `crates/studio-api/tests/nhl07_default_roster.rs`.
2. **PASS, automated with historical provenance evidence:** bundled dictionary metadata matches its inventory hash and loads with the dictionary. It is not loose on the ISO: the earlier source inventory identifies it inside `data1.big`, at `fe/loc/nhl_eng_us-meta.xml`, SHA-256 `5971befdedee01b61c3a4a22bcc75cc2332a2871405c4f61499509855402fff2`. See `docs/research/overnight-2026-09-18/01-xbox360-source-inventory.md` and its `data/xbox360-key-files.csv`. This turn did not re-extract that archive member.
3. **PASS, automated:** main database and localization database unchanged load/save are byte-identical. Damaged bundled baseline rejection regression passes.
4. **PASS, automated and prior user verification:** transfer/number changes, undo/redo, save/reopen, build, independent install, restore and publication-failure recovery pass the production API regression. Existing save files remain untouched. Earlier native Vancouver #90/#91 tests establish the tested default-roster path. MS-01/02 acceptance carries forward; no repeat required.
5. **PASS, automated; UI acceptance below:** missing metadata uses the bundled schema; mismatched or malformed supplied metadata is rejected. Imports reject invalid databases, saves masquerading as main databases, and dictionaries masquerading as main databases. NHL07 project reopening also checks metadata coverage and field widths. Optional dictionary omission leaves roster editing/save/reopen available; an invalid supplied dictionary must be corrected or cleared. Rejected imports do not create a project or write source files; correcting metadata allows reopening. These are structural checks, not proof of every metadata label's semantics or every game revision.
6. **PASS, user sign-off 2026-10-07:** all six desktop checks below passed. Field-specific semantics and native behavior belong to DB/AA/LOC work packages.

## Current automated evidence

- `cargo test -p studio-api --test nhl07_default_roster`: anchor/roundtrip, import boundary and production workflow checks; optional environment-driven bulk-package test remains ignored.
- `cargo test -p studio-api --test project_inputs`: all five tests passed, including asset-only gating and omitted-dictionary behavior.
- `cargo test -p studio-domain --lib baselines::tests`: two tests passed.
- No live installation, project, or save files were changed.

## Single-pass user verification

Prepared inputs: `E:\Repositories\nhl-modding-studio\local\ms03-review`. These are disposable copies, not the anchors or your QATest project. For each new import choose **Mod Studio → Import non-studio project → NHL 07 / Xbox 360**. Keep the game-assets input empty. Use the project names below or another unused name; leave completed projects saved for diagnosis if anything fails.

- [x] **1. Bundled metadata and optional inputs.** Name the project `MS03 DB Only`. Set Main database to `database-only\NHL07NG.db` under the prepared-input folder. Leave Localization dictionary and Game assets folder empty. Proceed into Studio (skip asset extraction if offered). Open Teams and Players, then Pavol Demitra. PASS: project opens, lists work, Demitra is on Minnesota #38, and no roster-save requirement or metadata error blocks you.
- [x] **2. Save/reopen with minimal inputs.** In that project change Demitra's number on Minnesota from 38 to 90. Click Save Session, close the project, and load `MS03 DB Only` again. PASS: Demitra remains Minnesota #90; editing still works without a dictionary/assets. Change back to 38 and save. This checks persistence; the already accepted in-game default-roster test need not be repeated.
- [x] **3. Wrong metadata rejection and retry.** Start another non-studio import named `MS03 Metadata`. Select `wrong-metadata\NHL07NG.db`. Attempt to proceed. PASS: Studio explains that metadata does not match and does not open the project. Change Main database to `matching-metadata\NHL07NG.db` and retry. PASS: the same onboarding session opens successfully with the matching sibling metadata. Close this disposable project normally.
- [x] **4. Invalid database rejection and retry.** Start an import named `MS03 Invalid Input`. Select `invalid-database\NHL07NG.db` and attempt to proceed. PASS: a database error appears and no editor opens. Change Main database to `database-only\NHL07NG.db`; verify the database is accepted, but remain on setup for check 5.
- [x] **5. Invalid optional dictionary recovery.** In the same setup, select `optional-dictionary\invalid.db` as Localization dictionary. Attempt to proceed. PASS: review identifies the dictionary as invalid. Clear that dictionary field and proceed. PASS: the database-only project opens and Players/Teams remain usable. Close the project normally.
- [x] **6. Valid dictionary attachment.** Start an import named `MS03 With Dictionary`. Select `database-only\NHL07NG.db` plus `optional-dictionary\nhl_eng_us.db`. Proceed, inspect Teams and Players, save, close and reload the project. PASS: no dictionary error appears; both views remain usable after reopening. Detailed localization key editing is LOC-01, not this check.

User reported all six numbers PASS on 2026-10-07. Anchor hashing, no-op byte identity, corrupt working-metadata rejection/recovery and source preservation are automated checks; no manual hex editing, anchor alteration, or native reinstallation is needed. All six passes close this NHL07 MS-03 milestone; other targets and the separate Save Editor remain unqualified.

## Deferred to Save Editor

The NHL07 change-list classifier, overlay, and narrow jersey/speed writers retain their existing evidence. Production Save Editor integration, biography/goalie updates, created-player/deletion semantics and broader container/native qualification remain separate. Snapshot mirroring must not be enabled for these difflists.

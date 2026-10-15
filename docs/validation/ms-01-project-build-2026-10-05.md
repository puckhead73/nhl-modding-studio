# MS-01 initial Project and Build verification

Latest status: see [NHL 07 Xbox 360 onboarding application verification](#nhl-07-xbox-360-onboarding-application-verification). Earlier entries retain the evidence available at their original checkpoints.

Date: 2026-10-05. Task: MS-01.1-A in the [feature roadmap](../plans/feature-work-package-roadmap.md).

## Outcome

Completed the bounded baseline-and-build-gating task. The existing foundation already implements project creation/import, isolated baseline/working files, save/reopen and saved-edit packages. No replacement shell was created. Full MS-01 remains incomplete.

Fixed BuildPackagePanel offering a build from old inventory data during a refresh, after refresh failure, or when the session already reports unsaved edits despite a clean inventory response. Build now requires a successfully loaded, non-refreshing inventory and a clean session. Refresh and unsaved-state messages explain why building is unavailable. The backend's existing unsaved-edit rejection remains authoritative.

Two new regression tests failed before the repair and passed afterward. They check unsaved-session disagreement and refresh → failure → successful retry, including absence of package_build calls while unavailable. Existing include/exclude and target-selection behavior still passes.

## Executed checks

| Command | Result | Scope |
| --- | --- | --- |
| cargo test -p studio-api --test project_workspace --test mod_package --offline | 13 passed, 0 failed, 0 ignored | 9 project-workspace and 4 package integration tests through the Registry seam |
| npm test -- src/components/BuildPackagePanel.test.tsx src/pages/OpenProject.test.tsx | 17 passed across 2 files | Mocked API frontend interaction; 4 Build tests and 13 onboarding tests |
| npm test -- src/components/BuildPackagePanel.test.tsx | 4 passed after final TypeScript adjustment | Final changed component regression check |
| npx tsc --noEmit | Passed | Application TypeScript check |

Frontend commands ran from app/. Backend commands ran from repository root. Existing Rust warnings and frontend Vite deprecation warnings were emitted; none caused these checks to fail. No desktop bundle or in-game execution was performed.

Backend checks exercise source preservation, save/reopen, explicit deployment/restore to owned fixture copies, distinct project staging, source import, baseline gaps, extraction behavior, package tampering rejection and target declarations. Package fixtures include NHL 21 PS4 and Legacy Xbox-authored edits applied to PS3/Recompiled owned bases. This is filesystem/format evidence, not native-game qualification or proof that all publication failure modes are covered.

The baseline-loop test asserts 26 successful fresh-project opens and one unavailable target: NHL 20 PS4. It checks identity and selected team views; it does not demonstrate complete editing across those 26 targets. The specific cause/remediation of the NHL 20 PS4 baseline gap remains to be examined under MS-03. Do not mark that target's fresh-project path complete.

## Existing boundary and deferred work

ProjectStore retains baseline/ and working/ separately. PendingEdits reviews session changes against the last acknowledged save. It therefore cannot meet the cumulative Project Overview contract merely by changing its title. Baseline-to-current comparison and entity categorization need their own implementation.

Deferred: actual desktop interactions; complete cumulative Overview; every change producer's integration; native package acceptance; all format/signing paths; comprehensive crash/conflicting-writer checks; NHL 20 PS4 fresh baseline availability. Existing audit findings remain open until individually reproduced and repaired, including the deployment publisher's external-write handling.

## Source identity and change scope

Starting HEAD: 2e9ac1b8f2363559f4f7375f999f86f202fe3153 plus extensive pre-existing working changes (1109 porcelain status entries). HEAD alone does not identify the tested source. Both BuildPackagePanel files were already untracked; they were edited in place without staging, committing, resetting or replacing other work.

This task added the consolidated roadmap and this evidence record, and narrowly changed app/src/components/BuildPackagePanel.tsx and its existing test file. No live game installation was modified by the task; backend tests use owned temporary copies.

## Next bounded task

Proposed MS-01.2-A: implement and verify cumulative original-baseline-to-current comparison for one scalar player edit, including save, close/reopen and return-to-original behavior. Keep the current pending-session comparison distinct. Establish the persisted comparison first; then connect the expandable Project Overview. Asset, creation and historical-package summaries remain separately deferred to their feature dependencies. User retains sequencing control.

## User refinement implemented after initial review

MS-01 project storage now uses one persisted `projectsRoot` preference shared across all targets. Onboarding asks for that folder and a new Project Name, displays the derived child directory, and sends that path through the existing creation API. Settings → Paths exposes the same preference. New-project names are not remembered, preventing accidental reuse of the previous project's folder. Legacy recent/existing-project opening remains available; changing the storage root does not relocate existing projects.

Validation rejects missing/relative roots, path traversal/separators in names, Windows-invalid characters/device names and trailing periods/spaces. The existing backend remains responsible for canonical path/source separation and refusing non-empty destinations. No new file mover or alternative project store was introduced.

Checks: `npm test -- src/pages/OpenProject.test.tsx src/lib/projectLocation.test.ts` passed 40 tests, including persistence across remount, derived creation arguments, invalid-name correction, Windows/UNC path joining and existing-project reopening. `cargo test -p studio-domain --lib project_store::tests::create_open_and_guard --offline` passed the existing create/reopen/source-preservation/collision check. `npx tsc --noEmit` passed. Desktop visual review remains for the user's running Studio; these checks do not claim manual desktop verification.

## Simplified entry flow and organized storage refinement

The subsequent user refinement replaces the flat new-project layout above with `<root>/<platform>/<game>/<project name>`. Stable catalog IDs name the game folders; Xbox uses `xbox360`. Existing projects are not moved and remain loadable.

The root route now presents only Mod Manager and Mod Studio. Manager shows an under-construction notice with access to the existing project-build installer at `/manager/install`; `/install` redirects there. Studio offers Create New Project, Load Project and Import non-studio project. Each Studio action prompts for the default NHL Project folder when none is configured. Creation/import always exposes game selection with Project Name, including when a startup target was previously remembered. Loading has its own recent-project/folder view.

BuildPackagePanel now supplies the explicit output path `<root>/Builds/<build_name>_<platform>_<game>.nhlmod` through the existing guarded package builder. Multi-platform names use sorted unique platform IDs joined with hyphens. Builds is created on the first successful build, and collisions preserve the existing release. Version remains package metadata rather than part of the filename; a different release filename requires a different build name. The displayed output path reflects the actual request.

Verification: 49 tests passed across Start, OpenProject, BuildPackagePanel and projectLocation tests; TypeScript passed. The targeted backend `mod_package` test `a_package_authored_on_a_platform_specific_base_declares_only_that_platform` passed with added assertions for shared Builds directory creation, exact release path, valid package inspection and refusal to overwrite an existing release with unchanged file identity. Other backend package tests were not repeated for this refinement. No native game or manual desktop visual verification is claimed.

## NHL 07 fresh Project Setup refinement

The fresh path now uses Project Setup with white game text and platform-specific colors (Xbox green, PS3 purple, PS4 blue). Removed introductory instructions and fresh-path database/save/install/legacy ISO panels. Kept name and folder, added a per-project folder browser without changing global storage, and moved optional baseline gaps into collapsed notes. An unavailable baseline remains an explicit blocker.

Select Game Source opens a native modal for ISO or dumped-folder selection. ISO selection advances to extraction-destination configuration with an emulator-location explanation, then returns to a summary card. Changing/cancelling source selection preserves the appropriate confirmed state. PlayStation installation handling remains deferred. Main setup Next calls the existing disc extraction service, polls status and offers cancellation; only successful disc completion proceeds to fresh creation using packaged anchors with null external save selections. A completed dump can be reused within the same setup attempt if project creation needs retrying. Failed/cancelled dumps require a new extraction destination. Existing file import and project loading remain separate flows.

Verification: 50 frontend tests passed across OpenProject, Start and projectLocation; TypeScript passed. The 22 onboarding tests include NHL07 ISO command ordering, completion/failure/cancelled-state gating, no iso_open/import command on fresh creation, source changes and folder override. The existing two `studio-domain` disc_extract tests passed on synthetic fixtures, covering safe paths, streamed extraction, source preservation and refusal to reuse an output folder. No user ISO was extracted, no emulator was launched and loose-file override support remains unverified. Actual desktop visual review remains with the user.
## Supplied Xbox 360 extraction engine integration

Windows Xbox ISO extraction now invokes the exact headless `exiso.exe` 2.7.1 supplied alongside `E:\Tools\XBOX ISO Extract\XBOX360 ISO Extract.exe`. The engine is embedded in the backend; the original tools directory and GUI are not required. Studio retains its source/destination selection, background job, progress and cancellation. Invocation uses extraction mode with `$SystemUpdate` exclusion, matching the supplied tool's default. Binary hash, reference source and license are retained under `crates/studio-domain/vendor/extract-xiso`; About includes attribution and the license.

The existing disc manifest checks run before launch. Studio requires a fresh destination and verifies every expected extracted file's size and bytes against the ISO before completion. Cancelled/failed output is retained for inspection and cannot be reused as a new extraction destination. ISO9660 and non-Windows extraction retain their existing implementation.

Verification: `cargo test -p studio-domain --lib disc_extract --offline` passed 3 tests, including execution of the supplied engine against synthetic bare XDVDFS and XGD2 fixtures, update-directory exclusion, source preservation, cancellation and destination refusal. `cargo test -p studio-api --test disc_extraction_job --offline` passed 2 integration tests. `npm run build` passed TypeScript and the production Vite build, including bundled license text. Existing Rust and bundle-size warnings remain. No user NHL 07 ISO extraction, emulator boot or loose-file override testing was performed; those remain the next real-game qualification steps.

## NHL 07 Xbox 360 onboarding application verification

Date: 2026-10-05. Target: NHL 07 Xbox 360, Windows Studio. State: **Application verified** for the current onboarding scope. Evidence: user verification in this chat after Studio rebuilt with the project parent-folder fix:

> Works now - I've also verified that the ISO extraction is functional, and that selecting an existing dumped ISO folder also works and progressing onboarding.

| Operation | Current evidence and status |
| --- | --- |
| ISO selection, extraction destination, bundled exiso extraction and progression through onboarding | Application verified by the user on NHL 07 Xbox 360 |
| Existing dumped ISO folder selection and onboarding progression | Application verified by the user on NHL 07 Xbox 360 |
| Project creation under the organized storage hierarchy | Application verified by the user after the missing-parent fix |
| Current onboarding presentation/navigation and name/folder refinements | Accepted as part of the user's current Xbox 360 onboarding checkpoint; individual visual controls were not separately enumerated in the verification report |
| Name/path validation and focused onboarding behavior | 62 automated frontend tests passed; hyphens/underscores preserved, specific active errors reported, empty untouched name has no inline note |
| Parent directory creation, existing-project protection, failed-copy cleanup and project workspace behavior | 6 project-store tests and 9 project-workspace API tests passed after the fix |
| Extraction guards, cancellation and output comparison | 3 domain tests and 2 API tests passed; manual cancellation/failure scenarios are not claimed |

Project creation now creates missing ancestor directories before exclusively claiming the project leaf. Failed creation removes its incomplete project folder while retaining shared parent folders. Studio rebuilt and relaunched with the fix before user confirmation.

This closes the pending real-application verification for the current NHL 07 Xbox 360 ISO/dumped-source onboarding work. Earlier statements that no user ISO extraction had been verified are historical and superseded for this scope. Verification is user-reported; no screenshot of a completed extraction, ISO hash or runtime receipt was captured in this update.

Still deferred: emulator boot from the extracted installation, loose-file override precedence, compatible NHL 07 roster-save anchoring/mirroring, subsequent editing and released-build acceptance, and qualification of other games/platforms. The existing ROSTER1 schema mismatch remains open under MS-03. This checkpoint does not mark all Xbox 360 functionality or the entirety of MS-01/MS-02 complete.


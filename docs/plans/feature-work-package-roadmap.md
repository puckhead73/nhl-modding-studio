# NHL Modding Studio feature implementation roadmap

Accepted scope consolidated on 5 October 2026. The user owns prioritization and assignment. Build on the current Studio foundation, one bounded feature task at a time, verified game by game and platform by platform.

This roadmap supersedes the selective-rebuild proposal and older sequencing that defers Project/Build functionality. It does not require a new application shell, canonical project rewrite, or unrestricted cross-game conversion. Existing code is a starting point, not proof of completion. Preserve the current working changes.

## Current handoff — 2026-10-11

Tester package prepared 12 October: `out/tester-package-2026-10-12/NHL Modding Studio Testers 2026-10-12.zip` (611,685,021 bytes, SHA-256 `6e333159…450f`): Save Editor incl. NHL 07/08 change lists, NHL 08 release/install and NHL 07 career-package fixes; same payload as 11 October; 1,051 files verified on a fresh extraction and extracted-app startup with bundled WebView2 passed. Internal tester packaging, not acceptance.

Tester package prepared 11 October: `out/tester-package-2026-10-11/NHL Modding Studio Testers 2026-10-11.zip` (611,353,365 bytes). Includes historical corpus, 150 verified bundled anchor files, current embedded Studio artwork, definitions, audio/PKG helpers and private WebView2. Production build, full archive verification, three extracted-resource tests and extracted-app startup/normal close passed. Handoff/checksum are alongside the ZIP. This is internal tester packaging, not additional feature acceptance; latest equipment preview visual review remains pending.

**Active target: NHL Legacy parity with the completed NHL 07 work (user direction, 11 October).** Start with Xbox 360 as the continuation of the current review platform; track PS3 separately. NHL07 MS-01–03 and DB-01–06 remain the reference scope, with the direct-observation fixes below carried forward where applicable. DB-07 has not been started in this qualification sequence. Existing Legacy functionality is not an acceptance pass. Begin with onboarding/source verification, then project/build/restore and database features; preserve user projects and installations.

Initial code review confirms that Team View's Schedule tab and the revised League competition/structure views are explicitly gated to NHL07. These are known presentation/adaptation gaps to investigate for Legacy, not features to enable by removing guards without checking its schema. The older Legacy Xbox360/PS3 beta sheets predate the current project workflow and do not establish parity. The development executable was launched with a responsive window for this review.

This handoff and the latest explicit acceptance records take precedence over older pending checkpoints and “next package” statements retained below as history. A passed package means its documented NHL07 Xbox360 scope passed, not every proposed feature or every game/platform.

| Package | Current status / accepted scope |
| --- | --- |
| MS-01 — Project and file management | Passed. Project save/reopen, autosave, builds, developer packages/import, Mod Manager activation/deactivation and restoration. |
| MS-02 — Onboarding | Passed. All 14 review checks, including corrected ISO failure/cancellation/retry behavior. |
| MS-03 — Anchors and format handling | Passed. All six supplied Mod Studio review checks. |
| DB-01 — General player editing | Passed for existing players, including transfers and free-agent package support. Record creation/duplication/deletion remains DB-08. |
| DB-02 — General team editing | Passed, including drag/drop corrections, immediate feedback and individual Overview reverts. |
| DB-03 — Advanced player editing | Passed for supported equipment/art/audio assignments. Asset authoring and full lookup/appearance coverage are separate. |
| DB-04 — Advanced team/league editing | Passed, including calendar, organization, schedule CSV/generation, packaging and the supplied native checks. Final tab order, gold All-Star card, preselected game dialog and CSV encoding refinements also passed. |
| DB-05 — Historical database/roster building | Current non-asset functionality passed by user report. Historical assets explicitly deferred to DB-08 review. |
| DB-06 — Arena editing | Passed. All nine supplied UI/native checks, including shared venues, assignment editing, persistence, installation and restoration. |
| DB-07 onward | Not yet qualified in this sequence. Existing implementation/research does not count as acceptance. |

PS3 native mesh integration (11 October): [implementation and evidence](../../research/rpsgl2-review/README.md#native-integration-11-october-2026). Native EDGE decode, static geometry export, textured Studio previews, OBJ/GLB export and guarded same-topology OBJ import are implemented. All 14 selected Legacy PS3 samples match the independent reader. Import is currently exposed by the native CLI; project edit tracking/build/Mod Manager mesh operations remain pending. RPCS3 acceptance, arbitrary topology, rigging, arena import and automatic cross-platform conversion are not qualified.

Current review focus: **Legacy PS3 asset parity**, per user direction following the DB-01 checklist. [Implementation, evidence and one-pass review](../validation/legacy-ps3-assets-2026-10-11.md) covers logo/texture/head/equipment previews, exports, catalog discovery, project-aware preview refresh and multi-raster texture packaging. User visual/RPCS3 acceptance is pending. Legacy DB-01 has not received sign-off and is paused while this asset pass is reviewed.

### Decisions and open dependencies to carry forward

- Mod Studio edits default game databases and assets; saves belong to the separate Save Editor. Do not reintroduce automatic save/database mirroring.
- Continue on the existing foundation, feature by feature and game/platform by game/platform. NHL07 has no PS3 release. Earlier NHL08 onboarding investigations do not constitute a full NHL08 qualification.
- Project source installations supply reference files. Mod Manager owns installation and restoration; do not bring direct deployment back into the Studio editing flow.
- Project Overview summarizes the project's changes relative to its creation baseline, not the number of editing actions. Saved changes remain visible after reopening; individual supported edits can be reverted.
- Historical portraits, heads, supporting assets, appearance and equipment packages must be reviewed with DB-08, coordinated with AA-01/03/05. Their deferral is not a pass.
- League naming/presentation mappings, affiliations and competition expansion remain unqualified where documented in DB-04.
- DB-06 does not qualify new arena creation/duplication, asset authoring, every assignment combination or every saved game mode. Ice quality, crowd type and crease behavior remain read-only/unqualified.
- The separate Save Editor remains independently scoped. No broad save-editing acceptance is implied by MS-03 or the earlier difflist experiments.

### Review builds and files

- Latest built executable at this checkpoint: `E:\Repositories\nhl-modding-studio\target\debug\nhl-studio.exe`, DB-06 build. It was relaunched and its responsive desktop window verified. The development build needs the frontend server at `http://127.0.0.1:1420`. Process IDs are transient; verify a real window instead of relying on an old process.
- DB06 project: `E:\Repositories\nhl-modding-studio\local\DB06 Review 2026-10-11`; prepared release in `exports\DB06_Review_xbox360_nhl07.nhlmod`; developer archive in `local\DB06 Review Deliverables\Project Archives\DB06 Review.nhlproject`.
- DB04 project: `E:\Repositories\nhl-modding-studio\local\DB04 Review 2026-10-08`; developer archive in `local\DB04 Review Deliverables\Project Archives\DB04 Review.nhlproject`.
- These are separate QA fixtures. Preserve QATest and user edits. Do not infer the currently loaded project or installed mod from the existence of a fixture.
- Latest detailed evidence: [DB-04](../validation/db-04-nhl07-review-2026-10-08.md), [DB-06](../validation/db-06-nhl07-review-2026-10-11.md). DB06 passed 15 backend and 16 UI tests plus frontend/native builds before user sign-off; two pre-existing external-fixture tests were ignored.

### Direct-observation review log

The prior DB04 refinements and DB06 review are accepted. The current Leagues observations are tracked below; partial mapping is not treated as completion. See [review evidence and remaining research](../validation/league-review-2026-10-11.md).

| ID | Observation / affected area | Fix and automated evidence | User verification | Status |
| --- | --- | --- | --- | --- |
| OBS-01 | League Identity | Dictionary-backed NHL/National and NHL group labels; anchor-backed tests | Pending | European bindings/consumer tracing open |
| OBS-02 | Competition names/presentation | Current dictionary text table and edit entry points | Pending | Names partially mapped; logo/commentary assignments open |
| OBS-03 | Competition structure | Team table, conference/division Apply/Cancel and schedule-safe league swaps | Pending | Team reassignment ready; entity add/remove and whole-division moves open |
| OBS-04 | League logo notes | Routine bullets removed; errors retained and UI tested | Pending | Ready for review |
| OBS-05 | Team View cleanup | Removed index/art swap card, Team Text, name calls/song catalogue and logo developer notes; updated miscellaneous-field warnings throughout Team/Player/Arena/Attributes. TypeScript/production build passes. | Pending | Presentation ready; Art ID dropdown subsequently implemented under OBS-06 |

Art ID follow-up (OBS-06): Identity now contains an ID/current-team dropdown. Selection automatically swaps IDs with that owner, including dependent rows, as one undoable operation. NHL07 resolves its stock/custom identity rows. Package replay admits complete ID permutations with required dependent edits; raw scalar writes remain refused. Shared/reserved IDs remain visible but unavailable. UI tests cover selection, owner labels/Undo refresh and refusal. Native checks cover NHL07 swap, byte-exact Undo, save/build/install/restore, incomplete-swap refusal, and existing later-title swap behavior. User review pending: choose another team's ID, verify both teams' dropdowns/logos, Undo/Redo, then verify the built mod in game.

Uniforms follow-up (OBS-07): NHL07 equipment discovery now searches nested extracted folders, including common geometry, and recognizes its sock_/skate_ filenames. Texture siblings are excluded from mesh selection. Nameplate card previews and Edit locations use the selected kit's explicit jersey color map and raster index; font selection also requires a color map. Missing color maps show unavailable instead of substituting a normal map or raw template. Preview query caches include the asset roots. Automated coverage: nested mesh selection/map exclusion, early skater/goalie filename fallback, and color-raster selection. Read-only real-install probe: crates/studio-api/examples/nhl07_uniform_probe.rs; confirms equipment submeshes and separate team color-map paths on NHL07. User verification pending: open two teams' home/away kits, confirm equipment geometry, compare nameplate thumbnail/editor against the selected team's color texture. This does not qualify every game's mesh shapes, gear variants, or runtime asset replacement.

Equipment preview follow-up (OBS-08): NHL07's separate native left/right glove, sock and skate meshes are now assembled as paired preview items (both sides share visibility/focus). The sock side selector is the third numeric token, not its cut/tuck token. NHL07's prepainted glove/helmet color maps replace the later-title hHbv composition path; glove/skate sides receive their separate color files, helmet uses team art ID and kit variant. Default skate model remains 0 unless a selected-player texture is already resolved. Nine kit-preview unit tests pass. The read-only NHL07 probe confirms each pair contains two textured submeshes with geometry on opposite sides of X=0, and the helmet has a bound color texture. Logs: target/nhl07-equipment-pairs-probe.log and target/equipment-pairs-tests.log. User visual/in-game acceptance pending; this is not qualification of every equipment model or non-NHL team's texture coverage.

### Resuming package work

When the user requests resumption, first reconcile the observation log and any unresolved regressions. **DB-07 — Game Settings and Executable Tunables** is the next planned package; obtain direction rather than starting it automatically during this review pivot. Subsequent packages are DB-08 Player Creation, DB-09 Team Creation, AA-01 Player Art, AA-02 Team/League/Arena Art, AA-03 Hockey Shop, AA-04 General Asset Tools, AA-05 Appearance Studio, AUD-01 Audio and LOC-01 Localization. Asset dependencies may require revisiting integration checks later without losing prior accepted evidence.

For a newly assigned package, retain the established workflow: complete implementation and available automated verification, then provide one consolidated user checklist with exact steps and explicit limitations. Rebuild/relaunch only when needed; preserve open project work and confirm the executable is released before replacement.

## Execution and verification

User-directed execution policy (2026-10-06): when assigned a work package, complete its implementation and automated verification for the active game/platform before returning a single consolidated user verification checklist with exact steps. Do not hand back individual implementation checkpoints. Carry accepted evidence forward and identify explicitly deferred work.

### Accepted separation of mod projects and save editing

Mod Studio projects target the game's default roster database and game assets. Save files are not a second editing target, project requirement, synchronized mirror, or build/deployment output for Mod Studio projects, including previously paired projects. The interface presents one database view. This decision supersedes earlier requirements to attach or synchronize a roster save in the main Studio workflow, including the NHL07 difflist project-integration step below.

The launch screen offers Mod Manager, Mod Studio and a separate Save Editor. Save Editor is scoped initially to basic, individually qualified save edits; it does not edit the installed game database or synchronize it with a save. NHL07-style difflists still require a matching game database as a read-only decoding baseline. The verified NHL07 jersey/speed reader/writer and native fixtures belong to that tool's implementation foundation. Its broader features remain separately scoped.

Implementation must update project creation/import, open/reopen, edit routing, save/build/deploy and presentation together. Existing project save files and saved edits must be preserved; adopting the new purpose must never silently delete them or continue mirroring through an old open path. Keep explicit transfer/import between tools separate from automatic synchronization.

Default-database editing must be qualified game by game. Existing saves can retain or override default roster values, so native tests must establish which fresh/default roster state loads the modified database. The current PS4 loader edits a standalone roster save and treats `nhl_game.db` as a template; changing that target to default-database modding requires identifying and verifying the actual game-loaded source rather than rebranding the save or removing its guard. A common database ancestry does not establish identical schemas or mode-state coverage for all saves.

#### Revised implementation sequence

Latest direct-database export verification: the user confirmed Demitra on Vancouver #90 with `local/nhl07-demitra-vancouver90/NHL07NG.db` (SHA-256 `bb701967d7be1ec932fc322c7607f75ce4b7e1ff54cda4081723231530252591`). This file was saved through the Studio project editing API, reopened, checksum-verified and exported from `NHL07 Studio Workflow`. Native acceptance of this exact exported database is confirmed. No package was created for this #90 export; the earlier #91 package test remains separate evidence and does not finalize the overall project/build system. Visible UI interaction remains unverified.

Native verification confirmed by the user (2026-10-05): after launching the extracted NHL07 Xbox 360 installation and selecting Restore Default Rosters, Demitra appeared on Vancouver wearing #90. This validates loading the edited loose database and applying its modified default roster in this installation. The preceding Minnesota #38 result came from a disc-image launch and does not invalidate the extracted-folder result. Speed was not edited. This does not establish loose override behavior for every packed asset or distinguish stock/default from exhibition table usage.

1. **Fresh Mod Studio projects — implemented, verification below:** copy only default database/metadata and localization components; omit roster saves and auxiliary save files, save destinations, save inputs and save-only baseline-gap warnings. Ignore obsolete save arguments from older clients. Require a main database and metadata. Fresh projects therefore use a single database session on initial open and reopen. PS4 templates retain their existing restrictions; no platform's native default-database write support is newly claimed.
2. **Existing projects and imports — implemented:** existing paired projects open the game database alone; original save files, baselines, manifest references and history remain preserved. Missing retained saves do not block database work. Save edits are excluded from new packages and deployments; relinking ignores obsolete save arguments. Save-only imports and projects are refused with a Save Editor explanation without altering their files. Imports accept game databases, localization and assets.
3. **Presentation and tool entry — implemented:** removed database/save switching and save input/destination controls from Mod Studio onboarding and project settings. Added Save Editor alongside Mod Manager and Mod Studio, with an under-construction page. Full editing remains step 5; the entry does not claim functional save editing.
4. **NHL07 default roster qualification — native verified for the tested transfer/number workflow:** the user confirmed Vancouver #91 after installing the production-built exhibition-only package and selecting Restore Default Rosters. Editing exhibition bio and roster tables is sufficient for this path; stock/default copies remained vanilla and require no mirror for these tested operations. Transfer, number change, undo/redo, save/reopen, build, independent install and restore passed automated checks; the actual installed build passed the native game check. Desktop UI review, other edit types and other titles remain separate qualification tasks.
5. **Separate Save Editor coverage — first implementation 12 Oct 2026, automated-tested only (not game-verified):** see [save-editor-2026-10-12](../validation/save-editor-2026-10-12.md). Opens roster/mode saves and extracted databases with automatic game/platform identification; tables a save lacks are shown read-only from the bundled anchor database; NHL 07/08 change lists remain refused. Original deferred scope, retained for history: integrate and extend the verified NHL07 jersey/speed paths under a save-specific session with a read-only database reference, independent undo/save/reopen, and explicit supported fields. Qualify biography, goalie and structural edits separately.

Step 4 experiment prepared (2026-10-05): Demitra (bio key 613) moved from Minnesota (14) to Vancouver (28), jersey 90, in a separate imported project `E:\NHL Modding Studio - QA\xbox360\nhl07\Demitra Vancouver 90 - Native Test`. The ordinary transfer operation initially refused because it assumed a fixed destination roster block; the later workflow update below records its correction. The focused `crates/studio-api/examples/nhl07_default_roster_probe.rs` changes eight keyed fields across exhibition/stock bios and exhibition/default rosters, verifies CRCs, imports the artifact, and uses Studio's backed-up deployment. Both copies are intentionally changed; this experiment does not distinguish which copy Restore Defaults reads. Original QATest remains unchanged. Deployed loose `F:\NHL Installations\Xbox 360\NHL07\NHL 07 (USA)\db\NHL07NG.db` has SHA-256 `50851d694a431262a5a9278cce88f987b9b597871349cd464db55a03ca2bf3cb`; original `67a00beaa05690d30527c9059146c7e610f33f892b0ddadc381e1cfefc8dbad3` is backed up in the test project's `deployments/1/backups/000-NHL07NG.db`. Reopen resolves Vancouver #90 and retains Slovakia #38. Packed containers and emulator saves were not edited. **User-confirmed native result:** launching the extracted folder and selecting Restore Default Rosters produces Vancouver #90. This first experiment verified loose database loading but did not isolate table roles or qualify a released build; the subsequent exhibition-only workflow below completes those checks for the tested edits. The imported edited artifact was its own import baseline.

Step 1 verification: 15 backend tests (`project_workspace` and `project_resume`) and 24 onboarding tests passed. Coverage includes all 26 available baseline targets opening without an attached save (NHL20 PS4 remains unavailable), database-only save/deploy isolation, old save arguments ignored, and omission of save-only onboarding notes. At step 1 completion, native Restore Default Rosters and desktop review were pending; see the step 4 result above; no existing user project or emulator save is migrated by this step.

Step 4 workflow update (2026-10-05): 40 automated tests passed: `tdb-edit` NHL07 transfer (1), repairing transactions (6), cluster operations (10), later-game field operations (17), plus `studio-api` NHL07 workflow/atomic replay guards (2) and package round trips (4). The new `NHL07 Studio Workflow` project was created from bundled vanilla anchors and edited through the production Registry APIs. Its released build `E:\NHL Modding Studio - QA\Builds\Demitra_Exhibition_Test_xbox360_nhl07.nhlmod` carries four explicit main-database cells: exhibition bio and roster team/jersey only, Demitra → Vancouver #91. Stock/default copies remain vanilla Minnesota #38. Prior test deployment 1 was restored through Studio before installing the new build. Installed DB SHA-256: `e24e040753d2484be8f47f17d84a1966725d6b50668f0ca3be8bcd8f909664f6`; rollback record: `E:\NHL Modding Studio - QA\xbox360\nhl07\NHL07 Studio Workflow\native-installs\1`. **Native result confirmed by the user:** Restore Default Rosters produced Vancouver #91. Exhibition bio/roster changes are sufficient for this workflow with stock/default tables untouched. This establishes sufficiency of the paired exhibition edits, not independent necessity of each individual field/table or coverage of other game modes. The visible Studio UI has not yet been manually verified with the rebuilt backend; headless checks use the same production API.

Steps 2–3 verification: 25 backend tests passed (`project_inputs`, `project_resume`, `project_workspace`, `mod_package`), plus 27 launch/onboarding tests and TypeScript checking. The legacy-paired regression verifies preserved save bytes through edit/save/build/deploy/reopen, exclusion of save-only edits from package inventory, database-only package cells, reopening with the retained save missing, and refusal of save-only projects without changing their files. Package round trips verify that new database builds leave recipient saves unchanged, including Xbox-authored Legacy builds installed onto PS3 and Recompiled test installations. Existing package formats with explicitly authored save edits retain installer compatibility. Native NHL07 Restore Default Rosters is now confirmed by the user as recorded above; desktop visual review remains pending; tests use owned temporary installations and do not modify user saves.

Start with **MS-01.1 Project/Build functionality**, establishing existing behavior before adding missing pieces. Package identifiers below remain stable; numerical order is an inventory, not permission to start all packages. The user selects subsequent focus. Dependencies can be implemented as narrow prerequisite tasks without expanding the active assignment into an entire other package.

Track each subfeature for a specific game/platform through **Not assessed → Specified → Implementing → Automated checks passed → Application verified → Native verified**. Record blocked and deferred checks separately. A passing lower-level check never implies native acceptance. For operations with no in-game effect, native verification is not applicable with an explicit reason. Different operations within a feature can occupy different states.

Each task records its outcome, exclusions, target, current state, tests possible now, deferred verification, exact dependency, evidence and next action. Finish or identify the concrete blocker for one task before changing focus. A completed task is not a completed work package. Evidence gates do not introduce routine permission checkpoints.

All applicable work retains the accepted 27 targets: Xbox 360 NHL 07–15 and Legacy (10); PS3 NHL 08–15 and Legacy (9); PS4 NHL 15–21 (7); Legacy Recompiled (1). NHL 07 PS3 remains research-only. No Xbox One target. Windows is the initial desktop platform. Internal milestones do not claim release readiness or qualification of sibling games.

Every editing feature includes relevant validation, pending/saved state, undo/redo behavior, persistence across reopen, Project Overview integration and a path back to its editor. Bulk editing and import/export are explicit per-operation capabilities where applicable. Verification must cover failures, source preservation and target-specific limits, not just the happy path.

## MS-01 General Project and File Management

**Acceptance: PASSED — user sign-off.** The initial NHL07 Xbox 360 project/build milestone is closed, including the final imported-project edit/rebuild, Overview and native install/restore acceptance checks. **Next focus: MS-02, NHL07 onboarding.** Previously verified extraction and source-selection behavior carries forward. Other games/platforms, future editor integration and the documented legacy save-mirroring test debt remain separate work; this sign-off does not qualify them. Earlier pending notes below are superseded for this milestone.

**User verification update:** importing a `.nhlproject` through the new **Import Project Package** entry successfully restores the project state. Developer-package import/restoration is now user-confirmed; further editing/rebuilding of that imported copy and the remaining Overview/native acceptance checks remain separately tracked.

**Current checkpoint — 2026-10-06, NHL07 Xbox 360:** save/lifecycle verification is user-confirmed complete, including the behaviors listed in the final MS-01 review. The user has also confirmed QATest build installation and restoration. The original slow Activate operation completed; its preparation was optimized and the actual 657-edit package now prepares in about 0.7 seconds on the test machine. Native retesting of that improvement remains in the final acceptance list.

Direct `.nhlproject` import is implemented under **Load Project → Import developer project** with a proposed platform/game/name destination, editable destination and folder picker, optional local game source, validation before publication, and no overwrite of existing projects. Imported projects discard the author's live destination links; source relinking remains available in Overview. Export → import → reopen → edit → save → rebuild is covered automatically. Interrupted-save recovery is again accessible through a conditional Overview card, replacing guidance to the removed Pending Edits controls. Changed archive files now have net-change summaries; individual member and asset-owner detail remain deferred to their editors.

**Closure scope:** the initial NHL07 project/build milestone can close after the user's final sharing, Overview and native build acceptance checks in [the acceptance record](../validation/ms-01-final-acceptance-2026-10-06.md). Other titles/platforms and future asset/editor integration remain separate qualification work. Older checkpoint statements below are historical; the acceptance record distinguishes current results, obsolete test assumptions and deferred scope. Do not label the entire multi-game package feature-complete.

**2026-10-06 session persistence correction:** QATest's working NHL07 database and localization file were byte-identical to their original baselines, with no saved-session records or transaction recovery files. The four existing loose logo files matched the linked installation. The user's queued database edits had not reached disk; no recoverable saved copy was found in that project folder. The window X had bypassed project-close handling.

Save Session is now always visible between Project Overview and Close. Both the Close control and native window-close request prompt to save and close, discard explicitly, or cancel; failed saves and refused backend closes keep the session open. A 10-minute autosave default persists in user preferences and is configurable from 1–1440 minutes in Settings → General. Autosave saves changed project sessions through the normal project-save path, avoids overlapping session saves, pauses during the close dialog, reports failures and retries on the next interval. This is periodic saving, not crash recovery of edits made since the last save. Evidence: 23 focused UI tests, TypeScript checking, and a successful desktop build; native close-prompt and timed autosave user verification remain pending.

Desktop lifecycle check: a sandbox-launched instance remained alive without a titled Studio window or Studio WebView2 children. Its creation time proved it was a stalled new launch, not an old session. Relaunch outside the restricted environment created the titled window and WebView2 processes. After the user closed that window normally, the app and all observed children exited; zero `nhl-studio` processes remained. No shutdown leak was reproduced in this test. Future launch confirmation must check the actual titled window and WebView2 startup, not just process existence. Closing during active operations remains a separate, untested case.

1. **Project/Build functionality:** create, open, save and reopen projects; one user-selected NHL Project folder shared across games. New projects use `<root>/<platform>/<game>/<project name>`. Released builds use `<root>/Builds/<build_name>_<platform>_<game>.nhlmod`; one BUILD action produces a separate package for each valid platform of the same game. NHL 07 is Xbox 360 only; NHL 15 PS4 is a separate game-generation target from NHL 15 PS3/Xbox 360. Authoring intent is platform agnostic; builds perform target-specific formatting. Remember storage preference and validate names/collisions. Changing storage does not move existing projects. Identify game/platform/source/baseline; isolate working files from originals; build saved changes for the selected target. Distinguish a saved project, distributable package and deployed game output.
2. **Project Overview:** cumulative differences against the original project baseline, including saved work; category/entity hierarchy; before/current field values; asset summaries; deep links; distinguish unsaved session changes.
3. **Package management:** export/import packages; inspect contents and target compatibility; explicit omitted categories and conflicts; install/deploy saved builds.
4. **Backup/recovery:** restore baseline or prior deployment; recover interrupted save/build/install; detect conflicting external changes and preserve unknown bytes.
5. **File/archive management:** browse/search/inspect/extract; replace/add supported members; preserve required metadata during rebuild. Track disc patching and full rebuilding separately.

**Test now:** project isolation, manifest persistence, save/reopen, package contents and failure behavior using existing supported edits. **Deferred:** full package coverage and Overview integration for each future database/asset/creation workflow; native output acceptance per target. Do not make the first project task depend on implementing every editor.

### Project Overview contract

**2026-10-06 build/sharing implementation:** BUILD and Package Dev Project Files now save the form metadata and current project edits before producing outputs. BUILD includes all currently packageable edits and produces one `.nhlmod` per compatible target under `<projects root>/Builds`; existing filenames get a numbered build-name suffix. Unsupported content blocks the build rather than being silently omitted; incompatible platforms are reported. NHL 15 PS4 is excluded from automatic PS3/360 builds (and vice versa). Packages retain the author platform as provenance while declaring only their destination platform, with the installer applying the portable edits to that target's files. Name, version, author and description travel in the mod manifest.

Package Dev Project Files writes a ZIP64-capable, compressed `.nhlproject` under `<projects root>/Project Archives`, preserving project file contents, baselines, working files, sources and history, excluding active project/transaction locks. It streams files, verifies the archive against source identities, detects external changes during packaging, and refuses linked files/folders rather than silently losing their contents. Existing archives are preserved. Automatic `.nhlproject` import/relink UI remains a separate step; standard ZIP extraction followed by Load Project is tested.

**Build/sharing evidence:** 9 API tests (new release/archive tests plus package/install and NHL07 workflow regressions), 3 package-format tests, 9 UI tests and TypeScript checking passed. New tests cover NHL07 single-target output, NHL15 separate PS3/360 outputs with PS4 exclusion, author/version preservation, repeated builds without overwrite, rejection of unsupported content and unsaved edits, and archive extraction → reopen → additional edit → save. UI tests verify save-before-package and no build after a failed save. Native testing of the user's new NHL07 edits remains pending.

**2026-10-06 presentation pass:** replaced the old Pending Edits panels with Project Overview, persisted Name / Version / Author / Description, and a collapsed league → team → player/category change tree. The database comparison is original project baseline → live working data, including saved edits; saving and reopening do not clear it, and returning to the original value removes it. Known player edits link to their data, attributes or equipment tab; known team and league changes link to their entity editor. Unmapped database changes remain visible. Loose project assets use their recorded original identity to filter unchanged outputs; unresolved asset ownership is shown under Shared assets. Localization changes use original/current text. Archive-member and settings detail integration is explicitly incomplete and disclosed when such edits exist.

**Presentation checkpoint (superseded by the build/sharing implementation above):** implement BUILD as one action producing separate target-formatted `.nhlmod` archives for supported same-game platforms, then implement Package Dev Project Files as a complete portable `.nhlproject` archive (ZIP internally is acceptable). Both controls were visible but disabled at that checkpoint. Do not treat the previous multi-platform declaration in a single package as completion of this contract. The metadata display name does not rename the existing project folder. Full entity ownership of art/audio assets, transfer references under both original/current teams, precise field focus and summaries for future editors remain integration work; this pass does not qualify them.

**Verification for this pass:** automated NHL07 baseline → transfer/jersey edits → save → reopen → return-to-baseline test, metadata persistence without folder rename, UI collapsed hierarchy/editor navigation and disabled packaging controls, and TypeScript checking. The user has since verified player-name edits and manual reversion in the Overview; the top badge was corrected to use the same net-change query. Broader layout review remains ongoing.

The Overview replaces Pending Edits as the cumulative review surface. Keep original-baseline comparison separate from last-save comparison: saving clears the unsaved marker, not the cumulative change. Reverting a field to its original value removes that net difference. Repeated edits appear as one original/current change. Represent added/deleted entities and replaced/added/deleted files explicitly. The baseline must survive save, close and reopen.

Start collapsed at the highest meaningful level:

```text
NHL — changed team, player and asset counts
  League changes — identity, localization, imagery, settings
  Pittsburgh Penguins
    Team changes
    Team assets
    Players
      Sidney Crosby
        Player data — expandable original/current fields
        Roster and contract changes
        Appearance and equipment
        Player assets — portrait, head, hair, audio
  Anaheim Ducks
    Team changes / Team assets / Players
Other leagues
Free agents and unassigned players
Shared assets and global settings
Other file changes
```

Use compact summaries drawn from the same labels, entity data and asset descriptions as the editors. Clicking a change opens its precise entity/tab/field or asset slot. Preserve useful navigation context. A transfer records original and current teams, with references under both and one underlying change. Multi-team assignments follow the same no-double-counting principle. Shared assets appear once with affected-entity references. Do not infer ownership only from filenames. Deletions need a retained summary when the live entity no longer exists.

Keep exact totals when details are truncated or loaded on expansion. Unknown/unmapped changes remain visible under Other file changes rather than disappearing. Source provenance and saved/unsaved indicators must remain accurate after undo, save and reopen. Build/install state is separate from whether a project change is saved.

**Staged verification:** controlled data can prove hierarchy/counts/navigation only. Actual database summaries wait for verified corresponding DB operations; textures/meshes/audio wait for their editing and save paths; created entities wait for DB-08/09; historical package summaries wait for DB-05 integrations. Track each category independently. Until wired and checked, label its integration deferred rather than claiming full Overview coverage.

## MS-02 General Onboarding

**Acceptance: PASSED — user sign-off, 2026-10-06.** All 14 NHL 07 Xbox 360 onboarding checks are confirmed, including corrected ISO error handling and cancellation/retry. MS-02 is closed for this target. Other games/platforms and additional region references require separate qualification. Next work package: MS-03 Game Anchors and Format Handling. Historical checkpoints below are superseded by this acceptance.

Entry flow: first screen offers Mod Manager, Mod Studio and Save Editor. Save Editor currently shows an under-construction notice; its implementation is separate. Mod Manager shows an under-construction notice and houses the existing build installer; full mod management is not yet scoped. Mod Studio offers Create New Project, Load Project and Import non-studio project. Entering any Studio action without configured storage prompts for the default NHL Project folder. Game selection has a Back button and no project name/folder fields; those appear on Project Setup. Loading shows existing projects separately. Existing flat-layout projects remain loadable without migration.

Current target focus: NHL 07 Xbox 360, starting from ISO and a fresh emulation environment. Fresh Project Setup contains the name, browsable per-project folder override and one game-source selector. A popup configures ISO → extraction destination or an existing dumped folder; setup summarizes the selection and permits changes. Main Next dumps the ISO before creating a project from bundled vanilla anchors. Existing-file editing stays in Load/Import. PlayStation-specific installation onboarding is deferred. Extraction and loose-file override behavior require separate native qualification; successful extraction does not prove override precedence.

**Current checkpoint — Application verified, 2026-10-05:** the user confirmed NHL 07 Xbox 360 ISO extraction is functional, selecting an existing dumped ISO folder works and advances onboarding, and project creation works after the missing platform/game parent-folder fix. The current Xbox 360 onboarding work is accepted for this target. This supersedes earlier pending desktop-verification notes for these paths. See the [verification record](../validation/ms-01-project-build-2026-10-05.md#nhl-07-xbox-360-onboarding-application-verification).

The checkpoint includes the implemented setup/navigation refinements, project name/folder handling and both source routes; automated coverage supports validation, cancellation and collision guards. Native emulator boot, loose-file override precedence, roster-save compatibility and later editing/build workflows remain separate checks. Other Xbox 360 games and PlayStation routes are not qualified by this NHL 07 result. MS-01 and MS-02 remain broader work packages with unfinished scope.

**Subsequent task — Automated checks passed, 2026-10-05:** NHL 07 USA original-disc manifest and read-only vanilla checker implemented, including region notes and acknowledgement-based continuation. Real installation check matched all 292 required disc files (281 loose, 11 BIG containers). The new warning/verification UI awaits user application review. The two unparsed containers remain the next separate investigation. See [manifest evidence](../validation/ms-02-nhl07-file-manifest-2026-10-05.md).

**Speech-container follow-up, 2026-10-05:** the two failed inputs are NHL 07's indexed speech banks, `pbp_eng.big` and `rink.big`. IDX/BIG support now routes them through the same guarded extraction service as other containers, producing encoded SNR/SNS pairs. All 11 original containers parse in the read-only installation scan. Extraction verification and limitations are recorded in [speech-container evidence](../validation/ms-02-nhl07-speech-containers-2026-10-05.md); audio playback/editing and in-game use remain separate work.

1. Import from ISO.
2. Import from dumped/extracted ISO.
3. Import from game installation.
4. Import standalone database/save files where supported.
5. Detect game/platform/version and validate required files.
   - Ship per-target, region-labelled original-disc file manifests alongside anchors. Compare original paths, sizes and hashes; distinguish original loose files, containers and additional outputs. Warn for mismatches or unknown references and permit proceeding after acknowledgement. Reference generation must use a reviewed original source, never the selected installation as its own baseline. Track region/edition coverage separately.
6. Locate assets, dictionaries and schema metadata; report missing optional inputs without hiding unrelated functionality.
7. Resume incomplete extraction and repair source bindings.

**Dependencies:** MS-01 project creation and MS-03 format identity. **Deferred:** downstream editor availability and native output validity; a successful extraction is not an integrity certificate or editing qualification.

**Final acceptance, 2026-10-06:** user confirmed all 14 MS-02 checks pass for NHL 07 Xbox 360. ISO error handling and cancellation/retry (7–8) passed after correcting the existing-folder guard and extraction lifecycle. See the [current acceptance record](../validation/ms-02-nhl07-acceptance-2026-10-06.md). MS-01 remains passed; earlier pending notes above are historical.

## MS-03 Game Anchors and Format Handling

**Acceptance: PASSED — user sign-off, 2026-10-07.** All six desktop checks passed for NHL 07 Xbox 360, including rejection and corrected retry. MS-03 is closed for this Mod Studio target. Save Editor and other games/platforms remain separate. Next work package: DB-01 General Player Editing. Earlier pending-verification notes below are superseded.

**Agent verification follow-up:** all six consolidated NHL07 scenarios passed through the production API, including both dictionary routes and the complete jersey save/reopen/revert sequence. All 29 onboarding component tests passed, with explicit error/retry coverage. Native desktop verification could not run because the computer-use kernel failed initialization twice; the user subsequently supplied native interaction/presentation acceptance on 2026-10-07. See the acceptance record below for the exact evidence boundary.

**Ready for consolidated user verification — NHL 07 Xbox 360, 2026-10-06:** Mod Studio qualification uses the default game database; Save Editor work stays separate. Anchor identity, both database no-op round trips, damaged-bundle rejection, production edit/build/install/restore, metadata/input rejection and optional-dictionary fallback are covered. NHL07 import and reopen now reject metadata field-layout mismatches. Prior native default-roster results carry forward. No remaining implementation checkpoint in this bounded package; user acceptance was completed on 2026-10-07. See the [MS-03 checklist and instructions](../validation/ms-03-nhl07-acceptance-2026-10-06.md).

**NHL 07 Xbox 360 roster change lists — initial automated checks passed, 2026-10-05:** metadata-driven recognition and a read-only effective-roster projection are implemented. All 31 companion tables map through explicit insert/delete/update declarations; the metadata's `updateshortname` resolves the AI mappings without the adjacency hypothesis in the earlier research. The empty anchor projects to all 16 parent tables byte-for-byte. Synthetic keyed update/insert/delete and malformed-input checks pass; existing main-database editing remains usable. Warnings now identify an unsupported change-list save model rather than incompatible anchors. This is diagnostic support, not enabled roster-save editing.

**Populated-save follow-up — automated checks passed, 2026-10-05:** user-provided ROSTER1 and ROSTER2 contain 27 and 59 paired roster deletions/replacements respectively. The user confirmed Pavol Demitra was moved from Minnesota to St. Louis and changed to #40 (correcting the initially reported #38). Stable identities are bio game_id 613 and roster playerindex 587; the saved roster changes team 14 to 24 and jersey 38 to 40. Delete-first/insert-second projection now handles these replacements regardless of directory order. Populated companions have allocation type 6 and max=current; rebuilding both game-produced inner databases from the empty anchor reproduces their exact bytes. Compact fixtures under `crates/studio-domain/tests/fixtures/nhl07` retain the original headers/payloads, with verified zero tails reconstructed and full-file SHA-256 checked by tests.

An experimental NHL07 jersey writer now preserves existing roster replacements, consumes the zero-tail allocation rather than growing the outer file, and rebuilds inner/outer checksums. Automated round trips cover empty and populated saves, unrelated-edit preservation, unchanged file length, no-op identity and rejection cases. It is not connected to Studio save editing. No snapshot-style record-index mirroring is admitted; NHL08 qualification is separate.

**Native verified — NHL07 existing roster replacement jersey edit, 2026-10-05:** the user loaded the generated ROSTER2, confirmed Demitra on St. Louis wearing #38, and saved ROSTER3. Read-back confirms ROSTER3 is byte-identical to the generated file: 1,598,888 bytes, SHA-256 `8d95de096f9c86a0d6cda78ec30d65667965837db4ca9c9d2723d00c69612cd1`. A compact ROSTER3 fixture and exact writer-versus-native-output regression preserve this evidence. This qualifies the existing replacement-row jersey write and load/re-save path; it does not qualify every difflist operation or production editor integration.

**Native verified — NHL07 stock-skater speed update:** a separate Studio-generated ROSTER3 test copy seeds the metadata-declared `RosT` update companion from Demitra's effective skater row (game_id 613) and changes only `p_speed`/`YWPq` from raw 42 to 49. The user subsequently located the limited in-game Edit Player section and confirmed Speed displays 99, correcting the earlier report that no editor existed. Re-saved ROSTER4 retains the same update row, St. Louis assignment and jersey #38. Its complete 1,598,888-byte file is byte-identical to the Studio-generated test: SHA-256 `f3352c3198d30c61c4be3960a89f5ad6ccf5b57c508007bb5459f4d4704254fb`. The compact game-returned fixture is covered by an exact writer-output regression alongside ROSTER3. Native load, displayed effect and re-save are now verified for this stock-skater update case; other attribute tables are not thereby qualified.

**Reassigned to Save Editor:** integrate the verified keyed-overlay read/write paths with the separate save-editing workflow, pending changes, undo/redo, transactional saves and reopen. These are no longer prerequisites for Mod Studio default-database projects. Biography updates, goalie attributes, created-player/deletion semantics and broader writer coverage still need separate qualification. The existing snapshot-based EditSession/Workspace path must not be enabled for difflists merely because these narrow writes passed. No further user action is needed for the completed jersey/speed verification cases.

1. Register/identify baseline files and game/platform/version variants; verify identities and required companions.
2. Save files: extract/edit payloads; rebuild containers/checksums; resolve signing/encryption requirements; verify native loading.
3. Internal game databases: load/save sets and metadata; preserve related-file relationships.
4. Probe capabilities from actual inputs; distinguish unavailable, unsupported and unverified operations.

**Test now:** identity, parser boundaries and format round trips. **Deferred:** signing/encryption and native game loading until the particular runtime and output path are available. Shared parsers do not qualify every target.

## DB-01 General Player Editing

1. Name, date/place of birth, state, country/nationality; height, weight, handedness, jersey number.
2. Primary/secondary position, player type/archetype, attribute ratings and supported potential fields.
3. Draft information, rookie flags and active status.
4. Primary/secondary team membership, international/other-league assignments, professional/junior rights and free agency.
5. Contracts: supported salary, term and eligibility fields.
6. Existing-player operations only. Record creation, duplication, deletion and their structural package support belong to DB-08 (user decision, 2026-10-07). Release/sign remains in DB-01, including NHL07's explicit free-agent list and package support (user decision, 2026-10-07).

**NHL 07 Xbox 360 status: PASSED — user sign-off, 2026-10-07.** All DB-01 verifications passed. Existing-player DB-01 is closed for this target; record lifecycle remains DB-08. See [DB-01 acceptance and step-by-step review](../validation/db-01-nhl07-acceptance-2026-10-07.md). MS-01, MS-02 and MS-03 also have user sign-off for this target. Next work package: DB-02 General Team Editing.

**This title's supported surface:** biography, physical details, primary position, handedness, jersey, nationality, skater/goalie ratings and archetype, draft year/overall pick, rookie flag, salary/term/two-way contract, club transfers, additional memberships, release and signing. Package schema 3 carries the explicit free-agent list by player ID; schema-2 packages remain readable. Native free-agency acceptance is still pending.

**Not represented in this title's main player schema:** separate secondary position, active flag, potential range/accuracy, draft team/round, professional/junior rights, UFA eligibility and separate AHL/CHL rookie flags. Do not add guessed fields or treat them as unimplemented NHL07 controls. Other titles require their own qualification.

**Dependencies:** MS-01/03 and entity/reference validation. **Deferred:** native interpretation of fields, all game-mode behavior and cumulative Overview summaries until those integrations are verified.

## DB-02 General Team Editing

1. Name, location, arena assignment, roster slots and colors.
2. Transactions: transfer players and draft picks; preserve rights and memberships correctly.
3. Lines, depth charts, dressed/scratched status, captain/alternates, jersey numbers/conflicts and strategies.
4. Team record creation/duplication/deletion where supported, with explicit capacity/reference limits. Native custom-team creation is DB-09.

**Dependencies:** DB-01 relationships and MS-03 schema handling. **Deferred:** strategy effects, game-mode eligibility and structural limits require native checks.

**NHL07 Xbox 360 status: PASSED — user sign-off, 2026-10-07.** All original and follow-up DB-02 checks passed, including Windows drag-and-drop, immediate edit feedback, Overview database reverts, save/reopen, build/install and native game acceptance. See [DB-02 acceptance record](../validation/db-02-nhl07-acceptance-2026-10-07.md). Supported existing-team DB-02 is closed for this target. Stock team colors, draft-pick assets and retired-number records are absent from this default database. Team record lifecycle remains unqualified; native creation remains DB-09. File/archive/structural reverts and other games/platforms require separate work. Next work package: DB-03 Advanced Player Editing.

## DB-03 Advanced Player Editing

**NHL07 Xbox 360: PASSED — user sign-off, 2026-10-07.** Supported existing-player assignments are accepted and DB-03 is closed for this target and scope. 13 backend and 13 selected frontend checks passed before user review. [Implementation roadmap](db-03-nhl07-implementation-roadmap.md) · [Acceptance record, evidence and limitations](../validation/db-03-nhl07-review-2026-10-07.md). Generic/bespoke classification, full gear/audio file lookup, asset authoring and other games/platforms remain outside this acceptance. Next work package: DB-04 Advanced Team and League Editing.

1. Equipment assignments and supported appearance parameters.
2. Portrait/head/audio IDs and generic/bespoke associations.
3. Shared versus unique assets; missing-reference validation.

**Test now:** stored assignments and reference validation. **Deferred:** visual/audio results depend on AA-01/03/05, AUD-01 and native checks. Assignment success is not asset compatibility.

## DB-04 Advanced Team and League Editing

**NHL07 Xbox 360:** organization, league swaps, calendars, CSV/generation and structural schedule packaging implemented, 2026-10-08. **PASSED — final user sign-off, 2026-10-11.** All review checks and four presentation/CSV refinements passed. DB-04 is closed for the verified NHL07 Xbox360 scope. [Roadmap](db-04-nhl07-implementation-roadmap.md) · [Verification steps, evidence and unmapped features](../validation/db-04-nhl07-review-2026-10-08.md). Naming, presentation assignments and affiliations remain dependent on mapping work; native checks in the supplied review are accepted by user report; broader runtime scenarios remain unqualified.

1. Team league/conference/division assignments and affiliations.
2. League/conference/division identities and structure.
3. Team Schedule subtab with a monthly calendar, opponent logos/names and Home/Away labels. Click a date to add/edit/remove a shared fixture; all affected opponents' calendars update automatically. League-level CSV export/import with preview, plus schedule generation, use the same fixture model.
4. League imagery/commentary assignments.
5. Expansion constraints and required supporting records, separate from native custom-team slots.

**Dependencies:** DB-02, LOC-01, applicable AA/AUD work. **Deferred:** schedules, competition structure and expansion acceptance in the relevant native modes.

## DB-05 Historical Database and Roster Building

**NHL07 Xbox 360: PASSED for current non-asset historical database workflows — user sign-off, 2026-10-11.** User confirmed all current historical database functionality passes. DB-05 is closed for that scope. Historical asset portions are explicitly deferred to review with DB-08, not marked passed. New-player record lifecycle remains DB-08. Next work package: DB-06 Arena Editing.

1. Browse players, teams and seasons; import/update/validate the corpus without losing prior valid data or silently dropping rows.
2. Identity matching, duplicate policy, season provenance and import history.
3. Import one player: data, ratings, portrait, head and supporting assets; update an existing instance or create one.
4. Import an entire team: players/assets; automatic captaincy, lines, numbers and dressed/scratched status; preview conflicts.
5. Ratings formula editor: conversion profiles, normalization, archetypes, previews/comparisons and selective recalculation. Separate mathematical correctness, statistical calibration and game behavior.
6. Legends/all-time teams: cross-season selections, franchise assignment, duplicate-player policy, review and import.
7. Historical appearance/equipment packages: store generic/bespoke head, hair/facial hair, skin/eyes, equipment selections/colors and referenced custom assets. Support season/era variants and defaults.
8. Import the selected appearance/equipment variant with the player; preview dependencies, incompatibilities, substitutions and omissions; record provenance and show resulting Overview changes.

**Accepted:** current non-asset historical database workflows, by user verification. **Deferred to DB-08 review:** historical portraits, heads, supporting assets, appearance/equipment variants and their import, Overview, packaging and native acceptance. These checks retain dependencies on AA-01/03/05; historical asset availability is not assumed. New-player creation remains part of DB-08.

## DB-06 Arena Editing

**NHL07 Xbox 360: PASSED — user sign-off, 2026-10-11.** Supported stock-arena editing, shared-team ownership, assignment dropdowns and project/package integration are accepted. All nine supplied review checks passed; DB-06 is closed for this scope. [Implementation roadmap](db-06-nhl07-implementation-roadmap.md) · [Review checklist and limitations](../validation/db-06-nhl07-review-2026-10-11.md). New arena creation and asset authoring remain unsupported. Native effects covered by the supplied review are accepted; untested assignment combinations and modes remain unqualified. Next work package: DB-07 Game Settings and Executable Tunables.

1. Name/location/capacity/ice type and team relationships.
2. Presentation, art and audio assignments.
3. Create/duplicate arenas only where supported.

**Dependencies:** MS-03; DB-02, AA-02 and AUD-01 for integration. **Deferred:** native venue geometry/presentation, audio and mode compatibility.

## DB-07 Game Settings and Executable Tunables

1. Catalogue menu settings, stored defaults, competition rules and database/archive CSV settings; verify load timing and affected modes.
2. Investigate executable configuration readers, constants, tables and consumers beyond exposed menus. Distinguish active paths from debug/unused/obsolete code.
3. AI research: decision-making, positioning, offense/defense, passing, shooting, checking and goalies. These are research objectives, not promised controls.
4. Define candidate meaning/type/default/range/units, dependencies, interactions and version identity.
5. Edit external configuration where supported; treat executable patches as separate, version-bound operations with original-byte validation and restore.
6. Controlled baseline/modified behavioral comparisons; document side effects and reproducibility.
7. Expose qualified controls; include configuration changes and executable patches in Overview and build/install review.

Track each candidate **Located → Referenced → Runtime-observed → Effect verified → Editing qualified**. **Deferred:** static references do not prove active use; controls require an application method, runtime access and effect verification. Patching depends on MS-01 deployment/recovery. No guessed AI controls enter the qualified UI.

## DB-08 Player Creation Suite

Scope includes new-record creation, duplication and deletion, supporting references, capacity checks, and portable structural packaging. These operations were explicitly moved out of DB-01 on 2026-10-07.

**Historical asset review checkpoint (user direction, 2026-10-11):** Review DB-05's deferred historical asset portions here alongside player creation: portraits, heads, supporting assets and stored appearance/equipment variants. Verify import dependencies, Overview, save/reopen, build/install and native results. Coordinate with AA-01/03/05; unavailable dependencies remain explicitly unqualified rather than inheriting DB-05's non-asset sign-off.

1. Identify and allocate native empty created-player slots; preserve occupied slots, report capacity and handle exhaustion. Distinct from arbitrary row insertion.
2. Biographical/physical data, positions, ratings, player type, draft/rookie information and applicable roster/contracts.
3. Appearance Studio integration (AA-05).
4. Hockey Shop integration (AA-03).
5. Validate required records/references; save/reopen; native player recognition and mode eligibility; show Added player and nested details in Overview.

**Test now:** slot identification, allocation and basic data where supported. **Deferred:** appearance/equipment integration until AA-03/05; final native save and mode qualification per target.

## DB-09 Team Creation Suite

1. Allocate native empty create-a-team slots; preserve occupied teams; track capacity and supporting records. This does not imply unlimited league expansion.
2. Reproduce the target game's in-game custom-team creation: name/location/abbreviation/colors/presentation, roster/captaincy/numbers/lines/strategies.
3. Custom jersey selection/design: native layouts, colors/patterns/crests/numbers/lettering; import new layouts; coordinate jerseys/pants/socks across supported uniform slots.
4. Equipment colors/assignments and Hockey Shop/goalie-equipment integration.
5. Existing/custom logos and required uniform/menu/scoreboard/presentation variants.
6. Arena customization: native options plus supported identity/ice/boards/jumbotron/venue assets.
7. Goal horn selection/replacement and supported additional audio.
8. Player Creation/historical roster integration; validate/save/reopen; native recognition; Added team summary in Overview.

**Test now:** slot and identity behavior. **Deferred:** design suite depends on DB-02/06, AA-02/03, AUD-01; player integrations on DB-05/08. Qualify team availability separately in each relevant game mode.

## AA-01 Player Art Assets

1. Portrait export/replacement.
2. Headmesh export/replacement.
3. Donor morphing and authored meshes as separate workflows; materials/textures/skeleton/weights/animation compatibility.
4. Player audio export/replacement using AUD-01 services.
5. Preview/validate before applying, with shared-asset ownership handled explicitly.

**Deferred:** preview/export does not qualify rigged round trip or native animation; packaging, install and native visual/listening checks are required.

## AA-02 Team League and Arena Art Assets

1. Team logos and every required schedule/transaction/menu/scoreboard/uniform variant.
2. League/conference/division imagery.
3. Team equipment colors and assignments.
4. Kit suite: import/export/add jerseys, pants and socks; jersey layout authoring integration for DB-09.
5. Team goalie equipment assignments.
6. Jumbotron replacement.
7. Ice editing: center logo, advertisements and differing layouts including European leagues.
8. Board advertisements and other supported venue graphics.

**Dependencies:** AA-04 codecs/file operations, entity assignment and MS-01 outputs. **Deferred:** complete variant coverage and visual results require each asset consumer to be identified and tested in game.

## AA-03 Hockey Shop

1. Import/export/add sticks, gloves, helmets and skates.
2. Import/export/add goalie helmets, blockers, trappers and pads.
3. Goalie mask editing suite.
4. Preview, texture/material editing, color customization, assignment and slot compatibility.
5. Integration with player/team creation and historical packages.

**Deferred:** new-item registration differs from replacement; verify capacity, references, materials and animation in game. Creation/import integrations wait for DB-05/08/09.

## AA-04 General Asset Tools

1. Browse/search assets outside dedicated entity editors.
2. Texture preview/export/replacement and supported conversions.
3. Mesh preview and supported interchange workflows.
4. Inspect dimensions, mipmaps, compression, materials and references.
5. Track shared assets and affected entities.

**Deferred:** format decode is independent from preservation-aware rebuild and native loading. Qualify each platform layout/compression variant; generic tools do not imply every entity editor is complete.

## AA-05 Appearance Studio

Select any database player; expose controls supported by the target and the selected head configuration.

1. Toggle generic/bespoke head and select the head; preserve or explicitly resolve incompatible settings.
2. Hairstyle/color; automatically update Card A/Card B meshes and required materials/textures.
3. Standard texture facial hair and 3D facial hair through hair cards; investigate adapting later-game hair to earlier games.
4. Skin tone, eye color and other supported appearance properties.
5. Combined preview; save configuration and required assets; avoid unintentionally changing users of shared assets.

**Dependencies:** DB-03, AA-01/04 and MS-01 publication. **Deferred:** assignments can precede asset installation; full visual checks wait for head/hair/texture/material pipelines. Later-to-earlier 3D hair remains compatibility research until native proof per target.

## AUD-01 Audio Editing

1. Goal horns, menu music and in-arena music.
2. PBP review/editing and supported announcements/arena sounds.
3. Bank browsing/playback/export/replacement and reference updates.
4. Shared services for player audio, custom teams and arenas.

**Deferred:** decode/playback does not qualify encoding/replacement, bank rebuild or native take selection. Listen in the relevant game context; preserve format-specific requirements.

## LOC-01 Localization

1. Key glossary/search and dictionary text editing.
2. Find attach points/references for team, league, player, arena and other names.
3. Add/repoint entries where supported.
4. Coordinated entity rename across relevant keys; shared-key conflicts and language variants.

**Dependencies:** dictionary codecs and entity mappings. **Deferred:** automatic propagation requires confirmed attach points; identical text alone is not proof of shared entity ownership. Verify text in the native consuming screens.

## Initial work record

**MS-01.1-A — Establish and harden the existing Project/Build workflow.** Bounded task completed 2026-10-05; package remains open. Inspected project source/baseline/working separation, ran focused backend/frontend checks and repaired stale Build availability. No Overview redesign or downstream editor was started.

Acceptance: record actual checks and limits; preserve source inputs; verify build gating and project save/reopen behavior at the tested seams. Existing fixtures may cover multiple targets, but evidence stays attached to those fixtures. No game is globally qualified by this task. See [initial work evidence](../validation/ms-01-project-build-2026-10-05.md) for results and the next bounded task.

Historical evidence at the initial MS-01.1-A checkpoint: 13 backend tests and 17 frontend tests passed; TypeScript passed. The baseline-loop test opens 26 targets and explicitly expects NHL 20 PS4 unavailable. Later packages were unassessed at that time; their subsequent acceptance is recorded in the current handoff and package sections above.



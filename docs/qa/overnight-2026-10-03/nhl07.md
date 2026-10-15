# NHL 07 QA findings — first pass reviewed, acceptance partial

All twelve requirements have been reviewed for Xbox 360. This is **not an all-green acceptance pass**: runtime verification and the specific unresolved bindings below remain open. The final disposition at the end of this report supersedes earlier chronological checkpoints.

## Verified so far

- Reproduced an actual ISO import failure: the locator searched only for `db/nhlng.db`, but the supplied NHL07 Xbox360 ISO contains `db/NHL07NG.db`. Added the measured earlier filename to loose-member and archive-entry discovery, preserving the actual source location for write-back.
- Five domain tests pass, including the real ISO read/parse/checksum test and synthetic loose/packed NHL07 regression coverage.
- Real ISO: XGD2, 311 members; main database 4,189,776 bytes / 90 tables, SHA256 `67a00beaa05690d30527c9059146c7e610f33f892b0ddadc381e1cfefc8dbad3`; dictionary 182,500 bytes / 3,952 rows, SHA256 `5d3c0b0187b8d673b9642eaec4fabf346fc3451f9a3bf14e4e6bfb4cd61743c3`.
- The Registry ISO workflow stages and opens 2,250 players and 93 teams. Editing the first player's name to QA marks Pending Edits dirty, saves in staging, reopens with the edited name and a valid database CRC. The original database bytes on the ISO remain unchanged.
- Dumped the real disc into an unrelated directory with spaces, then ran the production extraction service. It wrote 15,590 files / 2,150,485,833 bytes. Reopened the resulting database and dictionary through the Registry with matching player/team counts.
- Extraction is **partial**, not a clean all-assets pass: 17 pre-existing differing entries were preserved; two speech banks (`pbp_eng.big`, `rink.big`) are not EA archives understood by this extractor. These failures are recorded in the source-route receipt, not concealed by the successful database-open test.
- Fixed onboarding's imported-assets path: previously an imported packed-assets folder received no extraction offer unless a separate installation was supplied. It now uses the resolved asset root. Fourteen focused frontend tests pass, including the new regression and existing database-only behavior.
- Existing anchor edit-QA passes scalar/name edits, range rejection, database CRC/size, roster checksum and no-op byte identity. Its roster failures were isolated to an incompatible bundled mirror: the 31-table ROSTER1 schema shares zero tables with the 90-table Xbox360 game database. The workspace now rejects unrelated schemas and the baseline inventory omits that save, explicitly disclosing the gap. Main-DB jersey number, captaincy and release work and mark Pending Edits dirty; the regression verifies both those successes and incompatible-save rejection.
- Existing API ISO edit/apply/restore regression suite: 5 pass, 1 private-fixture test ignored. TypeScript check passes.
- Workspace mirror regression suite: 13 pass. Project workflow suite: six cases passed initially; the remaining baseline-open case exposed equivalent incompatible NHL08 save fixtures. Those were explicitly excluded as part of the compatibility guard correction, and that case then passed. This does not constitute an NHL08 game pass.

## Acceptance checklist

| # | Requirement | Current evidence / next work |
|---|---|---|
| 1 | ISO extraction and load | Real Registry stage/edit/reopen passed. Desktop full-disc copying, archive extraction, ISO staging and opening now verified; 93 teams and Anaheim logo/arena/city render using the copied tree. Fixed a second command-layer metadata-name bug found only in the desktop test. Archive results remain partial as described below. |
| 2 | Dumped-disc extraction | Real dumped disc extraction and DB load passed with explicit partial-extraction findings above. Review asset completeness. |
| 3 | Already extracted, arbitrary location | Production discovery and Registry load passed from an unrelated folder with spaces. Desktop onboarding still to verify. |
| 4 | League name/icon | Six league codes resolve; NHL label is inferred, with no traced dictionary name key. Three NHL logo families are present; big variant absent. Logo-index binding is inferred by count. Name-key identification and edited-image/game verification remain blockers. |
| 5 | Team biography/name/city/arena/colors | Anaheim fullname edited to QA Ducks and verified after save/reopen. Fixed separate NHL07 arena selector: city and arena name now resolve and edit the stock row. QA City / QA Rink saved and reopened through team view. Reassignment of the separate selector is not yet exposed; UI now states that limitation. No explicit RGB fields in stockteamtable metadata; kit/texture color bindings still require work. |
| 6 | Team logo | Fixed missing art-ID lookup. All four Anaheim families resolve. Copied t0.big now passes production preview/plan/replacement/fresh decode at 256×256, with exact backup and source unchanged. Packed archive deployment and game verification remain. |
| 7 | Center ice | Actual arena view reports unmeasured NHL07 asset mapping and no joined families. Implementation blocker, not proof that the game lacks editable ice. |
| 8 | Transactions/lines/scratches | Incompatible bundled save excluded. Main DB release/sign, two-team trade, jersey number, scratch and first-line centre assignment pass save/reopen. Traded memberships persist and the reassigned line slot has exactly one owner. Captaincy passes in-memory regression. Compatible Xbox360 roster-save fixture and game verification still needed. |
| 9 | Jersey texture/equipment colors | Kit mapping remains unmeasured. Corrected the earlier file classification: nine-number common files are meshes; actual color textures include rendering/jersey/jersey_0/jersey_0_0_0_cm.rx2. Fixed compressed texture review/Apply mismatch; this real 1024×1024 texture now passes replacement/reopen. Only base mip changes; 10 smaller levels retain previous content, explicitly disclosed. Full mip support, kit binding and game verification remain. |
| 10 | Player biography/attributes/portrait | Name and attribute serialization verified. 957 portrait paths resolve. Copied p1.big replacement/reopen passed at 468×248, exact backup preserved. Game verification pending. |
| 11 | Player equipment | Equipment-files API returns empty for sampled Anaheim player; field/file binding remains unresolved. Copied sticktape_0_cm.rx2 replacement/reopen passed at 128×64 after compressed-review fix. Base mip only, with 7 smaller levels explicitly retained. No player-assignment or game-verification claim. |
| 12 | Localization | Copied dictionary attached; one real text entry changed to QA through the retext API, saved and independently reparsed with the new value. In-game text verification pending. |

## Reproduction and evidence

- `crates/studio-api/tests/nhl07_source_qa.rs`: opt-in real-disc test, requires `NHL_STUDIO_XISO` and a fresh `NHL07_QA_OUT`; never patches the source ISO.
- `target/overnight-20261003/nhl07/source-routes-1/source-routes.json`: real source paths, counts and full partial-extraction diagnostic.
- `target/overnight-20261003/nhl07/source-routes-1/relocated disc dump`: usable copied game target for subsequent QA; originals remain untouched.
- `target/overnight-20261003/nhl07/edit-qa/reports/nhl07-xbox360.md`: anchor edit results and edited artifact paths.
- `target/overnight-20261003/nhl07/asset-observations.json` and `asset-observations-after-logo-fix.json`: before/after real asset view receipts. Read-only observations are not edit passes.
- `target/overnight-20261003/nhl07/edit-roundtrip-1` and `edit-roundtrip-2`: copied database artifacts; second run includes dictionary free-text edit. Both roundtrip tests pass. No originals modified.

Additional validation: NHL07 stock-team logo/mirror regression passes, existing kit API suite 3/3 passes, frontend TypeScript passes. A debug Studio binary was built and desktop-tested at the 10:25 checkpoint below.

## PS3 applicability — not yet advanced

The repository marks its NHL07 PS3 anchor as a research copy, not a shipping target. The supplied PS3 library begins at NHL08. Confirm applicability before attempting to treat the research fixture as a released PS3 game.

## 06:14 texture QA checkpoint

Reproduced a separate real Apply bug on compressed NHL07 jersey, stick-tape and ice files: the replacement plan returned unchecked/no notes without inflating RefPack, while the writer inflated it and discovered a partial-mip policy. The review guard correctly refused the mismatch, so the action could never complete. `plan_texture_replace` now unwraps one RefPack layer just like the writer, including the same nested-wrapper refusal. It discloses partial writes before Apply; it does not bypass the guard or claim full mip support. The new synthetic API regression failed before the fix; all 9 texture-review binding tests pass after it.

`texture-roundtrip-5/receipt.json` records five successful production preview/plan/write/fresh-decode checks on fresh copies: logo, portrait, jersey color, stick tape and generic ice texture. The three RX2 files remain base-mip-only writes; backups match originals and source files remain unchanged. Generic ice texture is not a verified center-ice/team binding. Earlier `texture-roundtrip-2` and `-4` receipts preserve failures. `-1` was interrupted by a QA fixture path mistake (missing common directory), not a Studio failure.

Also hardened NHL07 absolute-buffer parsing: use the measured DW8 pixel-entry reference instead of counting every untagged payload as texture data. A mesh-only common jersey file now correctly exposes zero rasters rather than an orphan-texture error. Synthetic mixed-payload regression verifies actual pixel decoding, preservation of the other payload on replacement, and refusal of dangling references. Texture codec unit suite: 17 pass, 7 private-fixture tests ignored.

Desktop: Xenia was located and foregrounded but no game launched in this checkpoint. Its log places storage under C:\Users\puckh\Documents\Xenia; isolated QA storage is still needed before saving games. No emulator configuration was changed, no new Studio binary built, no in-game edited result claimed.

## 07:10 arena and lineup checkpoint

The blank NHL07 arena list was a schema join bug. `exhibitionarena` (StEO) holds an arena ID plus stock/custom discriminator; names, city and venue attributes live in stockarenatable (cubz) or customarenatable (SulW). Arena detail now follows the key to the correct record, returns editable fields with their actual table, and resolves the team-to-arena relationship through exhibitionarena.index. Tenant counts and team links use that same join. This does not implement changing the home-arena selector; that refusal now describes the actual missing feature instead of saying there is no relationship.

Copied save/reopen tests passed in edit-roundtrip-3 and -4. The latter includes arena/city edits, an actual two-team player trade and clear/set first-line-centre assignment, in addition to previous name, jersey, scratch, release/sign and dictionary checks. Main-only NHL07 regression also passes. Arena asset/venue compatibility suites pass 6 tests, with 2 private-runtime cases ignored; these are regressions across schema shapes, not advancement to another game's QA pass.

Added Xenia Edge to the ISO write/restore running-emulator gate. The user's running xenia_edge.exe was missing from its recognized executable names. The boundary regression passes for Xenia, Canary and Edge, asserting unchanged ISO bytes on refusal.

Final compatibility check: team_relationships passed over all 27 committed main-database anchors, including save/reopen and undo (448.77 seconds). This validates the shared view change against later schemas; it is not a completed QA pass for those games. All test processes finished.

## 08:05 full-disc service

Added the production `disc_extract` service, replacing the QA-only disc-copy loop. Real NHL07 image copying passed to a fresh arbitrary path: 293 files, 3,360,408,912 bytes; every copied file's length checked, database bytes compared against the image, and database found by normal install discovery. The disc has 311 total entries including 18 directories. Receipt: `target/overnight-20261003/nhl07/production disc dump 2.receipt.json`.

The service streams with progress/cancellation and refuses preexisting output, unsafe names and case collisions. Interrupted copies are kept but are not success. Source image remains read-only. Two focused unit tests pass, including exact bytes, cancelled-copy handling and preservation of edited output. This service is not yet wired to onboarding or a durable reuse manifest; requirement 1 remains incomplete at the UI level. No claim of archive inflation, texture completeness or in-game edits follows from this copy test.

## 09:13 ISO onboarding integration

The service is now wired to a cancellable background command and ISO panel. Users can copy a full disc into a fresh arbitrary folder, extract its archives, and open the staged database with that copied tree associated for assets. Existing game folders can also be linked explicitly; leaving the field empty retains database-only editing. Active extraction blocks opening, and failed/cancelled disc jobs cannot open through that panel. Existing destinations are never overwritten by the disc copier.

Synthetic backend job integration passed; ISO/OpenProject frontend suites passed 20 tests and TypeScript passed. Real file-copy evidence remains the previous 293-file receipt. Desktop integration and edited gameplay remain unverified. There is no durable full-copy receipt or resume workflow: manually linking an existing folder after restarting only checks ordinary install detection, not copy completeness. The twelve-item game pass is still in progress.

## 10:25 desktop integration and metadata correction

Built and launched target/debug/nhl-studio.exe through Computer Use. Full-disc extraction from the real NHL07 ISO into `target/overnight-20261003/nhl07/desktop disc copy` completed through the UI. The following archive job wrote 15,590 files / 2.0 GiB, preserved 17 differing files and reported two unreadable containers. Opening was disabled while extraction ran. The onboarding scrollbar remained inside the 1200×800 window, and scrolling reached the bottom controls.

The first desktop Stage and open attempt failed because commands.rs assumed every title used nhlng-meta.xml. NHL07's inventoried name is NHL07NG-meta.xml. The resolver now uses the verified baseline inventory and its files root, supporting both development anchors and packaged baselines/files. Regression covers NHL07, NHL14, absent target, relocated packaged root and altered-schema refusal. Test passed. Rebuilt executable SHA256: `93147401298d295ea28947f00257165839e6b0e2b4658947c57c33aec019a213`.

After restarting, the same ISO opened to 93 teams with staging under `desktop iso staging`; its ISO-SOURCE.json records the real database/dictionary members. Anaheim's logo renders, along with Arrowhead Pond and Anaheim, CA. The four logo replacement controls resolve against the copied tree. No edits were deployed to originals.

New remaining issue: the automatic post-extraction rescan recursively treats extracted .big assets as additional extraction candidates. The original 11-container scan becomes 3,369 candidates with 3,349 unreadable entries, 13 members still to write and four collisions. This is a misleading second-pass result and needs archive-versus-asset classification; do not call those thousands of entries failed disc copying or silently suppress them. The completed job's original counts remain visible and preserved in its markers.

Created a separate portable Xenia Edge copy and local StudioQA profile under `target/overnight-20261003/nhl07/xenia portable`. Its log confirms storage, content and host cache roots are all inside that folder. Booted the previous `source-routes-1/relocated disc dump/default.xex` to the NHL07 intro/title screen. No edited gameplay or compatible roster save has been verified. Keyboard emulation was disabled by default; changed only the QA copy's keyboard_mode to 1 after exiting, retaining a before-keyboard config backup. Original emulators/configuration/saves are untouched.

## Continuous-session continuation after 10:25

The user clarified that checkpoints must not end active work. Continue through the checklist in this active session; the hourly heartbeat is recovery, not a reason to stop.

- Fixed extraction rescans by deriving ownership from actual parsed archive members. Extracted .big outputs are excluded as new source archives while independent unreadable containers remain reported. Cycles without an outer root remain visible. extract_service unit suite 9/9 passes; real read-only rescan of desktop disc copy now finds 11 source containers, two unreadable, 15,607 existing members, zero remaining writes and no collisions (previously 3,369 candidates).
- Exposed NHL07 equipment fields from ubPc when later equipment companion tables do not exist. Real stick edit saves/reopens in edit-roundtrip-5, with existing team/arena/roster/line/dictionary assertions retained. Main-database regression passes. equipment_scope now explicitly exercises early biography gear and retains later vocabulary checks; 2 tests pass across all 27 anchors in 125 seconds. This is schema regression evidence, not runtime validation of those games.
- Corrected the uniform QA observation: team_outfits, used by Uniforms, finds 35 Anaheim textures even though team_kits reports an unmeasured kit contract. asset-observations-with-outfits.json records this. Implemented filename fallback for unnamed single-raster textures to preserve number-font piece and color/normal-map labels; real observation regression is currently running. Do not claim it passed until its result is checked.
- TypeScript passed after the equipment view change. Current running r76 Studio does not contain these changes; the previous debug Studio is no longer open. No new build yet.
- Isolated QA Xenia still reaches title/intro, but injected Start has not produced a verified menu transition. Changed only its copied config from sdl to winkey (supported setting documented in that config), retaining .before-winkey backup. QA Xenia is running; original emulator/settings/saves untouched. Runtime edited-game acceptance and compatible save capture remain unverified.

Still NHL07 Xbox360. Preserve all unfinished changes. Next check the running real texture-label test, then continue copied-target deployment/runtime investigation and finish per-game report before advancing. No commits, pushes or releases.

### Continuous-session verification update

- Real NHL07 loose texture-label regression PASSED; receipt asset-observations-font-labels.json. Existing outfit filtering test also passed. Unnamed single-raster files now retain the filename's font/jersey and color/normal/spec identity; named multi-raster bundles keep internal names.
- Real NHL07 BIG4 deployment regression PASSED (233.71 s): copied data0.big with 3,490 members, edited two league-logo packages, one fitting and one growing by 23,040 bytes. Four-file transaction committed, sidecar and mirrored boot.big index re-bound, changed images decoded, unrelated entries checked, and all four files restored byte-exact. boot.big's mirrored member grew from 45,706 to 45,731 bytes within existing slack. Original files were only read. This proves archive write/restore integrity, not the game's loader acceptance.
- Rebuilt Studio SHA256 6FC10CF2037CBA5759A6ED3FCF477084714429274EAE1D4C50C1FB9439D22117 and began existing-folder desktop QA. Found a further bug: choosing a 360 installation required a save folder even for a main-database-only project. Removed that frontend gating from auto-detection and Next, and marked save location optional. Existing backend accepts no save destination. OpenProject suite passes 11/11 including this NHL07 case; another desktop build is in progress to verify the fix.
- Isolated Xenia no longer running; log ends Cheap-skate exit. Do not label this a crash without evidence. Injected menu input remains unverified, so no compatible save or edited gameplay claim.
- NHL07 PS3 applicability source located: EA's July 2006 announcement lists Xbox360, PS2, Xbox, PC and PSP, no PS3: https://s204.q4cdn.com/701424631/files/doc_news/2006/07/1/314374.pdf . The repository's PS3 NHL07 anchor is a research copy, not proof of a shipping PS3 game. Complete this applicability section when finalizing the per-game report.

### Desktop opening and capability correction

Desktop build 87AEEAF32869F998FDA5B6C51E0DA0DAC0E2056F3B37409EE2AA321053AF68EE opened a relocated NHL07 main database and dictionary with the save field blank into desktop gear project. The actual archive screen confirms 11 containers, 15,607/15,607 existing members, zero to write, two unreadable speech banks. Scroll controls remain inside the window.

The player page exposed one more missed integration gate: player_view returned early gear but SurfaceCapabilities still omitted the Equipment tab. Fixed the loaded-schema capability probe for biography stick/helmet fields; also enabled arenas when the early selector resolves stock arena names. Representative surfaces suite passed 4/4. Rebuilt binary EB92184EA56680D54DE1651443D030F4EEF6BFA160542E6C10A462F363437746 is now being desktop-checked.

Discovery found an original default Xenia roster even with the save field blank. No save was written. Removed that destination from the closed QA project manifest, preserving all original files. Copied the discovered roster into discovered-roster-copy: SHA256 40A06D5344D4CA7460CFFEB8EC0CB1626BDEFB1A28BA337F17C6F9DDACD2223D, identical to the already-rejected incompatible 31-table anchor. It is not a newly acquired compatible save. Follow up the auto-selected destination behavior; a blank save location currently still permits default-save discovery.

## First-pass disposition — proceed with recorded blockers

The user's instruction allows remaining items to be reviewed when one cannot be completed. All twelve Xbox360 items have evidence or an explicit blocker; none of the outstanding bindings is declared impossible merely because Studio lacks it. Moving to NHL08 does not close these NHL07 follow-ups.

| Requirement | Final disposition for this pass |
|---|---|
| 1 ISO extraction/load | Passed real service and desktop copying/staging/opening; archive extraction partial only for two unsupported speech banks and preserved differing files. |
| 2 Dumped-disc extraction | Passed real copied tree; repeat-extraction scope fixed and verified in desktop: 11 containers, 15,607 present, zero remaining writes. |
| 3 Existing files/location independence | Passed API plus desktop project creation/reopening from paths with spaces, with blank save location. Optional-save gating fixed. |
| 4 League name/icon | Logo replacement, archive/index cascade and rollback passed on real copied data0/boot pairs. League label key remains untraced; inferred logo association and edited-game display are not verified. |
| 5 Team biography | Name, arena name and city save/reopen passed; arena selector resolution and Arenas UI capability fixed. Home-arena reassignment and team color binding remain unresolved. |
| 6 Team logo | Real copied t0.big preview/replace/redecode passed; edited team-logo gameplay not verified. |
| 7 Center ice | Generic ice texture replacement works partially; arena-to-center-ice binding unmeasured, so team-specific acceptance blocked. |
| 8 Roster/lines/scratches | Main-DB release/sign/trade, jersey/scratch and first-line ownership save/reopen passed. Compatible roster container still unavailable: default Xenia save is byte-identical to the incompatible bundled fixture. |
| 9 Jersey/equipment colors | 35 Anaheim uniform files found. Native jersey replacement/redecode passed, with old smaller mips disclosed. Fixed font/map labels. Kit selector mapping, full mip rewriting, color-field mapping and gameplay remain open. |
| 10 Player biography/attributes/portrait | Data serialization and copied portrait replacement pass. Runtime identity/portrait verification open. |
| 11 Player equipment | Early biography gear now returned and Equipment tab shown in desktop. Stick edit save/reopen passes, with cross-anchor vocabulary regression. Numeric IDs are not proof of stock-player runtime assignment; 3D kit preview still fails to resolve, file bindings unresolved. |
| 12 Localization | Copied dictionary retext/save/independent decode passed. In-game rendering remains unverified. |

Desktop build EB92184EA56680D54DE1651443D030F4EEF6BFA160542E6C10A462F363437746 shows the Equipment tab and 18 biography gear fields for Vitalij Aab, plus the Arenas navigation. No desktop edit was deployed. The isolated QA project has no original save destination; discovery's automatic default-save association is a follow-up concern. All original game/save bytes were preserved by this pass.

### NHL07 PS3 applicability

Not applicable for all twelve shipping-game requirements: NHL07 did not ship for PS3. EA's [July 12, 2006 announcement](https://s204.q4cdn.com/701424631/files/doc_news/2006/07/1/314374.pdf), page 2, lists Xbox360, PS2, Xbox, PC and PSP. The repository explicitly labels its PS3 NHL07 anchor a research copy; schema regression over it must not be reported as PS3 gameplay support. No PS3 image or runtime pass is claimed.

### Remaining runtime blocker

Isolated Xenia reached the title/intro but injected Start/A did not yield a verified menu transition with either SDL keyboard-enabled or WinKey configuration. No edited gameplay, fresh compatible roster save, or renderer acceptance was obtained. These checks require a working controllable input route and measured asset bindings. No additional file permission was needed; no approval request is outstanding.

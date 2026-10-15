# NHL Legacy PS3 asset parity — 11 October 2026

Status: review build installed and launched; automated checks passed, user visual/RPCS3 acceptance pending. Scope is the existing Xbox 360 asset-preview/export/texture-replacement surfaces, applied to Legacy PS3. This does not qualify other PS3 games by inheritance.

## Changes

- Shared RenderWare texture reading now supports PS3 in file exports, player texture exports, head/hair catalog exports and rendering-logo fallback. Raw exports retain their native extension. Multiple colour maps in a file export receive distinct names instead of overwriting one another.
- PS3 meshes and textures participate in appearance census and generic/staff catalog scanning. Texture containers are classified as textures rather than attempted as head geometry. Separate hair lookup follows the platform extension.
- Equipment fallback searches and colour-map selection accept PS3 containers. The player equipment page uses the same preview layout as Xbox 360.
- Team/head/equipment previews resolve project overlay files. Absolute-path head/catalog/portrait routes also use recorded project edits; head/catalog previews include the shared eye textures. Successful texture writes invalidate relevant preview and Overview caches without invalidating unrelated player tables.
- Fixed a shared multi-raster packaging defect: repeated edits to a container now retain ordered image sources. Installation composes those operations in memory and writes one final container. Older Studio builds refuse the new `texture-layers` origin instead of silently packaging only its last image. Existing one-image projects remain readable.
- Image-free Apt custom-jersey packages report that no image exists, rather than an Xbox-specific decode error. These are not invented bitmap textures.

## Evidence

Actual input: `X:/Games/Emulation/Roms/Sony Playstation 3/NHL Legacy - Vanilla/PS3_GAME/USRDIR`. All live installation reads were read-only. Replacement/build/install/restore work used temporary project and recipient trees.

- `legacy_ps3_asset_probe`: 5,627 image previews; 27 replacement/redecode checks; FE and rendering-logo fallback checks. Six team-logo families, four league-logo families, 3,023 portraits, stock jersey thumbnails and representative rendering bundles. The three custom jersey packages contain Apt drawing records and no bitmap and are explicitly distinguished.
- `legacy_ps3_assets`: real-container player GLB/DDS/native export, head/hair catalog, PS3-only generic sweep, individual equipment preview and export filename collision checks. Project image edits are checked through save/reopen/build/install/byte-exact restore, including two rasters edited in the same head container and preview reads from the overlay.
- `legacy_ps3_uniform_probe`: skater/goalie kits through the actual Registry API, both team defaults and player-assigned gear. Asserts material bindings and both sides of paired equipment; the metal goalie cage intentionally has no paint texture.
- Existing package and bound texture-review regressions cover malformed packages, destination conflicts, stale reviews, and unchanged single-write behavior. Frontend tests cover replacement refresh and uniform routing.

Final checks: 2 real-fixture integration tests, 13 package/review regressions, 7 census tests, 4 focused storage/export/equipment tests and 29 frontend tests passed. All four uniform probe variants passed, including paired geometry checks. TypeScript/Vite and the native development build succeeded. The census suite's one optional fixture test remained ignored; the explicit Legacy probes above were run separately. Studio was relaunched for user review.

Logs: `target/ps3-asset-probe.log`, `target/ps3-package-proof.log`, `target/ps3-parity-uniform.log`, `target/ps3-parity-package-regression.log`, `target/ps3-parity-domain.log`, `target/ps3-parity-ui-tests.log`, `target/ps3-parity-ui-build.log`, `target/ps3-parity-studio-build.log`.

## Single-pass user review

1. Open a Legacy PS3 QA project linked to the extracted vanilla source. Inspect league/team logos and their variants, player portraits and jersey thumbnails. Confirm colours, transparency and correct team/player identity.
2. Open a skater and a goalie: check heads/eyes/hair, Uniforms and player Equipment. Rotate/zoom and toggle pieces. Check both gloves, socks, skates and goalie pads, and the assigned models and team colours.
3. In the File Browser, export a head mesh and multi-raster texture bundle. Confirm GLB and separate images open. Export player art with original files enabled; PS3 originals must end in `.rpsgl`. Check head/hair catalog textures.
4. Replace a team logo and two distinct rasters in a texture bundle with recognizable QA images. Confirm 2D and relevant 3D/header previews refresh. Save, reopen and verify both changes remain.
5. Build a PS3 package, activate it through Mod Manager, restart Legacy in RPCS3 and inspect the edited assets. Check a short match for model placement, lighting, clipping and texture orientation.
6. Close Legacy, deactivate the mod and restart. Verify the original assets return.

Report the item number, asset/player/team and observed defect. Automated material assignment does not certify in-game appearance.

## Onboarding performance follow-up

Source selection was followed by a blocking extraction preflight that read complete BIG archives and reopened nested asset containers. Preflight now reads BIG directory entries only, skips deeper containers already declared as parent outputs, and retains the original extraction order. Database discovery checks standard locations first and uses one fallback traversal for missing inputs. The onboarding extraction screen reuses its preceding scan for 30 seconds instead of immediately repeating it.

Read-only Legacy reference measurements during this change: PS3 discovery under 1 ms and preflight 3.9 seconds (17 archives, 45,714 members); Xbox discovery 211 ms and preflight 27.8 seconds (17 archives, 45,845 members). These are source/disk-dependent scan timings, not total project-creation guarantees. The original PS3 preflight was stopped after several minutes without completing. Verification: 14 extraction regression tests and 36 onboarding/extraction UI tests passed; the packed speech-index test also covers a BIG4 header whose directory-offset word understates its table length. User timing confirmation remains pending.

## Nameplate and face-shading follow-up

The Legacy Anaheim alphabet atlas contains an apostrophe between L and M. Connected-component mapping previously counted it as a letter and shifted the remaining alphabet. Short punctuation is now excluded from letter slots, with a regression for M/T/Y/Z. The PS3 EDGE normal decoder also inherited a missing range scale from RPSGL2; decoding and normal/tangent encoding now use the correct `1/sqrt(2)` range for stored components. This changes preview shading without modifying source assets. See the RPSGL2 review correction for the independent Xbox comparison. Five font tests, 131 codec tests and the 13-file native mesh round-trip fixture check pass. Visual confirmation of Getzlaf and Horcoff remains a user check.

## Boundaries

Native PS3 same-topology OBJ import from the preceding codec task remains a command-line operation. Arbitrary mesh replacement, project/build mesh-operation integration, rig/animation editing and automatic Xbox/PS3 conversion are not added by this parity pass. Shared Xbox features that already refuse a project write are not silently bypassed for PS3. Arena archive replacement and console archive-versus-loose precedence retain their existing qualification boundaries. The distributed tester ZIP is unchanged.


## Tester feedback follow-up (11 October)

Implemented measured member progress for the current archive and whole extraction. A finished job replaces the extraction prompt with a completion card; skipped files remain explicitly identified. The extraction destination is editable with a folder picker. A separate assets tree preserves relative paths and includes files shipped loose. Extraction does not relocate or overwrite the source installation. The project retains its source location separately from its extracted-assets location; relinking and reopening are tested.

Project Overview now has a persistent Send to Mod Manager action. It waits for queued edits, saves the project and its metadata, builds current changes, and opens Mod Manager with those packages selected. Activation remains a user action. The library displays game box art and offers a platform filter, while incompatible installation targets cannot activate the selected mod.

Uniforms now exposes Edit Jersey Layout above the preview. Nameplate replacement sits with Kit assignments, rather than another full-size texture card. The layout editor offers discovered jersey meshes; the default kit preview first matches the kit's style. Filename suffixes for body/sleeve/LOD variations are retained rather than assigning unverified era labels. In-game model-selection parity beyond the style prefix is still a manual check.

NHL07 career-history row removals now use edits schema 5. Earlier package schemas remain readable. Unchanged surviving career rows are matched in order, eliminating false field edits caused by row shifts. The removal package binds to the original table's row hashes and refuses a changed recipient layout. A disposable NHL07 test deletes six regular-season and six playoff rows, builds, installs, checks exact saved bytes, restores exact originals, and verifies refusal against a changed baseline. This qualifies pure row removals, not arbitrary mixed career-row insertion/replacement operations.

### File Browser texture diagnosis and proposed next work

The reported NHL07 `eyes_628_0_cm.rx2` warning comes from `Texlib::large_level_support`: DW0 `0x00000003` is outside the qualified later-generation tiled large-mip writer (`0x00400003`). The current path replaces the base image and intentionally retains smaller mip levels. The preview can therefore show a correct replacement while distant in-game sampling still uses original content. The screenshot reports ten retained levels and 196,608 retained bytes. The codec was not changed in this follow-up.

Next work: inventory NHL07/08 texture layouts by format, dimensions, descriptor and mip count; verify each large-level offset and packed-tail placement against independent decoder/runtime evidence; implement complete mip-chain generation and native tiling while preserving sibling rasters, descriptors and padding; add changed-image and no-op round trips, then compare near/distant views in Xenia. Do not widen the descriptor gate without that placement evidence. Later-generation large-mip support also does not by itself imply full packed-tail replacement.

### Build versus installation

Build currently writes platform-qualified packages containing portable database/dictionary edit intent and authored texture payloads. It does not preconvert every asset into destination-native files. Mod Manager stages the destination's files, replays supported edits and encodes supported texture replacements through the destination codec before installing with backups. Database cross-platform support is based on measured baseline compatibility. Texture support is narrower; automatic Xbox-to-PS3 conversion of every mesh/texture is not provided.

### User verification

1. Create a project from a dumped source, choose a separate extraction destination, and extract. Check both bars, completion status and the displayed destination. Pause once and resume. Enter Studio, inspect an asset, then save/reopen and inspect it again.
2. Change a project field and click Send to Mod Manager without separately saving/building. Confirm the latest build appears with game art; exercise the platform filter and activate only for a compatible installation.
3. In Uniforms, open Edit Jersey Layout. Confirm nameplate replacement is in Kit assignments and select two available jersey models. Inspect placement and return to the current kit model.
4. Rebuild the tester's NHL07 career-row-removal project, activate the resulting package, inspect Modano in game, then deactivate and confirm restoration.

Automated evidence: extraction regressions; focused onboarding, overview, manager and uniform UI tests; native career removal install/restore and extracted-assets reopen regressions. Final counts and native build outcome are recorded in the task response. A new tester archive has not been packaged for these changes.

## NHL 08 review follow-up (12 October 2026)

User verification: the updated onboarding works correctly on NHL 08. The user reports all other items in the preceding verification list passed, with the new findings below requiring this follow-up. This does not independently certify every title/platform combination.

Implemented:

- NHL08 Rival Team now uses the stored team index directly rather than subtracting one. Anchor witnesses include Anaheim 13 -> Los Angeles, St. Louis 10 -> Detroit, Calgary/Edmonton 4 <-> 11, and the All-Star teams 30 <-> 31. The original game assignments are preserved; St. Louis is not automatically changed to Chicago.
- NHL08 uniforms now assemble the native opposite glove, sock and skate meshes. Gloves select the handed raster masks/details within the brand texlib; helmets use their early-title mask/detail filenames. The preview uses NHL08 equipment type codes 4/7 rather than the later 5/8. NHL07's prepainted texture handling remains separate.
- Edit Jersey Layout sits immediately below the player preview. The editor warning now says reference alignment and the scale of letters and numbers are approximate and need in-game verification.
- NHL08's seven-bit yeardrafted field now displays/edits with base 1959. Anchor witnesses: Modano and Selanne 29 -> 1988, Pronger 34 -> 1993, Sakic 28 -> 1987, Crosby 46 -> 2005. The unverified note is removed for this supported layout. Wider NHL09/10 layouts are not newly qualified by this change.
- Removed the About & Licences Settings tab. The standalone legal-information page remains available.
- Transaction player names now select their row without navigating away or discarding the transaction setup.
- Mod Manager reads the selected validated package and uses the shared expandable ChangeTree. It displays database/entity/field before-and-after values, localization, texture replacements, schedule changes, free-agent membership changes and career-row removals without editor links or revert buttons. Reviewing does not activate a package. Existing packages do not store full league/team ownership or descriptive identities for structural operations: those entries retain package table/record/player/slot identifiers rather than inventing names.
- Corrected native-to-TypeScript number annotations for extraction progress counters so regenerating bindings preserves the working progress bar contract.

Automated evidence:

- 35 focused UI tests passed, including transaction-name selection, read-only mod review and retry, existing activation/deactivation, Project Overview and uniforms regressions. TypeScript check passed.
- NHL08 native rival read/write and five-player draft-year display/write-offset regressions passed.
- A read-only native preview of the user's extracted NHL08 Xbox360 installation passed: both-handed geometry for gloves/socks/skates and bound colour textures for gloves/helmet. This verifies assembly and texture bindings, not final in-game aesthetics.
- NHL07 paired equipment/prepainted texture regression passed.

Review in Studio:

1. Open NHL08 QA. Check Anaheim's rival is Los Angeles and St. Louis's is Detroit. Select Chicago for St. Louis if desired, confirm the choice persists, then Undo if this was only a test.
2. Open Anaheim Uniforms. Rotate the preview and inspect both gloves/socks/skates plus glove and helmet textures; check a home and away kit. Open Edit Jersey Layout below the preview and read the expanded reference warning.
3. Open Modano (1988), Pronger (1993) or Crosby (2005). Confirm Draft Year shows a calendar year without an unverified note. Edit once and Undo.
4. In Transactions, select names on both sides. Confirm selections remain on the page and the transaction stays prepared.
5. Check Settings has no About & Licences tab. In Mod Manager, select a built mod and expand its Changes in this mod groups to review field values and any asset/structural changes. Merely selecting and reviewing must not activate it.

Final follow-up evidence: all eight cross-title field-profile regressions and both native career-package/asset-reopen regressions passed. The career-package check now confirms all twelve removals appear in the manager's read-only review. Total focused checks: 35 UI + 14 native/domain tests passed. Development executable rebuilt successfully on 12 October 2026; no updated tester archive was produced in this task.

### NHL08 right-skate texture follow-up

User review: every item in the 12 October follow-up passed except the right skate's laces in Uniforms preview.

Diagnosis: the paired NHL08 skate meshes were both taking the first colour raster from `skate/texlib_0.rx2`. That bundle contains separate `skate_0_0_0_cm.Raster` and `skate_0_1_0_cm.Raster` maps, with different UV layouts. The right mesh therefore sampled the other side's lace layout. Preview now selects a colour raster matching the skate mesh's handed filename tokens, while retaining the generic picker for other mesh layouts.

The native real-file regression now requires both handed colour-raster names in the assembled preview, in addition to both meshes and textured gloves/helmet. Remaining user check: reopen NHL08 Uniforms, zoom in on the skates and confirm the laces align on both sides. No project or game asset data is rewritten by this preview fix.

Automated result: the real NHL08 native kit test passed with separate side-0 and side-1 skate colour maps (4.48 seconds). Visual confirmation of the lace alignment remains with the user.

User verification (12 October 2026): NHL08 right-skate lace alignment passes. All items in this NHL08 review follow-up are now user-accepted. Next review target: NHL09; no NHL09 parity claims are implied by NHL08's results.

### PS3 ISO onboarding (12 October 2026)

Enabled Game ISO for PS3 in Project Setup, using the existing native ISO9660 extractor. The picker explains that PS3 images must be decrypted; Studio does not decrypt encrypted images. Suggested extraction folders strip `.dec.iso` as well as `.iso`. Xbox support is retained; PS4 ISO support is not enabled.

Verification:

- All 36 OpenProject UI tests passed, including new PS3 success, failure/retry and cancellation/retry cases. Project creation waits for successful disc extraction. TypeScript check passed.
- Read-only source: `X:\Games\Emulation\Roms\Sony Playstation 3\NHL 08 (USA).dec.iso`. Native QA extracted 46 files (5,589,771,798 bytes), detected the PS3_GAME/USRDIR installation, validated database CRCs, opened 2,962 players and 138 teams, and confirmed the source database remained unchanged. Complete test took 147.57 seconds including game-asset extraction.
- Asset extraction wrote 19,342 files. Two speech containers, `audio/speech/pbp_eng.big` and `audio/speech/rink.big`, report unsupported speech samples and remain unextracted. This is a separate audio-format follow-up; disc copying succeeded. Do not mark all PS3 NHL08 containers supported.
- Evidence: `target/nhl08-ps3-iso-review.log`, `target/nhl08-ps3-iso-review-20261012/receipt.json`, and `target/ps3-iso-ui-test.log`. QA output is separate from user projects and installations.

User verification: Create New Project -> NHL 08 -> PS3 -> Select Game Source -> Game ISO. Choose the decrypted ISO, select a new or empty extraction destination, then complete Project Setup. Confirm extraction finishes and the project opens. In-game verification remains pending.

### NHL08 PS3 speech-container support (12 October 2026)

The two remaining extraction failures were a codec admission issue, not damaged ISO files. The compressed IDX tables use the same record and version-0 SNS framing as the supported Xbox banks, but identify EALayer3 (codec 5) instead of EA-XMA (codec 3). The extractor now admits both codecs while retaining its table, offset, block, sample-count and zero-padding validation. Encoded SNR/SNS pairs are preserved without transcoding. Unsupported headers now include codec and stream details in the diagnostic.

Measured PS3 NHL08 USA population:

- `pbp_eng.big`: 29,750 streams, mono; 29,745 at 44,100 Hz and 5 at 48,000 Hz.
- `rink.big`: 8,463 streams at 22,050 Hz; 8,438 mono and 25 stereo.
- All 38,213 streams passed an independent full-bank framing/count/padding scan. All are version 0, non-looping, streamed EALayer3.

Scope: native onboarding / Tools container extraction and retained encoded exports. This does not add audio playback, replacement, encoding or bank rebuilding, and makes no in-game audio-editing claim. Existing Xbox extraction behavior is retained.

Regression coverage: 3 speech parser tests and 15 extraction-service tests passed. New opt-in test `crates/studio-domain/tests/nhl08_ps3_speech_extract.rs` exercises real banks, cancellation/resume, every exported SNR/SNS pair, complete-state rescan, repeat extraction, and original BIG/IDX hash preservation. Run with `NHL08_PS3_INSTALL` pointing to the extracted PS3_GAME/USRDIR folder. Results are logged under `target/nhl08-ps3-speech-extract.log`.

Final result: the full real-bank regression passed in 510.96 seconds. All 38,213 samples (76,426 SNR/SNS files) matched source bytes; cancellation/resume succeeded; rescan returned complete; repeat extraction wrote zero files and retained all 76,426 outputs; original BIG/IDX hashes were unchanged. All eight original installation containers parsed successfully. Native development build succeeded and Studio was relaunched with a responsive window. Full-bank QA outputs remain isolated under `target/nhl08-ps3-speech-qa/1791798777581635700`.

User check: in the NHL08 PS3 project, run Extract remaining (or Settings -> Tools asset extraction) against the existing source. Confirm both speech banks finish and the extraction panel reports complete without either bank being skipped. There is no need to re-extract the ISO.

### NHL08 PS3 equipment/uniform preview failure (12 October 2026)

User report: both Player Equipment and Team Uniforms showed "no kit piece could be decoded".

Cause: NHL08 PS3 uses RW4 static geometry with OpenGL primitive 5 (triangle strips). The native PS3 static reader only admitted primitive 4 (triangle lists), so every equipment piece was rejected at draw-range validation. Added bounded strip-to-triangle conversion with alternating winding; degenerate joins are skipped while retaining their parity. Existing triangle-list handling and index/buffer bounds checks remain in place.

The real preview regression also exposed Xbox-only glove texlib and helmet mask/detail paths. NHL08 compositing now resolves PS3 glove bundles through the native PS3 texture reader and uses the helmet mesh's platform extension for its masks/details.

Verification: five PS3 mesh unit tests passed, including strip winding through degenerate joins and triangle-list compatibility. Real PS3 API previews for Ryan Shannon/Vancouver passed in both team-uniform and player-equipment modes, checking geometry and texture bindings for jersey, pants, socks, gloves, helmet and skates. Both sides of the gloves, socks and skates assemble. The existing real NHL08 Xbox 360 preview regression also passed. Source assets are read-only; these are preview-decoding changes, not game-file conversions or mesh import support.

Evidence: `target/nhl08-ps3-preview-test.log`, `target/nhl08-ps3-mesh-unit.log`, `target/nhl08-xbox-preview-regression.log`. Remaining user verification: reopen NHL08 PS3, inspect Ryan Shannon -> Equipment and Vancouver -> Uniforms; rotate the figure and check textures and both sides of the paired equipment. No extraction rerun is required.

### NHL08 PS3 selected-player head follow-up (12 October 2026)

Ryan Shannon resolves correctly to head ID 10010, but that PS3 file splits one index buffer into several named draw ranges. The static mesh reader previously required each draw to consume the entire buffer, silently dropping the head from kit assembly. It now validates and consumes each draw's index range independently, resetting strip winding at each draw. Static submeshes retain their resource names, allowing face, eye and mouth texture binding.

NHL08's shared eye colours ship as loose `player/eyes_<colour>_cm.rpsgl` files instead of the later `player_common/texlib_999` bundle. The preview now resolves this layout too. Kit assembly also includes a head's separate hair companion when that file exists; this is not a new generic hairstyle-assignment system.

Verification: five PS3 geometry unit checks passed; the real NHL08 PS3 regression now explicitly requires `head_10010.rpsgl` and texture bindings on every head submesh in both team and player-equipment modes (8.75 seconds). The existing Xbox 360 kit regression passed (5.14 seconds). Evidence: `target/nhl08-ps3-head-test.log`, `target/nhl08-ps3-head-unit.log`, `target/nhl08-head-xbox-regression.log`. No source or project assets are changed.

Remaining visual check: Ryan Shannon -> Equipment, then Vancouver -> Uniforms with Ryan selected. Confirm the head appears beneath the helmet and inspect face/eyes from front and side. No re-extraction is needed.

Final companion check: the real PS3 regression additionally requires `hair_10010_0_0_0.rpsgl` in the same head item and passes with a textured `haircap_.Mesh` alongside the face/eyes/mouth in both preview modes (9.10 seconds). Head and hair share one visibility/focus item.

User verification (12 October 2026): NHL08 PS3 selected-player head preview passes. The head/hair/eye follow-up is accepted in the equipment and team-uniform preview review. This closes the reported missing-head issue; it does not qualify unrelated NHL08 PS3 features.

### NHL09 Xbox 360 review fixes (12 October 2026)

User report: one glove/sock/skate, missing eye textures and helmet; Draft Year unverified; vanilla Centennial Canadiens flagged for multiple captains.

Implemented:

- NHL09 now uses its handed equipment meshes in both preview modes. Helmet default lookup includes `helmet_common`, while mask/detail colours still resolve from `helmet`. Shared loose eye textures also resolve from `player_common`.
- NHL09 gloves use the existing early zone-mask compositor with direct brand numbering. For an unset brand 0 (no shipped glove texlib 0), preview uses shipped brand 1; this is a preview fallback, not a database change or a runtime-verified assignment rule.
- NHL09's eight-bit yeardrafted uses +1900, verified against Modano/Selanne raw 88, Pronger 93, Sakic 87, Crosby 105. The profile displays calendar years and supports edits with that offset. Other unqualified titles remain unchanged.
- The NHL09 Centennial Canadiens (team 157) are exempt from the user-facing single-captain audit only. No other team's findings, other predicates, roster data or underlying transaction guards are changed.

Verification: real NHL09 Xbox previews pass for both team/default and player-equipment modes, requiring paired gloves/socks/skates, helmet geometry, and texture bindings including eyes. Five draft-year witnesses and an edit to 2008 pass. Centennial audit test passes, including deliberately adding duplicate captains to an ordinary team and confirming they still produce a finding. Existing NHL08 Xbox and PS3 preview regressions pass. Logs: `target/nhl09-review.log`, `target/nhl09-captain-test.log`, `target/nhl09-nhl08-preview-regression.log`. Native development build succeeded.

User checks: inspect NHL09 Team Uniforms and Player Equipment from both sides; confirm helmet and textured eyes, gloves, socks and skates. Check Modano's draft year is 1988 or Crosby's is 2005 without an unverified label. Re-run audit and confirm the seven Centennial Canadiens captain findings disappear. No game-file edits or extraction rerun are required.

Final regression result: all eight cross-title field-profile tests passed (76.56 seconds). Studio relaunched with a responsive window; visual NHL09 acceptance remains with the user.

### All-PS3 asset read qualification (12 October 2026)

The NHL09 report had two independent causes: its selected source contained only the three shipped boot textures, with player/uniform files still packed; and NHL09 uses an older EDGE mesh layout unsupported by the later-title decoder. The project had recorded extraction as completed because Continue to Studio unconditionally stored that choice.

Changes:
- Continue to Studio records skipped unless extraction is actually verified complete. A finished job with missing outputs keeps extraction available instead of claiming assets are ready.
- PS3 asset-root resolution recognizes EBOOT.BIN beneath disc root, PS3_GAME or USRDIR. The empty uniforms view links directly to asset extraction settings.
- Native NHL09 EDGE reader supports bounded 128-byte segment configurations, interleaved 16/20/21-byte vertex layouts, fixed-point positions, unit normals, UVs, padded vertex counts and split DMA lengths. Extra UVs/skinning are not rewritten. This newly supported layout is preview/export only; mesh import remains refused pending a write contract.
- NHL11 low-detail geometry lacking normals gets topology-derived preview normals; position and UV data remain mandatory.

Read-only archive qualification sampled the actual original archives for NHL08, NHL09, NHL10, NHL11, NHL12, NHL13, NHL14, NHL15 and Legacy. Samples cover primary and secondary mesh families, lower LODs, uniform/player/equipment textures, team/league logos and portraits. All sampled reads pass; this is representative format coverage, not exhaustive verification of every asset or in-game acceptance.

| PS3 title | Decoded samples | Result |
| --- | ---: | --- |
| nhl08 | 72 | Pass |
| nhl09 | 76 | Pass |
| nhl10 | 78 | Pass |
| nhl11 | 78 | Pass |
| nhl12 | 84 | Pass |
| nhl13 | 85 | Pass |
| nhl14 | 85 | Pass |
| nhl15 | 85 | Pass |
| nhllegacy | 85 | Pass |

NHL09 full extraction into `target/nhl09-ps3-art-qa` passes native preflight, and both team/default and player-equipment previews contain textured head/hair, helmet, jersey, pants, gloves, socks and skates. The 541-vertex glove and 607-vertex hair mesh match the corresponding Xbox assets at every position/normal/UV within codec precision; truncated inputs and unqualified writes are rejected. Original source folders and user projects were not changed.

Evidence: `target/ps3-asset-final-matrix.log`, `target/ps3-nhl09-preview.log`, `target/ps3-nhl09-geometry-compare.log`, `target/ps3-root-tests.log`, `target/ps3-mesh-unit.log`, `target/ps3-extraction-ui.log`, `target/ps3-types.log`. Eight asset-root tests, five mesh unit tests, 42 onboarding/extraction UI tests and TypeScript checking pass.

User review:
1. Reopen NHL09 PS3. Settings > General > Tools > Extract now, retaining the intended assets destination. The existing project flag does not prove its source was extracted. Wait for completion.
2. Open a team's Uniforms and a player's Equipment; inspect the complete figure and face/eyes, then change the selected player and kit.
3. For NHL10, NHL12 and NHL13 sources used in this audit, complete extraction before reviewing art; their original source trees also lack required loose art. NHL08, NHL11, NHL14, NHL15 and Legacy source trees already expose uniform textures, but some unrelated containers remain unextracted.
4. Visual/in-game verification is still with the user. No claim is made that all other feature work packages pass for these titles.

Final result: 728 samples passed across all nine PS3 titles. All six uniform UI tests pass, including the extraction-settings link and rescan after external extraction. Native development build succeeded (`target/ps3-parity-dev-build.log`).


### NHL10–Legacy equipment preview parity (12 October 2026)

Scope: NHL10, NHL11, NHL12, NHL13, NHL14, NHL15 and Legacy on Xbox 360 and PS3, through both Uniforms (team/default equipment) and Equipment (selected-player equipment). NHL15 PS4 is a separate asset generation and is not included in this qualification.

Changes:
- NHL10–11 gloves use a handed filename family different from NHL07–09. Their gloves, socks and skates now include the native opposite-side mesh.
- NHL12 shortened all three handed filename families. Those counterparts are resolved separately; NHL13–Legacy combined bilateral meshes are retained without duplicate geometry.
- NHL10–11 glove masks are decoded by raster name from the brand texlib. NHL12 uses loose handed feglove masks; NHL13–Legacy use loose combined masks. Team zone colours and detail overlays produce diffuse textures in both tabs, including selected-player glove models. Generic brand-zero records fall back to an available shipped mask set.
- Selected-player helmets receive the matching shell's zone mask/detail and team colours. Existing shell/chinstrap resolution remains intact.
- NHL12's shorter skate raster names are selected by side, preventing the opposite skate from borrowing the wrong lace texture.

Verification fixture: `crates/studio-api/tests/equipment_preview_parity.rs` reads original PS3 archives and Xbox ISOs without changing them, extracting only QA samples under `target/equipment-parity`. It exercises 28 previews (seven games × two platforms × two tabs), checks positive and negative X geometry for each paired equipment type, verifies helmet presence, requires textures on every equipment submesh and rejects raw glove masks in place of painted diffuse textures. This is representative skater/kit coverage, not every player, equipment brand or in-game acceptance.

Eleven kit-preview unit tests pass, covering native pairing, no duplication of later combined meshes, idempotent completion and handed skate texture selection (`target/equipment-unit-final.log`). Five texture-compositor unit tests also pass (`target/equipment-texture-unit.log`). Final cross-title qualification passes: all 28 previews and 112 equipment checks, with bilateral gloves/socks/skates and textured helmets on both platforms (`target/equipment-parity-final.log`). Native executable rebuild succeeds (`target/equipment-draft-dev-build.log`); the updated development Studio was relaunched for user visual review.

Remaining visual review: for each title/platform, open a team's Uniforms and a skater's Equipment tab. Rotate the preview and check both gloves, socks and skates, both lace panels, the helmet shell and their colours/textures. Switch the player and kit once to check refresh. Sources must have their game assets extracted.


### NHL10–Legacy draft-year review (12 October 2026)

NHL10's eight-bit `yeardrafted` field was excluded from the verified year-base resolver, leaving values such as Crosby's 105 visible as raw/unverified. NHL10 is now qualified for +1900 alongside NHL09. NHL11–Legacy's `draftyear` already uses the correct +1900 conversion; those titles were verified rather than assigned a new offset. The Player Data summary now reads the verified calendar display from either field generation, preserving unavailable/sentinel values as absent instead of manufacturing a year.

`crates/studio-api/tests/draft_year_parity.rs` verifies Crosby (2005), Ovechkin (2004) and Joe Thornton (1997) in all seven Xbox360 and PS3 anchors: 42 witnesses pass, including absence of the unverified warning, correct edit offset, matching Player Data summary, editing to 2006 and restoring the original value. All edits are in memory; anchor files are unchanged. Evidence: `target/draft-year-parity-final.log`. Existing NHL08/NHL09 draft-year regression tests also pass (`target/draft-year-regressions.log`).

User review: reopen a project for each title, inspect Player Data > Draft Year, edit a year and undo it. For NHL10 Crosby should display 2005 rather than 105. The equipment checks above can be completed in the same session.

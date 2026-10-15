# NHL08 findings — first pass reviewed, acceptance partial

All twelve requirements have been reviewed on Xbox360 and PS3. Edited gameplay remains unverified. The later Joliet filename repair resolves the earlier PS3 loading crash; title/intro rendering now works, but menu input is unresolved. Originals remain unchanged. The dated continuation notes below preserve the investigation; the latest results supersede earlier pending states.

## Verified source routes

### Xbox 360 onboarding follow-up, 2026-10-05

The original `NHL 08 (USA).iso` now supplies `anchors/xbox360/nhl08/game-files.json`, embedded in the Studio verifier. ISO SHA-256: `4cb8f03fa7274d1a3009acac8e619feb7931ad5967f0918bf85bdc5d7c62624b`. The manifest contains 26 required files (18 loose, 8 containers) and one optional system-update file. Read-only verification of the user's fresh installation returned Vanilla / USA, all 26 matching, no findings. Both database and dictionary anchors and their metadata exactly match original loose disc files. Receipt: `target/nhl08-vanilla-check.json`. Completed extraction no longer bypasses the verification page for NHL08.

The two unreadable speech banks are `audio/speech/pbp_eng.big` and `rink.big`. Their `.idx` files ship compressed inside `data0.big`, rather than loose as in NHL07. Extraction now resolves these packed indexes using the existing BIG4/BH reader and decompression path. Loose indexes take precedence; invalid loose indexes are not silently replaced. Completion hashes bind to decoded index contents, so writing the same index loose during extraction does not invalidate the bank's marker. The existing IDX/SNR/SNS layout validation passed every stream: 29,750 play-by-play and 8,463 rink samples. These are encoded audio exports, not decoded playback or a bank rebuild implementation. NHL07 roster-schema research is unaffected.

The historical two-container limitation below is superseded by this follow-up. Desktop extraction review and in-game audio behavior remain separate from automated verification.

All 14 extraction-service regression tests passed, including extraction while indexes are still packed, unchanged completion identity after indexes become loose, and refusal of an invalid loose index. Completion-marker lookup now indexes member names once to avoid quadratic scans on these large speech exports.

Post-extraction correction: the user's full installation exposed a gap in that first test: BIG extraction preserves native RefPack wrappers, but the loose speech-index reader expected decoded bytes. Both speech-bank markers were complete, yet the next scan rejected the compressed loose indexes. The reader now decodes RefPack in memory with a bounded declared output size, preserving the on-disk index. The regression fixture now uses a compressed packed index and verifies identical identity after extraction. All 14 regression tests passed. A read-only scan of the user's actual extracted NHL08 installation then reported all eight containers complete, including both speech banks; no files or markers were rewritten. Reproduce with `cargo run -p studio-domain --example extraction_status --offline -- <installation-root>`.

The opt-in `extract_service::tests::nhl08_packed_speech_indexes_validate_every_stream` test passed in 468.31 seconds with `NHL08_INSTALL` pointing at the user's original installation. All 8 input containers parsed. Both original banks were copied into an owned test directory with the decoded indexes, and extraction produced 76,426 SNR/SNS files with zero failed archives, unsupported samples or conflicts. Every exported header/stream matched its source; completion rescan passed; a repeat run wrote zero files and retained all 76,426. Original bank hashes remained unchanged. Evidence: `target/nhl08-speech-qa/2026-10-06T03-17-11Z`. TypeScript checking and all 24 onboarding tests also passed.

`nhl08_source_qa::xbox_iso_dump_and_relocated_files_open_and_preserve_staged_edits` passed against the real Xbox360 ISO in 144.98 seconds. It staged 2,962 players and 138 teams, edited a first name, saved/reopened staging, checked CRC and unchanged source database bytes. Full-disc copying wrote 27 files / 5,197,431,724 bytes into `target/overnight-20261003/nhl08/source-routes-1/relocated disc dump`. Archive extraction wrote 19,182 files / 3,053,412,646 bytes, no conflicts, with two unsupported speech containers. Existing extracted files reopened with matching entity counts and localization attached. Receipt: `source-routes-1/source-routes.json`.

The receipt's rescan says complete although two speech containers were unreadable. This exposed a status bug: all supported archives being complete suppressed the aggregate partial status. Code now keeps the result partial when unreadable archives remain; regression passed (9/9 extraction tests).

## Playtest career-stat defect

Reproduced wrong 1940s years on NHL08 Xbox360. Also found an incorrect player join: Crosby displayed Getzlaf's Anaheim stats. The early career join metadata has range_low=0, while later tables use -1; the old code always added one. Early years have 6/7-bit fields based at 1959; later fields are 8-bit based at 1900.

The view now uses the metadata join offset and schema year base for season labels, field display and edit transforms. NHL08 Xbox360 and PS3 anchor regression passes exact Crosby 2006–07 79 GP / 120 PTS and Santala 2003–04 33 GP / 3 PTS rows. The known Crosby line is also supported by the [NHL player data](https://api-web.nhle.com/v1/player/8471675/landing). Existing later-schema career view tests pass 3/3. Added career-row authoring has the same old hard-coded offsets; that path has now been changed and its added-row regression passes both schemas; later-schema add/edit/delete also passes. No original database is saved by these tests.

## Twelve-item tracking

| # | Requirement | Status |
|---|---|---|
| 1 | ISO extraction/load | Real API copy/stage/edit/reopen passed; desktop check pending. |
| 2 | Dumped-disc extraction | Real newly dumped disc extracted; two unsupported speech banks retained/reported. |
| 3 | Existing files, arbitrary location | Relocated extracted tree opened with matching counts and dictionary; desktop pending. |
| 4 | League name/icon | Derived dictionary key and native logo present; copied logo replacement passes. Code-to-name binding remains a hypothesis; targeted dictionary edit test pending. |
| 5 | Team biography/arena/colors | Team name and independently selected arena name/city save/reopen pass. Team-to-arena association unresolved; no verified RGB team palette. |
| 6 | Team logo | Six Anaheim variants found; copied t0 package replacement/decode passes. Archive deployment running; gameplay pending. |
| 7 | Center ice | Generic ice base image replacement passes; arena-specific center-ice binding unmeasured. |
| 8 | Transactions/lines/scratches | Release/sign, trade, unique line slot and scratch/jersey state save/reopen pass on main DB. Incompatible bundled save excluded; gameplay pending. |
| 9 | Jersey/equipment colors | 65 Anaheim outfit textures; actual jersey color slot and stick texture replace/decode pass on copies. Smaller mip levels remain unchanged; full replacement blocked. |
| 10 | Player biography/attributes/portrait | First name and aggression save/reopen pass; copied head replacement passes. Career player join/year fixes pass; desktop and gameplay pending. |
| 11 | Player equipment | 17 fields exposed; raw stick ID edit/save/reopen passes. Catalogue labels and component sets remain unmeasured. |
| 12 | Localization | Copied dictionary opens and text edit/save/reparse passes; runtime pending. |

No edited gameplay has been verified. Do not advance to PS3 or NHL09 until the remaining items are reviewed and this report records their evidence or concrete blockers.

## Continued editing and texture evidence

Career authoring now passes the extended regression for both schemas; later-schema add/edit/delete also passes. The extraction status suite passes all nine tests.

`nhl08_source_qa::nhl08_team_and_roster_edits_save_and_reopen` passes against fresh copied targets (`edit-roundtrip-2`). It verifies team naming, the independently selected Honda Center's name/city, release/sign, cross-team trade, one unique first-line center, scratch/jersey state, first name, aggression displayed as 85, stick equipment ID, localization retext and CRC after save/reopen. Team-to-arena association remains absent: this schema has venue names directly in StEO but no kOtt.buDa, and metadata does not establish the reverse association. Do not infer ownership solely from equal indices.

`nhl08_native_texture_replace_roundtrip` passes (`texture-roundtrip-3`) for team logo, player head, league logo, actual jersey color image slot 2, stick and generic ice. Earlier texture-roundtrip-1 covered slot 0, a number font. These tests verify changed decoded pixels, exact original backups and unchanged input files. The RX2 replacements remain base-level-only; smaller mip levels retain old content, explicitly disclosed by the writer. Generic ice replacement does not establish a team's center-ice logo binding.

Asset observation receipt records six Anaheim logo families, 65 outfit textures and 17 gear fields. Six league codes are derived from team membership; localized name bindings are hypotheses, not verified league-table edits. Team-kit, arena-art and equipment component-set contracts are unmeasured. BIG4 copied archive deployment test is running; desktop and edited gameplay verification are still pending.


## Desktop and runtime continuation

Desktop build `0088388D4959578083056980785E2A1E7036C1F796A258400D9CF97B2464289B` opens relocated NHL08 data/assets in an isolated project with no save destination. It correctly displays partial extraction (8 containers, 19,182 members present, 0 remaining, 2 unreadable speech banks). Santala's career now shows 2003–04 and 2006–07 with correct statistics and a resolved portrait. Anaheim Uniforms renders a jersey and five mesh pieces, eight selectable kits and texture controls; gloves and helmet are untextured in the preview.

That desktop check exposed a free-agent banner bug: a stale biography main-team pointer overrode an empty roster membership. Removed this fallback; NHL08 two-schema and later Legacy header regression passed. This last correction and clearer uniform/deployment copy are newer than the running executable. TypeScript and 22 relevant frontend tests pass.

The extended `edit-roundtrip-4` also preserves a new 2007–08 career row after reopening and changes dictionary NHL text to QHL, reflected in the derived league display. Runtime league binding is still unverified. Localization holds one pending operation at a time: sequential edits require saving between them; a multi-edit queue has not been implemented.

Isolated portable Xenia launched the copied NHL08 disc and accepted Start/A input to advance into local profile/autosave setup. Original emulator state is untouched. No roster save or edited gameplay has yet been verified. The real copied archive test has committed after 1,044.6 seconds; final integrity/restore checks remain running.

## Xbox360 archive result and PS3 continuation

The copied Xbox360 BIG4 deployment completed successfully in 1,449.02 seconds: 14,162 archive entries, fitting logo and 1,664-byte growth, edited-image/index checks, unrelated-member preservation, and byte-exact archive/sidecar restoration. This supersedes the earlier pending result. The isolated emulator attempt ended at profile/autosave setup without a captured roster or edited gameplay verification.

PS3 work is now active. A decrypted NHL08 ISO copied 46 files / 5,589,771,798 bytes through the new bounded ISO9660 reader, including split multi-extent DATA0.BIG. Main DB CRC passed. Archive extraction ran, then exposed a hardcoded RPCS3-root discovery failure. The discovery correction passes synthetic disc-root/PS3_GAME/USRDIR/relocated-HDD checks and existing RPCS3 tests. The full real-source rerun remains pending; PS3 editing, native-texture and runtime acceptance are not yet established. The new frontend offers decrypted-ISO copying and explicit import of completed output. It does not decrypt encrypted images or write changes back into PS3 ISOs.

## PS3 requirements reviewed so far

| # | Requirement | Current evidence / limitation |
|---|---|---|
| 1 | ISO extraction/load | Decrypted ISO9660, including multi-extent DATA0.BIG, copied via the desktop job API; project creation verified on desktop after schema fallback fix. Encrypted and non-ISO9660 disc formats remain unsupported. |
| 2 | Dumped-disc extraction | Desktop PS3 dump extraction writes 19,342 assets from six supported containers; two speech banks remain unreadable. |
| 3 | Location-independent files | Relocated disc root, PS3_GAME, USRDIR and HDD discovery regressions pass; direct extracted DB/dictionary project opens. Explicit relocated save location supported and tested. |
| 4 | League name/icon | Copied NHL→QHL dictionary edit persists and changes Studio's derived league display. League logo replacement decode/backup checks pass. Runtime name binding not verified. |
| 5 | Team biography | Team name and separately selected arena name/city persist. Team-to-arena association remains unmeasured; team city/color fields are not established in this schema. |
| 6 | Team logo | Native PS3 logo replacement passes changed decode, backup and input-preservation checks. |
| 7 | Center ice | Generic ice texture replacement passes. Specific team-center-ice binding remains unmeasured. |
| 8 | Rosters/lines/scratches | Release/sign/trade, unique first-line centre, scratch and jersey state persist after save/reopen. |
| 9 | Jersey/equipment colors | Actual PS3 jersey color slot 3 visually verified and replacement passes. Earlier slot-2 test was a font specular map, superseded. Stick-tape color persists. PS3 uniforms now discover 80 Anaheim textures. 3D mesh preview requires unsupported PS3 mesh decoding. |
| 10 | Player data/portrait | Name, rating, career add/edit and native portrait replacement pass copied-file checks. Career year/join corrections tested on both platforms. |
| 11 | Player equipment | Raw stick and tape-color changes persist; file catalogue resolves PS3 files. Component-set contracts and game appearance remain unverified. |
| 12 | Localization | Real 5,718-string dictionary imports; text changes survive save/reparse. Single pending localization operation still requires saving between edits. |

Evidence: ps3-source-routes-5 (job API, before later runtime edits), ps3-edit-roundtrip-2, ps3-texture-roundtrip-2, ps3-asset-observations-4 and its six slot previews. Desktop build 5DB30AB6... completes source review, project creation, archive extraction and team-page loading. Frontend kit-grouping correction is newer than this executable and needs desktop recheck.

Runtime remains unaccepted. Isolated RPCS3 loaded the edited copy through compilation to NHL08 loading, then showed a likely-crash warning before names or gameplay appeared. A same-settings baseline comparison is running. Do not attribute that failure to the edits or emulator until comparison evidence is available; do not report the game pass complete yet.

## NHL08 first pass completed with explicit blockers

All twelve requirements have now been reviewed on both platforms; this is partial acceptance, not verified edited gameplay. Latest desktop build SHA256 5DCE9A43383146597222422B7479762C0F32AA408195D7E05DEC7CC0FAFC38F9 renders PS3 Anaheim jersey color/specular/normal maps, separate number-font maps and nameplate controls across eight kits. The real asset regression passes with an exact assertion on jersey color slot 3 (ps3-asset-observations-5). PS3 mesh decoding remains unsupported. Bulk outfit PNG export still uses the Xbox-only decoder and remains a separate limitation.

The unedited PS3 baseline reproduces the same loading-screen likely-crash warning as the edited copy under isolated RPCS3 0.0.41-19510. EBOOT.BIN, DATA0.BIG and baseline NHLNG.DB match the independent existing disc dump byte-for-byte (SHA256 checks). No evidence establishes an edit-induced runtime regression, and no changed gameplay/name has been accepted. Isolated game stopped after comparison; original emulator state remains untouched.

Remaining blockers: fresh compatible Xbox roster capture/runtime acceptance; PS3 baseline runtime crash; PS3 3D mesh and bulk outfit export; early Xbox lower-mip replacement; measured team-arena/center-ice and kit assignment; localization multi-edit queue. Encrypted/non-ISO9660 PS3 images remain unsupported. Continue to NHL09 Xbox360, then PS3; retain these limitations for follow-up.

Follow-up: PS3 bulk outfit export decoder fixed during NHL09 continuation. Real NHL08 six-raster jersey bundle exports every PNG byte-identically to the established per-raster preview; source bytes unchanged. ps3-outfit-export-1 and native_ps3_bundle_exports_every_raster PASS. This resolves the earlier bulk-export limitation; PS3 mesh preview remains unsupported.

NHL10 investigation exposed primary ISO filename aliasing in earlier PS3 copies: NHL08 lacked ESRB-E-nV.vp6, NHL08-Intro-ng.vp6 and nhlng-meta.xml under their full names (punctuation replaced in primary aliases). Thus the earlier missing sibling schema was caused by this extraction naming issue; bundled-schema fallback remains useful for genuinely missing XML. Joliet manifest's 46 filenames/lengths match the independent existing dump. Added only the three missing correctly named files to generated source-routes-5 and desktop disc from that read-only dump, SHA256 checked, preserving aliases and all existing files. Receipt target/overnight-20261003/joliet-copy-repairs.json. Runtime baseline retest underway; earlier likely-crash result should not yet be classified as an emulator-only blocker.

NHL08 corrected baseline reaches title screen (RPCS3 ~30FPS), superseding the earlier loading-crash blocker. Fullscreen captures stayed on an intro frame; windowed capture after Escape reveals title. Continue input/menu verification in windowed mode.

Corrected baseline remains stable through title and multiple intro/attract frames, unlike the old incorrectly named copy. Start/Cross attempts did not produce an observed menu; the title transition alone does not establish input acceptance. Edited gameplay still blocked by menu/input progression. No emulator-only crash claim remains. Runtime stop requested before NHL09 regression.

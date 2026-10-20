# NHL 27 → NHL 24 deking backport: continuation roadmap

Updated 2026-10-16. This is the starting document for a fresh session, not an installation receipt.

## Scope correction — current user instruction

The user superseded the broad runtime objective below: port the moves into NHL24's existing gameplay systems, with only individual-move adapters. Keep the panic/input/PTM runtime research separate and uninstalled. The historical roadmap below is retained as research, not installation scope. The current build lives in `deke-trial/moves-only`; completion requires integration, activation, installation and installation checks. Gameplay testing belongs to the user.

### Current installation — six-frame pickup fallback for both reversal routes, 2026-10-17

User accepted the movement and drop strength and requested a six-frame pickup wait for both boards and open ice. The local general-pickup fallback cap is now 6 ticks for private variants 7/8/9/10, reduced from 12 for boards and 24 for open ice. Shorter native waits, negative sentinels, native eligibility checks and unrelated moves are preserved. This caps fallback eligibility; it does not force possession.

The accepted situational net-reversal animations, board rebound, and open-ice soft drop remain unchanged. Open-ice launch speed retains the exact 1.5x post-clamp multiplier (0.1125–0.165 per tick), with no incoming velocity inherited. Launch/state banks match the previous drop-power-150 installation byte-for-byte. Runtime SHA-256: `01856d6b417a106ca90454b7d1c1d1dca150c354b0adeecc98a05ae92bcc92f8`; payload 12,848 bytes and the same 12 declared executable edits.

All 14 runtime and 9 integration tests passed, including both handedness routes, the 5/6/7-frame boundary, unrelated variants and negative sentinel preservation. Installed and active in `F:\Games\NHL 24\game-mods`. Startup remained responsive for 81.3 seconds with zero crash markers, then closed normally. Post-startup installation checks passed for all six overlay files, decoded banks, allocation metadata, activation and protected originals. Gameplay testing remains with the user.

Previous overlay is preserved at `F:\Games\NHL 24\nhl27-moves-only-before-pickup-six-20261017-002012`. Evidence: `moves-only/user-video-review/pickup-six-*`, current integration/runtime receipts, and the installed metadata. Launch NHL24 normally.

### Previous installation — situational net reversal along the boards, 2026-10-16

The user rejected the forced-stop/isolated-turn approach: it stalled the player while the puck played behind them. New instruction: extend the complete working back-of-net reversal to board situations, rebounding off boards instead. All direct pose velocity/speed writes, edited turn tracks, artificial entry blends and turn retiming from the failed experiments are removed from this build. Historical files remain as evidence, not active fragments.

The change now extends gameplay selection as well as animation selection. After NHL24 recognizes a board-bank opportunity and the reverse command, native type 1/private 7 becomes supported native type 6 (BH net reversal); native type 2/private 8 becomes native type 5 (FH net reversal). Private values 7/8 remain latched so the release adapter uses nearest-board geometry. The normal native bank owner/lifetime continues; ordinary banks, actual net reversals and open-ice types are not remapped. NHL24's own setters emit native 5/6 at `2b642c7`/`2b643b5`, and its bank-release code consumes those supported types. No source-only 7–10 values enter native gameplay tables.

Board rows 7/8 share the exact same complete sequence objects as net rows 6/5, including all clips, blends, warpers, events and recovery. This differs from the earlier animation-only route: the native gameplay type now selects the corresponding net-reversal behavior too. The existing puck-release join replaces the net-derived velocity with the board rebound for private 7/8. Board release again uses the live skater speed, with no forced stall; open-ice retains its pre-command capture. Availability follows NHL24's existing board-bank opportunity detection around the rink and the existing reverse Skill Stick command.

Nine bank integration tests and eleven runtime tests pass. They verify shared board/net object identity, source-identical sequence timing/events, 1,024 gameplay mapping combinations, unchanged ordinary/net/open-ice dispatch, idempotent promotion, board geometry and hook ABI. Controller audit reports zero unexplained differences. Static/startup checks do not establish gameplay success; the user owns gameplay testing.

Launch bank SHA-256: `9c27d3ae19ad9f0f520a3d10fea431d3e80cd659830c01e9e99388c1580380da`. EBOOT: `ad3233b8157e71cb1f86c7ab65ab623540ee032ce864e43a043dabf87e746212`. State remains `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6`. There are 125 controller groups, five native asset replacements, the same ten executable edit sites, and a 10,896-byte RX payload. The old installation is preserved at `F:\Games\NHL 24\nhl27-moves-only-before-situational-net-20261016`. Evidence: `moves-only/user-video-review/situational-net-*`, the current integration/runtime receipts and `drop-back-audit/report.json` (now explicitly records no animation or velocity edits).

### Previous installation — handed net turns and reverse-entry stop, 2026-10-16

Latest user feedback: the dedicated open-ice animation improved the excessive board excursion, but turn direction appeared the same across sides/handedness and open-ice reversals did not stop. The user explicitly requested the behind-net stop/turn animation and logic for boards. Updated `nhl bank move.mp4` is 14 seconds and shows continued open-ice translation; sampled frames are `moves-only/user-video-review/current.jpg`.

Board FH value 8 now selects the private source `StartFromStop_180R` clip (`194a8553075dbb0b`); board BH value 7 selects `StartFromStop_180L` (`9fcc3b3c892b7d3c`). These are the second clips of the corresponding net sequences. Private board sequences retain the net event/recovery tags and actor flags, remove the bank/sidestep lead-in and its +/-25-degree warper, and play the handed turn from clip time zero. Their duration equals the original turn end time (158/162 ticks). The net rows 5/6 remain untouched. Open-ice rows 9/10 retain their dedicated source clips. Four-tick private entry blends remain.

The move-specific runtime now applies a one-time entry stop for accepted private reverse values 7–10: it clears horizontal pose velocity at +50/+58 and skating speed +90, preserving position, facing and vertical velocity. This occurs after the existing pre-command velocity capture and before publishing the reverse request. The existing nonzero-request latch prevents repeated stopping during the move or recovery. It does not freeze the actor or alter general skating. Both reverse puck solvers use the captured incoming velocity; the board solver previously read live speed, which would otherwise become zero after the new stop. Native enum values and original hook sites remain unchanged.

Eleven runtime tests and eleven bank integration tests pass. Coverage includes the exact stop write footprint, untouched ordinary/net variants, board puck release with a stationary live pose and nonzero captured speed, actual selected handed clips, removed lead-in/warper, source event preservation and unchanged net rows. Controller audit records the intentional board track differences explicitly and verifies the linked adapted fragments with zero unexplained differences. These checks are not gameplay verification.

Launch bank SHA-256: `c254d355f10bea36988b1e477c034a62b84312e34397e882c1c21302a93e7501`. EBOOT: `f0f8386b56612bd43cd81b3074e19abc2cdfbd6b345d8db9aea417f48fde03fe`. State remains `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6`. The RX payload is 10,832 bytes with the same ten declared executable edits. The user authorized closing their running NHL24 process normally for installation. Previous overlay is preserved at `F:\Games\NHL 24\nhl27-moves-only-before-handed-entry-stop-20261016`. Current evidence is in the integration/runtime receipts, `drop-back-audit/report.json`, and `user-video-review/handed-stop-*`.

Final installation/activation checks passed. Startup PID 72376 remained responsive for 77.9 seconds with zero crash markers and closed normally; gameplay was not entered. Post-launch checks passed for all six overlay files, decoded banks, allocation metadata and protected originals. See `user-video-review/handed-stop-startup.json`, `handed-stop-startup.log`, and `handed-stop-installation-check.json`. Launch NHL24 normally.

### Previous installation — dedicated drop-back board sequence, 2026-10-16

The user supplied `C:\Users\puckh\Videos\nhl bank move.mp4` showing that the stationary-pivot revision still swung the skater far away from the boards. Review of frames throughout the 18.47-second recording shows repeated sideways excursions before recovery, notably around 2.5–3.5 and 6–7 seconds. The prior clip-translation edit did not establish the desired on-ice result. Do not describe that revision as gameplay-verified or retain its coefficient edits as the current solution.

The board route had retained a DK21 bank lead-in plus a separate skating turn borrowed from the net reversal. It now uses private copies of the dedicated DK25 drop-back sequences: board FH value 8 uses `cfbf592baad94152` / clip `74f3d0ccc47e82e0`; board BH value 7 uses `dc7b10363575bca2` / clip `5b659266dd830670`. Each is one source clip with its source actor flags, release event and recovery tags, without the old lead-in, appended turn or net warper. The private four-tick entry blend remains. Open-ice rows 9/10 keep the same dedicated source sequences. Net rows 5/6 and the runtime puck solver remain unchanged; board values still use board-specific puck release. No compressed animation coefficients are edited by the current adapter.

`deke_pivot.py` stages to `moves-only/drop-back-audit`; old `pivot-audit` files are historical and not included. Eleven integration tests pass. The controller audit compares every cloned sequence object, dynamic array, event and reference with its dedicated source and reports zero unexplained differences. Tests verify actual selected clips, handedness, one animation/no warper per track, unchanged net routes and source zero horizontal trajectory. These static checks do not prove the in-game swing is eliminated; gameplay comparison remains the user's task.

Installed launch hash: `2061450bcbc150b11cd5220f0987eefe262c6346d8d88a26ff4e11494302197a`. State and EBOOT hashes remain `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6` and `16d5313d78f1279a9ba699f865f14845403945af99b5995dd822a902c3dbd97d`. There are 131 controller groups and the same five native record replacements. Previous overlay is preserved at `F:\Games\NHL 24\nhl27-moves-only-before-dedicated-drop-back-20261016`. Review evidence and frame contact sheets are in `moves-only/user-video-review`; current sequence evidence is in `drop-back-audit/report.json` and the integration receipt.

Final checks passed: PID 98372 remained responsive for 65.0 seconds, logged zero crash markers and closed normally without entering gameplay. Post-launch installation verification passed for all six overlay files, both decoded banks, allocation metadata, activation configuration and protected originals. Evidence: `user-video-review/startup-check.json`, `startup.log`, `installation-check.json`, and the live installation receipts. Launch NHL24 normally.

### Previous installation — stationary board and open-ice pivot, 2026-10-16

User supplied https://www.youtube.com/watch?v=ve6Lhdn9QBU and https://www.youtube.com/shorts/TuF7jB-TjQQ as drop-back references, and explicitly requested both open ice and boards. The first video's playback/transcript shows the abrupt stop, tight pivot and backward puck pickup. Controls remain the existing Skill Stick command: left stick opposite facing plus L1/LB and R2/RT. No new runtime hook or gameplay-system port was introduced.

`deke_pivot.py` now creates private copies of the compound board sequences and their four animation clips. Horizontal translation coefficients in vector channel 92 (CTD identity `72b6a0f8`) are zeroed, with Distance metadata set to zero. This trajectory identification is supported by its horizontal endpoint displacement matching clip Distance across the four clips, and the same DOF being zero in both source open-ice reversal clips; a symbolic DOF name was not recovered. Every other compressed coefficient, rotation, vertical lane, pose lane, key time, event and sequence duration remains unchanged. This is an intentional stationary-movement adaptation, not a claim that the edited clips are byte-identical to NHL27. The prior net sequence carried 2.50–3.37 metres of turn travel and entry blends of 92/78 ticks.

Only bank rows 7–10 select the new private sequences. Board rows 7/8 retain their compound turn rotations, but use stationary clips and a four-tick entry blend. Open-ice rows 9/10 retain their already-stationary source clips and receive private four-tick entry blends (previously eight). Net rows 5/6, original clips/blends, puck release, command selection and general skating remain unchanged. Eleven integration tests pass, including exact unaffected-coefficient comparisons, actual bank-row/track reachability, stationary open-ice samples and preserved net routes. Controller audit reports no unexplained differences and declares the stationary adapters explicitly.

Installed launch bank SHA-256: `6b1f2b876b73181a3eb21fbcd4e07fbdf0ce70a0ba0b016bcda06be6ece6c7d3`. State bank remains `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6`; executable remains `16d5313d78f1279a9ba699f865f14845403945af99b5995dd822a902c3dbd97d`. There are 135 dependency groups and the same five native record replacements. Previous installation is preserved at `F:\Games\NHL 24\nhl27-moves-only-before-stationary-pivot-20261016`. Evidence: `moves-only/pivot-audit/report.json`, source/edited resources, integration/package and installed receipts. Gameplay comparison remains with the user; static stationary trajectories do not establish complete frame-by-frame gameplay parity.

Final installation checks passed after startup: six overlay file hashes, both decoded banks, allocation metadata, activation configuration and protected originals. Startup PID 80704 stayed responsive for 65.6 seconds with zero crash markers and closed normally. Evidence: `moves-only/startup-stationary-pivot.json`, `startup-stationary-pivot.log`, `installation-stationary-pivot.json`, and the live `F:\Games\NHL 24\nhl27-moves-only` receipts. Launch normally; gameplay was not entered.

### Previous installation — board reversal turn routing, 2026-10-16

User gameplay feedback: the board reversal must promptly turn the skater 180 degrees and move back, matching the working back-of-net reversal. The previous report incorrectly attributed the added compound 180-degree sequences to the board-reversal rows. Actual database column values are `1,2,3,4,5,6,8,7,9,10`: values 5/6 select the compound DK26 sequences, while board reversals 8/7 still selected the older single-clip DK21 sequences. The prior controller audit did not expose this behavioral mismatch.

The narrow `BOARD_TURN_ROUTES` adapter now maps the board forehand row (value 8) to the same sequence as net forehand (5), and board backhand (7) to net backhand (6). This deliberately changes two source database output references to implement the user's requested comparison. It preserves command recognition, the enum values, both existing net rows and the board-specific puck release. It does not modify the runtime executable or the general skating system. The comparator explicitly declares these two intentional reference adaptations rather than presenting them as unchanged source data.

Eight integration tests pass, including a regression checking the actual column ordering, both handedness routes and unchanged bank values. Evidence: `moves-only/board-turn-review.json`, `output-controller-audit.json`, integration/package receipts and the installed installation/startup receipts. Previous overlay is preserved at `F:\Games\NHL 24\nhl27-moves-only-before-board-turn-route-20261016`. Launch normally; gameplay verification remains the user's responsibility.

Final checks passed: the launch bank differs from the previous installed bank in exactly two relocation destinations; object records and animation samples are identical. New launch hash: `9c27d3ae19ad9f0f520a3d10fea431d3e80cd659830c01e9e99388c1580380da`. EBOOT remains `16d5313d78f1279a9ba699f865f14845403945af99b5995dd822a902c3dbd97d`. Installation/activation and protected originals passed verification. Startup PID 90320 stayed responsive for 75.9 seconds, logged zero crash markers and closed normally. No gameplay was entered.

### Previous installation — move-output correction, 2026-10-16

The output audit corrected four move-specific mismatches:

- Reverse mid-ice banks 9/10 now capture velocity at command acceptance (`2b64346`, source `47e62ce`), reverse it with multiplier 0.3, blend 0.6 toward the skate midpoint, and apply minimum puck speed 0.12. Source virtual feet 0x28/0x26 map to native 0x23/0x21. Three private per-actor context words hold the snapshot. Reverse-board 7/8 retain the previous board geometry correction.
- Both reused 180-degree skating clips differed in animation samples and movement distance. Private source copies, metadata and embedded tags now serve the reverse-board sequences. Original NHL24 skating clip IDs/samples remain untouched. Source distances: 3.3691206 and 2.5025182; native: 3.3596940 and 2.4783785.
- Toe playback's maximum changes from native 1.10 to source 1.05 at the move-specific curve endpoint `aa52b9c`.
- The private toe request clears at native `2b702d2`, matching source `47e9d7a`, after animation start. Previously only the native request cleared, leaving a stale private selector.

All 84 reachable source controller groups pass scalar, dynamic-array and normalized-reference comparison. The 33 added move payloads and two private turns round-trip against staged source conversions. Ten runtime/selector tests and seven integration tests pass, including 1,728 board geometry cases, 300 mid-ice cases, source constants, snapshots, request clearing and general skating preservation. These checks do not establish frame-by-frame gameplay parity. NHL24 eligibility, collisions, general skating and recovery remain native by scope. Gameplay testing belongs to the user.

Final EBOOT SHA-256: `16d5313d78f1279a9ba699f865f14845403945af99b5995dd822a902c3dbd97d`. Launch bank: `c96be7b3237c0e10c885aa060970afbea7600dba67ca1a517daa04a3ace736f9`. State bank remains `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6`. Ten declared edits (nine hooks, one curve word), 10,656-byte read-only payload, 125 dependency groups. Overlay: `F:\Games\NHL 24\game-mods`; launch normally. Earlier revisions are preserved under the game root in `nhl27-moves-only-before-movement-audit-20261016` and `nhl27-moves-only-before-toe-clear-20261016`.

Rebuild order: `python scripts/nhl27/deke_turn_clips.py`; `python scripts/nhl27/deke_moves_only.py local/nhl27-eboot/compatibility`; `python scripts/nhl27/build_deke_skill.py`; `python scripts/nhl27/deke_expanded_package.py local/nhl27-eboot/compatibility --moves-only`. Audit with `deke_output_audit.py` against the same compatibility root. Evidence: `moves-only/output-review.json`, `output-controller-audit.json`, `turn-audit/report.json`, build receipts and installed installation/startup receipts. Use host process visibility for installation checks.

Final installation and protected-original checks passed. Startup PID 76148 stayed responsive for 115.2 seconds with zero crash markers and closed normally. The final installed receipt confirms the active overlay; no gameplay was entered.

### Previous installation — NHL27 Skill Stick selection, 2026-10-16

**Reverse-board release correction (previous):** The user's gameplay test found a blended forward bank/reversal, with the puck continuing forward. The previous adapter published reverse variants 7/8 only to animation, so native NHL24 still computed its forward bank release. Source function `47e1430` explicitly handles reverse-board velocity and skips its forward-only fallback. A seventh narrow hook at native `2b6ca8b` now substitutes that reverse-board calculation at the existing puck-release join. It uses NHL24's nearest-board cache and NHL27's board tangent reversal, speed-to-angle curve and distance-to-speed curve. Ordinary banks and behind-net banks retain their native result. Animation assets and command recognition did not change in this correction. The broader runtime remains separate.

Current overlay executable SHA-256: `d28c7e4f8247194caa5e83eead003652dbb651446dcbf137319d30b661d8b1cd`. Previous installation and disabled overlay are preserved at `F:\Games\NHL 24\nhl27-moves-only-before-reverse-release-20261016`. The eight Skill Stick tests pass, including 1,728 independent geometry cases, source curve-default verification, actual guest variant gating (only 7/8 modify velocity), and execution of the new hook's argument/frame mapping and replayed instructions. Original payload reconstruction and installation checks passed. Startup PID 73260 remained responsive for 68.1 seconds, logged no crash marker, and closed normally; protected originals and the active installation passed checks again afterward. Evidence is in the current installed `startup-check.json` and workspace `startup-reverse-bank.log`. No claim is made that the user's observed skater-turn behavior is verified fixed on ice; the user must retest the integrated move.

The user rejected the extra left-stick modifiers described in the historical section below. Those modifiers are removed. Use NHL27's normal Skill Stick commands: the existing NHL24 recognizers activate the moves, and narrow adapters reproduce NHL27's contextual glide, toe-drag and bank variant selection. Native NHL24 gameplay ownership, eligibility and transitions remain in place. The broader panic/PTM/input runtime is not installed.

The current package is installed and active at `F:\Games\NHL 24\game-mods`. It contains five TOC/CAS files and an adapted overlay `eboot.bin`; the original base/update executables, original assets, runtime executable, configuration and existing patch XML remain unchanged. The obsolete modifier build is preserved under `F:\Games\NHL 24\nhl27-moves-only-left-stick-20261016`.

- Six selection hooks publish move selection into four private animation properties; the seventh hook corrects reverse-board puck release. NHL27-only enum values do not enter NHL24's native gameplay properties. Original NHL27 22-row toe-drag and 10-row bank databases and lateral-glide chooser use those private properties.
- 389 shared clip replacements, 33 new move clips, 121 controller dependency groups and five native record replacements. The 21 source-only panic clips remain excluded.
- Previous selection-only executable SHA-256: `535be5af0a90519a7bf649bfc41df93cd47d8b8a71f665b9ddfa9760abcd990f`; superseded by the release correction above.
- Launch bank SHA-256: `f87263b9a04643b781d3530ed0d1b316aecfce7ad3dc1ae3742958c781a57d5a`. State bank: `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6`.
- This revision passed 22 relevant tests: six integration invariants, five Skill Stick/runtime fixtures, ten linker tests and one database test. The runtime fixtures include all 65,536 packed left-stick/camera angle combinations against the original source and native machine decoders, plus hook ABI preservation and move-selection boundaries. These are component checks, not on-ice behavioral verification.
- Startup PID 66272 remained responsive for 137.4 seconds, with no logged unhandled exception, then closed normally. No gameplay was entered. Evidence: `nhl27-moves-only/startup-check.json` in the installation and `deke-trial/moves-only/startup-skill-stick.log` in the compatibility workspace.
- Installation hashes, decoded banks, allocation metadata and protected originals passed again after startup. The six-file rollback was exercised, the identical package reactivated, and installation checks passed again. Receipts: `F:\Games\NHL 24\nhl27-moves-only\installation.json` and `rollback-check.json`.

Build in order: `python scripts/nhl27/deke_moves_only.py local/nhl27-eboot/compatibility`, `python scripts/nhl27/build_deke_skill.py`, then `python scripts/nhl27/deke_expanded_package.py local/nhl27-eboot/compatibility --moves-only`. Check installation with `python scripts/nhl27/deke_moves_install.py check local/nhl27-eboot/compatibility/deke-trial/moves-only/package "F:\Games\NHL 24"`. Run installation checks with host process visibility; sandbox process enumeration can miss the running game. Rollback remains `F:\Games\NHL 24\nhl27-moves-only\rollback.ps1`, with NHL24 closed.

Implementation, integration, activation, installation and installation checks are complete for the moves-only Skill Stick revision. Gameplay testing belongs to the user; actual on-ice control and animation behavior has not been tested here.

### Historical modifier installation — superseded; do not use these controls

The first installed build crashed on startup: `libc.prx+0x1d30d` called its string-length implementation with a null name. The linked launch bank contained 305 unnamed internal records, whereas the original native bank contains no null registry names. Assigning stable ID-derived names to those internal records corrects the loader contract. The six moves-only integration tests now include this regression. The failed overlay and receipt are preserved at `F:\Games\NHL 24\nhl27-moves-only-failed-null-names-20261016`.

The corrected installed build was actually launched: it remained responsive for 64.5 seconds, loaded UI assets beyond the former crash point, and logged no unhandled exception. The test process then closed normally. Evidence: `F:\Games\NHL 24\nhl27-moves-only\startup-check.json` and `deke-trial/moves-only/startup-verified.log`. No gameplay was entered. The rollback script's path-separator bug was also fixed and its real rollback completed successfully before reinstalling.

- Installed and enabled at `F:\Games\NHL 24\game-mods`, using the existing runtime's normal overlay precedence. Launch NHL24 normally. No game executable, original TOC/CAS file, user configuration or existing patch file was modified.
- Installation receipt: `F:\Games\NHL 24\nhl27-moves-only\installation.json`. Installed file hashes, TOC-to-manifest-to-resource resolution, decoded bank hashes and allocation metadata passed. Protected original file hashes also passed after installation. This is an installation check, not a gameplay test.
- 389 existing clip replacements (373 DCT and 16 VBR), plus 33 previously unmatched move clips. The 21 source-only panic reaction clips belong to the separate runtime research and are excluded.
- Launch bank SHA-256: `a9f1de68cb1ff1aa2ce8ca75df564eb277d13a3f8c81f54131c114cdcfbed3b6`. State bank: `a97359324e6d4e38e0b40091c2175f3273e2d68c6bf9371adcafca7b3ef214f6`.
- 116 dependency groups, five native record replacements. Native glide activation/transition arrays and the original toe-drag/bank database predicates, priorities and row counts are preserved. No new runtime enum values or executable patches are installed.
- Source-only toe-drag secondary values 5/6, move types 25–27, bank types 7–10 and glide enum values 4/5 are **not** produced by NHL24. The move-specific adapters use existing native left-stick enumeration choosers to make those asset outputs reachable without changing NHL24's producers.
- Added glide variants use left/right while the native glide-deke node is active; other stick directions use its original controller. Within native toe-drag requests, up selects transition variants and down selects big-stride/same-side variants; middle-origin requests additionally expose lateral variants with left/right. Behind-net bank requests use down for between-the-legs variants and up for the older reverse-bank variant. These are selection modifiers within existing NHL24 move eligibility, not unconditional move triggers. Detailed row mapping is in the integration receipt.
- 41 tests passed across the implementation: 6 moves-only integration invariants (rerun after the crash fix), 10 linker tests, and 25 tag/interface/database/buffer/clip-format tests. All 22 toe-drag database outputs and 10 bank outputs have native selector coverage; all 33 new clip payloads round-trip byte-for-byte after linking.
- Rollback: run `F:\Games\NHL 24\nhl27-moves-only\rollback.ps1` with NHL24 closed. It verifies the overlay inventory and hashes, then preserves the disabled overlay under the receipt directory. Original game files need no restoration.

Rebuild: `python scripts/nhl27/deke_moves_only.py local/nhl27-eboot/compatibility`, then `python scripts/nhl27/deke_expanded_package.py local/nhl27-eboot/compatibility --moves-only`. Verify installation with `python scripts/nhl27/deke_moves_install.py check local/nhl27-eboot/compatibility/deke-trial/moves-only/package "F:\Games\NHL 24"`. The installer refuses an existing overlay instead of overwriting other mods. For revisions, reconcile the installed receipt before replacement.

## Historical broader-runtime objective and working agreement (superseded)

Finish, integrate, enable and install the NHL 27 deking/puck-handling backport in NHL 24 for the user's gameplay testing. Include animations, move inputs, eligibility/interruption logic, motion selection, transitions, panic-turn/pressure reactions, relevant computer-controlled input paths, and the memory/update support needed by those systems.

Do not expand this into a wholesale skating, checking, collision or puck-physics engine port. Adapt their interfaces only where the deking backport requires it. Franchise, Be a Pro, presentation/commentary, PWHL/rules and the wider executable-editor research remain separate/deferred work.

The user has already authorized implementation and installation. Gameplay testing is assigned to the user after the complete package is ready. Continue through the roadmap without ending at intermediate research milestones. Give concise progress messages while working. Stop for user input only when a concrete dependency genuinely requires it; do not describe task size or unresolved reverse engineering as a user blocker. Respect actual permission/resource limits and report them accurately if encountered.

Do not remove compatibility requirements merely to make the linker succeed. Do not label component fixtures, a successful build, a resolved asset graph, or an earlier Free Skate launch as full behavioral parity.

## Historical broader-runtime state at handoff

**The complete port is disabled, uninstalled and not ready for gameplay testing.** Existing partial motion trials are not the finished port. The installed game's files were not changed by the latest implementation work.

- Latest candidate: `local/nhl27-eboot/compatibility/deke-trial/runtime-build/eboot-input-producer-candidate.elf`.
- SHA-256: `620884ee781cdfe2c324304380392941c0cc8cd3efee9ad60cc90091ee706ecf`.
- Build receipt: `runtime-build/input-producer-eboot-build.json`, relative to the trial directory. It declares 162 patches and explicitly says this is not the full port.
- Latest graph: 520 fragments, 1,388 defined objects, no unresolved native IDs, no interface blockers, no graph errors, **54 compatibility requirements**.
- The animation inventory previously reached 389 clips: 373 DCT and 16 VBR. Converted clips still need the complete controller/runtime integration; verify the current inventory before packaging.
- The input candidate includes eight action adapters and 76 producer hooks. Whole-producer, ownership and scheduling parity is not yet established.
- KD compatibility is qualified for the exact eleven imported databases: 105 nonempty trees and 2,100 source/native query cases. This qualification is input-hash limited, not a general format guarantee.
- Continuation runs: 45 panic tests and 53 input tests passed with no skips. PTM's earlier 31-test run was not rerun. The panic suite now includes availability-tail (20,287 source-instruction cases), optional request services (32,640 source-instruction cases), and 257 additional availability-status checks.
- Earlier input and asset tests passed on earlier revisions. Consult their receipts and rerun relevant suites after subsequent integration changes.

The most current summary is `runtime-port-contract.json` → `latest_verified_components`. Older sections in that JSON and `docs/research/nhl27-deke-trial-2026-10-15.md` contain historical hashes, counts and unfinished-work descriptions. Prefer current source, current build/audit receipts and this handoff when those disagree.

## Workspace, inputs and tools

Repository: `E:\Repositories\nhl-modding-studio`; shell: PowerShell.

| Input | Path | SHA-256 |
|---|---|---|
| NHL 27 executable | `W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin` | `10c7bee4c27e5260b417f9574e65cee8338019d08635b3ab9dddab30ba524126` |
| NHL 24 update executable | `F:\Games\NHL 24\game-UPDATE\eboot.bin` | `fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9` |
| NHL 24 base executable | `F:\Games\NHL 24\game\eboot.bin` | `0118720c7e0338e20769063437be5656d1c16262c6b81a688adb659582544e17` |
| CPU-native shad runtime | `F:\Games\NHL 24\nhl24.exe` | `0da12e71bf4f0acea34e758f46b849ecd7f0b90a3e3570ddb1dc71dd5bfc01ce` |

Runtime revision previously logged: `7ce4ce8317b61583798cd2228e098ecbee8bdc0b`, dirty build. The user's earlier no-space path `F:\Games\NHL24` resolves operationally to the spaced directory above; the no-space directory was absent.

Use Python 3.12, Capstone, NumPy and LLVM at `C:\Program Files\LLVM\bin`. Source eboot is ELF; target update is PS4 SELF. Use `ElfImage` and `SelfImage` respectively. `deke_runtime_evidence.checked_image` checks exact binaries. Guest code uses the SysV ABI; Windows fixtures require `__attribute__((sysv_abi))` on guest-facing functions.

Source analysis root: `local/nhl27-eboot`; target: `local/nhl24-eboot/update`. SQLite indexes contain strings, references and unwind function starts. Source relocated pointer tables require relocation addends rather than raw zero pointer bytes. Ghidra source decompilations are under `local/nhl27-eboot/decompiled`; native outputs under `local/nhl27-eboot/animation-target-decompiled`.

Read applicable instructions before edits. There are extensive unrelated dirty/untracked changes: do not reset, clean or broadly commit. Existing policy from the runtime project disallows game-specific emulator hacks; keep these changes in private game assets/executable adapters. Installation outside writable roots may require tool escalation; the user's installation authorization already exists.

Do not dump `source-catalog.json`: it is a very large single-line file. Use targeted Python extraction. Do not run two Ghidra jobs against the same project concurrently. Treat function-shape matches as candidates, not verified equivalents. Capstone register-access metadata can miss AVX writes; inspect actual instructions for hook qualification.

## Read first

All paths below are repository-relative unless otherwise stated.

1. This document and `local/nhl27-eboot/compatibility/deke-trial/runtime-port-contract.json`.
2. `.../runtime-build/input-producer-eboot-build.json` and `.../action-adapted-link-audit.json`.
3. `scripts/nhl27/build_input_eboot.py`, `deke_link_audit.py`, `ant_ptm.py` and the relevant runtime modules.
4. `docs/research/nhl27-deke-trial-2026-10-15.md` for historical evidence, then `nhl27-animation-conversion-2026-10-15.md` and `nhl27-port-compatibility-2026-10-15.md` as needed.

Hereafter, **TRIAL** means `local/nhl27-eboot/compatibility/deke-trial`; **COMP** means `local/nhl27-eboot/compatibility`. Do not literally pass those aliases to scripts without expanding them.

## Roadmap, in execution order

### 1. Close input producer and scheduling parity

Existing readers/writers are not the remaining problem by themselves. Complete every source path that produces their values and prove the table updates occur at the correct points in the frame.

- Work from `TRIAL/target-input-producer-sites.json`, `scripts/nhl27/input_producer_sites.py`, `input_producer_hooks.py`, `scene_schedule_audit.py` and `runtime/input_bridge.c`.
- Cover actions **224, 223, 221, 172, 144, 219, 218 and 139** across human, AI, assignment, unassignment, handoff, inactivity, reset and controller-context changes.
- Preserve current/previous bytes and change timestamps, including release behavior for actors whose controller index is already negative.
- Resolve the remaining gesture producer/phase behavior, including the action-171 signal requirement. Do not mistake the eight main action IDs for the complete gesture dependency graph.
- Source human producer `4a1b3d0` maps to native `2ecb960`; AI `4a1c960` maps to `2eca610`. Existing packed float decoder has exhaustive 65,536-value checks.
- Source controller indexing 0/1 versus native 1/2 remains a qualification issue. Preserve the corrected native third-controller skip at `2ecca7b` and the eight glide changes unless evidence warrants revising them.
- ActionInputUpdateSceneOp was absent from both active scene-operation lists; do not insert a second history advance merely because that class exists.

Memory implementation already expands native GSBool storage `0x1580 → 0x21a0`, with table at `0x1580` and selector at `0x2190`. There are twelve shared tables, 244 boolean entries and forty analog entries. Validate lifecycle and ownership rather than replacing the implementation wholesale.

**Exit:** each action has a documented source producer → native adapter → asset reader path, scheduling/ownership tests pass, and the eight producer/scheduling audit requirements are satisfied by implementation evidence.

### 2. Finish the live panic-turn adapter

Most generic algorithms exist; the missing work is connecting all required game services and producers with correct native layouts and lifetimes.

#### Resume the precise investigation where it stopped

Latest code additions in `runtime/panic_services.c`:

- `nhl24_panic_actor_state_blocked`, `nhl24_panic_actor_blocked`, `nhl24_panic_global_blocked` and its guest wrapper. Source `453cd50/453a9b0/453aa00`; native evidence `27c88b0/27c63e0`. Keep source raw-byte semantics: native normalizes two byte results.
- `nhl24_panic_availability_prefix` and guest wrapper. Source `455ec20`; native layout evidence `27c10d0`. It deliberately performs the source prefix before the deeper availability check. Native's complete routine checks in a different order and cannot simply be substituted.
- Tests: `test_panic_owner_gates.py` and `test_panic_availability.py`. The latter covers a registry callback changing ownership before the action-state query.

**Continuation implementation:** `nhl27_panic_availability_tail` in `runtime/panic_update.c` now preserves source `455e7b0` query order, low-byte call results, full-width scalar checks, captured owner identity, late component acquisition, and the exact-one request option. `test_panic_availability_tail.py` executes the source routine against this implementation in 20,287 cases. Its queries are still callbacks; this is not a connected live native adapter.

`nhl27_panic_request_available` and `nhl27_panic_request_blocked` implement the sixteen-record scans and twice-acquired optional service lookup. `test_panic_request_services.py` executes source `453e350`, `453e080`, `4a72120`, `3461480`, `4a66730`, and `4a70a20` in 32,640 comparisons. The optional manager was traced to **NHLArcadeGameplayComponents**: source creation `4a62e50` names it at `4a62e6d`, publishes singleton `104c4f08`, and member `+28` supplies the request service. No native resolver has been qualified. These functions accept real record/service callbacks; no constant-return live substitute was added.

**Next concrete panic task:** supply the qualified native availability queries and request-service resolver, then connect the tail to the prefix and feature dispatcher. Evidence files in TRIAL:

- `source-panic-availability-tail.asm`, `native-availability-tail.asm`, `native-availability.asm`.
- `source-availability-registry-refs.txt`, `native-availability-registry-refs.txt`.
- `source-request-availability.asm`, `source-feature-enable.asm`.
- `source-panic-flag691-refs.txt`, `source-animation-flags-serialize.asm`, `source-animation-flags-update.asm`.

The last file named `serialize` actually contains the arithmetic producer at source `460630`; its name is misleading. It writes animation flag `+691` at `4607a5`. That flag is read by `4a52090` and `4ffbc0`. Its native field/producer binding remains unresolved. Source `5008db` consumes it before the source-only `+6e3` flag.

Availability registry: source `f226d60`, native `ba07870`, class `cd82ad34`. Source initializer `3bd900`/vtable `e402ed8`; native initializer `41dc90`/vtable `a5d5270`. Source query virtual `b0` becomes native `78`; evaluation context `+68` becomes `+60`. State bytes `31/33` require exact comparisons to one. Native action state lookup uses root getter `2ec80c0`, property `root+68`, getter `3ffbf0`, disabled pointer `+1528` (source `+1060`).

The deeper native routine `27c0d80` contains additional checks and different constants/order. The status `0x11` → native `0x12` mapping is now qualified narrowly for availability: source producer `480e6f8` and native producer `2bf106d` publish those statuses after setting **SignalDekeOnKneesEnter**. Source getter `382660` resolves serialized `+e38` to runtime `+e20`; native getter `3ffbf0` resolves native bank field `+1268` to runtime `+1270`. `panic_status_evidence.py` verifies exact instructions and signal identity and writes `panic-status-evidence.json`. `nhl24_panic_availability_status` translates native 18 to source 17, while keeping native 17 distinct. This is not a general actor-enum mapping. Source flags at actor `b0/b4/b8` still require explicit qualification against native packed bits.

Source request availability: `453e350` accesses singleton `104c4f08` through `4a61a40`, member `+28` through `4a61ba0`, then `4a72120`. The latter tests sixteen records (actor ID at `70 + n*2c`, request kind eight bytes later); helper `3461480` identifies kinds 11, 13 and 15. Native equivalent/service integration remains unresolved. Source feature request gate `453e080` also uses this service. Do not replace it with a constant result.

#### Complete the remaining panic services

- Checking pending contexts: source state member `670` → native `210`; source evaluation `678` → native `218`; source children `7c8/7d0` → native `360/368`. Native `170` is ActionData, not checking state.
- `nhl27_panic_pending_check` exists with 8,192 source-instruction comparisons. Repeated asset guards are intentional because queries can mutate state. Complete the live service adapter.
- `panic_checking_stage.py` stages four missing resource fragments. Implement the rally-request and failed-shoulder-check producers; do not alias rally-request state to the older shoulder-check state.
- Missing state IDs: rally `c08fc9a14b9c9ef3`, interacting `d7d8b06a92894e16`, failed-check `0448c2e885bb4e8b`. ScenarioStatusBoolAsset serialization has been checked against three native records.
- State-condition registry is **ba0eee8**, class `5ac1034c`; it is not `ba07898`. Existing `nhl24_panic_state_context_guest` and state-condition adapter cover this path.
- Finish `panic_update.c` feature, reaction, cancellation, takeoff and alignment services. Generic functions passing tests does not mean their native callbacks are connected.
- Connect lane-history producers and consumers. Existing writer/prediction/zone fixtures pass; lane history means threatened shot/pass lanes, not completed passes. Native hooks: update `2e0a7fc`, reset `2e09950`. Activation is still off.
- Finish the optional trigger/stick-variation consumer path. Source `455d9a0`/`4a2c310` relates to the native inline region in `2eef280`; do not enable a writer with no correct consumer.
- Finish reaction effects, events and puck-rank service bindings. Some source effect classes lack a direct native equivalent; similarly named native classes are not evidence of compatibility.
- Integrate the 21 staged alignment assets and remaining handlers. Locator registry `ac8fc18`, constructor `8c7130`, remove `8c55d0`, native size `770`; do not overlay the source locator layout.
- Install lifecycle hooks only after the frame adapter is complete. Candidate sites: owner creation `2bf036c`, destruction `2bbeb1a`, update `2bf1810`. The native owner `+e8` is occupied: use the existing private 18-owner side table with identity checks.

**Exit:** the complete frame update has real, qualified native services; signals have producers and consumers; lifecycle/reentrancy tests pass; no pending panic/checking/lifecycle compatibility requirements remain.

### 3. Finish PTM motion selection and transitions

Reuse `runtime/ptm_*.c` and the existing hook generators. Much of transition allocation, pose ownership, query scoping, interpolation and frame timing is already implemented and tested but inactive.

- Preserve the exact-input KD qualification in `ptm_kd_qualification.py` and `ptm_kd_qualified.json`. Source and native encode invalid candidate indices differently; tests normalize only invalid sentinels. Keep native encoding at runtime. Changes to any qualified tree/configuration invalidate the certificate.
- Resolve eleven `MotionTypesEnumRef` requirements. The source/native eight enum names and values match (Default 0 through Stop 7), but consumer/tag-filter semantics still need qualification. All trial databases set `FilterMotionInfoTagsByMotionTypesEnum=false`; equality of names alone is insufficient.
- Resolve eleven `PoseFeatureReferenceSystemState` requirements; trial entries are all value one. Establish how the native feature/reference coordinate systems reproduce those semantics.
- Resolve the three `TrajectoryJoint` requirements with the correct native joint and reference-pose mapping.
- Finish source-only parameter consumers: `UglyParams`, `BlendToTargetPoseTrajectory`, `KDAccuracyTestEnable`, `MaxSpeedToPlayStop`, `MaxConstantCycleSpeedDiffPercent`, `MotionWarpTimeHorizonTicksDynamicShort`, `EnableStrideLogic`, `MaxTimeScaleEvent`, `MinTimeScaleEvent`, `MaxDesiredWarpPositionArcAngle`, `MaxDesiredWarpPositionArcDiffNear`, `MaxDesiredWarpPositionArcDiffFar`, and `PreserveAnimationFacing`.
- Finish the `BlendOutBlend` trigger and complete its integration with automatic/manual transitions and the imported asset. Existing implementation components must be validated together before this requirement is cleared.
- Preserve pose/controller ownership differences: native pose refcount/destructor `+18`, controller refcount `+8` and destructor virtual `+20`.
- Preserve existing match extension (`1740 → 17d0`) and blend extension (`2d8 → 330`). Their enable bytes remain off. Do not overlay native match `+1730` with the different source transition layout.
- Relevant hook entrypoints: blend begin `369555`, query `3699ef`, speed `36a019`, pre-child `369a09`, finish `369d2c`, late `369d7f`; match evaluation `31166f`, offset view `3125ce`.

Last PTM investigation: source PTMDatabaseAsset class `a7129111`, vtable `e36dab8`, constructors/copy/destruction `126c60/127780/126ee0`. These are not interchangeable with native serialized record layouts. EBX descriptor offsets are not automatically live native offsets. Newly saved decompilations `001ed480.c`, `00204100.c`, `002043c0.c`, `00130840.c`, `00126940.c` came from `ptm-runtime-layout-targets.txt`. `1ed480` is PCA projection; `204100/2043c0` are cleanup routines, not evidence of motion-enum consumption.

**Exit:** all PTM requirements have either an implemented consumer or narrowly justified source/native equivalence; full transition paths and ownership tests pass with the intended assets.

### 4. Final asset and executable integration

- Regenerate the controller/checking/alignment fragments and combined graph from the completed implementations.
- Maintain an explicit per-move inventory: source move/clip, converted resource, controller, input trigger, selection metadata, transition dependencies and package destination. Reconcile every unmatched move against it.
- Recheck VBR/DCT payloads, DOF/joint remaps, bank allocations and fixups after final packing. Do not repeat conversion from scratch unless evidence shows it is necessary.
- Confirm there are no unresolved imports, interface mismatches, duplicate IDs, unimplemented partitions or unsupported runtime requirements. A zero reference-error count alone is insufficient.
- Rebuild the candidate; verify patch preimages, segment flags, stack alignment, preserved registers/SIMD state, writable storage bounds, relocation safety and asset-scoped activation.
- `.port` protection must remain correct; do not make the whole injection region writable/executable to accommodate data. Existing private BSS has a 64-KiB budget; recompute usage after additions.
- Run relevant input, panic, PTM, asset/link and package tests against the final candidate. Record exact binary hashes, test counts, skipped tests and limitations.

**Exit:** one internally consistent, enabled candidate and package; no compatibility gate is merely suppressed; the package manifest identifies every installed replacement.

### 5. Install and hand over gameplay testing

- Inspect `deke_trial_package.py`, `launch_expanded_deke_trial.ps1` and the existing isolated trial layout before choosing final deployment paths. Older launchers/packages may contain only motion replacements; do not relabel them as the full port.
- Back up every file being replaced and provide a precise rollback manifest. Preserve the user's configuration and saves.
- Install the completed assets and executable together, verify installed hashes against the final build manifest, and confirm the chosen launch configuration actually selects those files.
- Enable the qualified input/panic/PTM components for the intended imported assets. Verify activation through configuration/patch manifests and available non-gameplay checks; do not claim live execution from static evidence alone.
- Leave gameplay control and comparison testing to the user. Provide the exact launcher, installation/rollback locations and a short test list covering new moves, holds/releases, player switching, transitions and pressure reactions.

**Done means:** the complete scoped backport is installed and configured active, ready for user gameplay testing. It does not mean source-identical gameplay has already been demonstrated.

## Reproduction commands

Run from `E:\Repositories\nhl-modding-studio`. These build/audit outputs, not an installation.

```powershell
python scripts/nhl27/build_input_eboot.py --producer-sites local/nhl27-eboot/compatibility/deke-trial/target-input-producer-sites.json
python scripts/nhl27/deke_link_audit.py local/nhl27-eboot/compatibility --action-adapters
python -m unittest discover -s scripts/nhl27 -p 'test_input*.py'
python -m unittest discover -s scripts/nhl27 -p 'test_panic*.py'
python -m unittest discover -s scripts/nhl27 -p 'test_ptm*.py'
python -m unittest discover -s scripts/nhl27 -p 'test_ant_ptm.py'
```

Use meaningful targeted tests during implementation, then the relevant combined suites at integration. Private-input tests can skip when their binaries/toolchain are unavailable; inspect skips rather than counting them as evidence.

Inspection examples:

```powershell
python scripts/nhl27/trace.py local/nhl27-eboot function 0x455e7b0
python scripts/nhl27/trace.py local/nhl24-eboot/update function 0x27c0d80
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/nhl27/decompile.ps1 -Targets local/nhl27-eboot/ptm-runtime-layout-targets.txt
```

The trace tool's narrow-operand scan was corrected to include 1/2/4/8-byte encodings and overlapping candidates. Keep that fix. `oracle_fixture.assembly` now accepts an optional instruction-address → label mapping for relocating jump tables.

## Guardrails for the fresh session

- Preserve and extend the existing implementation; do not restart the research or repeat all previously passed tests without a reason.
- Treat unknown fields, callbacks and enum mappings as unresolved until demonstrated. Do not use constant-return stubs to get a package to launch.
- Keep source/native ABI differences explicit, including raw-byte comparisons, callback ordering and reacquisition after mutation.
- Existing trial directories contain junctions to installed game directories. **Never recursively delete through those junctions.**
- Do not claim the full port is active until installation and activation are actually verified.
- No percentage estimate is justified by the number of open requirements; their sizes vary substantially.

## Suggested opening instruction for the next session

> Read `docs/plans/nhl27-deke-backport-session-handoff-2026-10-16.md` and the current runtime/build/audit receipts. Resume the existing NHL 27 → NHL 24 deking backport, preserving the narrowed scope and existing implementation. Finish producer/scheduling parity, panic runtime services and reactions, PTM compatibility/transitions, then integrate, enable and install the complete package. Gameplay testing is mine. Continue through implementation and installation without stopping at another research checkpoint unless you genuinely require something from me. Do not bypass compatibility checks or claim unverified parity.

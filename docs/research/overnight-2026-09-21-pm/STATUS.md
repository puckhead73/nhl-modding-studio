# Research lane status — overnight 2026-09-21 pm (continuation)

> **Superseded in part — 2026-09-22.** The coordinator's [delivery review](../../validation/research-20260921-pm-delivery-review.md)
> (F1–F3) withdraws the Xbox slot-extent conclusions of handoff 17, restates handoff 18's EA-MP3 reconstruction as a hypothesis and
> reconciles handoff 03's coverage wording. Read
> [the F1–F3 correction checkpoint](../overnight-2026-09-22/01-correction-checkpoint-f1-f3.md) before consuming rows 03 / 17 / 18.
> All raw evidence (`data/*.json`, probes) is unchanged.

Campaign brief: the fresh overnight research prompt in
[overnight-research-2026-09-21-review-r1.md](../../validation/overnight-research-2026-09-21-review-r1.md) (correction checkpoint → resize
contract hardening → league blockers → audio → Teams → Players → Equipment → Miscellaneous; parity operation-by-operation; no launches,
no writes, no commits). Codex coordinates / reviews; the developer lane owns application code (its active package at start:
OVERNIGHT-DEV-20260921-R1, receipt 16 delivered — resize representability + swap ownership; all `dev-*` claims `complete`).
This lane owns `docs/research/overnight-2026-09-21-pm/**`, `local/runs/overnight-research-2026-09-21-pm/**` and
`local/coordination/overnight-2026-09-21/claims/research-r4[2-9]*.json` (the shared claims directory — kept so the developer's boundary
check finds them), plus dated correction notes in its own earlier campaigns' files. Baseline: branch `feat/phase5-asset-tooling`, HEAD
`8d60a6d`; `docs/research/` untracked; the dirty working tree is preserved; nothing of the developer's is touched.

Startup read (16:46–16:55 MDT): repository instructions, the review R1 and its prompt, the 2026-09-21 atlas / STATUS / queue / cards /
handoffs 02, 03, 04, 11, 14, 15, the r32b / r31 data, the developer review R1 and receipt 16, the developer claims, Studio's `gnf.rs` /
`texlib.rs` resize gates (read only).

## R42-0 — Correction checkpoint (16:55–17:05 MDT) — handoff [01](01-correction-checkpoint-r1-r3.md)

Plain-language: the three review corrections are applied as dated notes in the previous campaign's files (nothing deleted), the three user
cards are rewritten as observation cards with controls, and the two deleted scratch measurements (Xbox `DW8` / `DW12` rule census, GNF
pitch sample) are replaced by runnable probes that are now running over the full trees.

Changed paths — 2026-09-21 campaign (research-owned): handoffs 11 / 14 / 15 (top notes + one table-row annotation in 11), USER-CARDS,
DEVELOPER-QUEUE rows 1 / 11 / 13, STATUS (blocker list + closing section), 00-ATLAS (rows 11 / 15, Q-R32-1, §5). New (this campaign):
00-ATLAS, STATUS, DEVELOPER-QUEUE, USER-CARDS, handoff 01, probes `r42_rw4xb2_slot_join.py`, `r43_gnf_header_census_per_title.py`,
claim `research-r42-corrections-r1-r3`. Executed: r42 smoke runs (Legacy 300 / 600 files, NHL 07 300 files), r43 smoke run (NHL 20
1 500 files); hex dumps of `stadium/ice_intl_container_0.rx2` (Legacy), `indicator/orig_indicator_0_cm.rx2` (NHL 09),
`logo/texlib_27_0_0.rx2` (Legacy). **Background (this lane's only processes):** r42 over the 10 X360 trees (started 16:57, ≈ 1 h) and
r43 over the 7 PS4 trees in the order 20 / 15 / 17 / 18 / 19 / 16 / 21 (started 16:54, ≈ 4 h) — logs `r42.log` / `r43.log` in the
scratchpad; outputs under `data/`. Not executed: launches, writes to any source, Studio binaries, commits.

**Correction checkpoint published (17:05 MDT).** Continuing to resize contract hardening without a review stop, per the brief.

## R45 / R46 — Audio continuation (prompt step 4, first two items; 17:05–17:15 MDT) — handoffs [04](04-ctx0-v31-script-ids-bind-to-gas-blob-id-words.md), [05](05-ctx0-header-words-across-versions.md)

Plain-language: the NHL 11 / 12 script-id question is closed — the id in a context record is the id word at the start of the script blob
(a lookup, not a position), and every reference on every set resolves; and the three undecoded header numbers are shown to be the same
three fields in both file versions, with candidate labels (priority-like, time-like, kind code) and the module XML attributes recorded.
Taken while the texture censuses run in the background (steps 2 / 3 continue when they finish).

Changed paths (new): handoffs 04 / 05, `data/r45-…json`, `data/r46-…json`, probes `r45_…py`, `r46_…py`, claims `research-r45-ctx0-v31-gas-ids`,
`research-r46-ctx0-header-words`; atlas rows 04 / 05 + Q-R24-1 note. Executed: r45 (1 dev run + 1 final), r46 (2 runs), scratch value
censuses in the shell (not retained). Not executed: `.gas` decoding, launches, writes.

## R42 / R44 / R47 — Resize contract hardening (prompt step 2; r42 finished 17:22, written up 17:22–17:33 MDT) — handoff [02](02-rw4xb2-slot-join-and-resize-bounds.md)

Plain-language: the Xbox 360 `DW8` question is closed exactly (the word is the file-table index of the texture's pixel entry — every
descriptor on every title), NHL 07 / 08 are brought under the same measured rules (NHL 08 is RefPack-wrapped; 110 of its jersey
libraries are undecodable and must be refused by name), and the developer gets the field-bound / shipped-range / hardware-limit
separation with 40 boundary vectors and a slot-size oracle.

Changed paths (new): handoff 02, `data/r42-…`, `data/r44-…`, `data/r47-…`, probes `r42_…`, `r44_…`, `r47_…`, claims `research-r42-rw4xb2-slot-join`,
`research-r47-resize-bounds`; 2026-09-21 handoff 11 (second dated note: resolution); atlas row 02 + Q-R32-1 closed + Q-R44-1. Executed:
r42 (4 dev runs + 1 full), r44 (1 smoke + 1 full), r47 (1), the NHL 08 archive-member identity / entropy check and the Legacy `DW13`
exception lookup in the shell. Not executed: writes, launches.

## R48 — PS4 typed event parameters (prompt step 4, third item; 17:15–17:41 MDT) — handoff [06](06-ps4-typed-event-parameters-and-name-phrase-pools.md)

Plain-language: the PS4 event definitions parse with parameter types on all seven titles; the name phrases consume an integer
`surname_ID` (20–21) / `player_ID` (15–19) and draw from keyword-tagged pools — the same mechanism as the established team binding — but
the DB column that yields the keyword is still not found (the one discriminating test is negative), NHL 20's install ships a 2-keyword
stub, and NHL 15's phrase items need their own parser.

Changed paths (new): handoff 06, `data/r48-…`, probe `r48_…`, claim `research-r48-ps4-typed-params`; atlas row 06 + Q-R40-1 note + Q-R48-1/2;
DEVELOPER-QUEUE rows 3–6. Executed: r48 (2 dev runs + 1 full), shell tests of §4 (not retained), `eb_list --grep` over the 13 NHL 20 archives,
one `eboot.bin` string scan. **Background:** r43 restarted 17:34 with the concatenated-stream walk (NHL 15 ships 1 115 two-stream `.gnf`
files — found in its first pass), order 20 / 16 / 17 / 18 / 19 / 21 / 15; the first-pass `r43-gnf-nhl15.json` stays until the rerun replaces it.

## R49 — EA-Opus preview decode contract (prompt step 4, fourth item; 17:38–17:43 MDT) — handoff [07](07-ea-opus-preview-decode-contract.md)

Plain-language: the PS4 NHL 18–21 commentary samples decode with the ffmpeg already on this machine once their length-prefixed Opus
packets are wrapped in a standard Ogg container — 24 of 24 vectors, lengths exactly 960 samples per packet, declared durations honoured by a
trim. This is a preview contract only; no encoder exists and replacement stays refused.

Changed paths (new): handoff 07, `data/r49-…`, probe `r49_…`, claim `research-r49-ea-opus-preview`; atlas row 07 + Q-R28-1 note; queue row 7.
Executed: r49 smoke (3 vectors) + final (24 vectors); temporary Ogg / WAV files removed (scratch dir verified empty and removed). Not executed:
listening, encoders, writes.

## R50 — Teams 2.7 PS4 goal horns (prompt step 5, Teams / audio; 17:43–17:49 MDT) — handoff [08](08-ps4-goalhorn-catalogue-join.md)

Plain-language: the arena table's goal-horn number resolves to a named row of the shipped `goalhorns.csv` catalogue on NHL 17–21 (club
horns by "+ 1", generic horns directly), so the Team page can name horns; the step from catalogue to sound is a Harmony patch parameter
(`GoalHornID`) whose data lives in `sfx_ptc.big`, not in any sound-bank index — left open with the file named.

Changed paths (new): handoff 08, `data/r50-…`, probe `r50_…`, claim `research-r50-ps4-goalhorn-catalogue`; atlas row 08 + Q-R9-1 note; queue
row 8. Executed: r50 (1 run), shell exploration of the NHL 21 CSV / DB / banks, one `eb_list --extract` of the horns XML to the scratchpad
(read, then removed). Not executed: writes to sources, launches.

## R51 — Teams 2.5 arena venue inventory (prompt step 5; 17:49–17:54 MDT) — handoff [09](09-arena-venue-parameter-inventory.md)

Plain-language: every arena-table field on every anchor is listed with its range, constancy and a name-based class, grouped by the seven
title signatures; the PS4 colour-zone fields are RGB triples. Inventory only — no field's runtime meaning is claimed.

Changed paths (new): handoff 09, `data/r51-…`, probe `r51_…`, claim `research-r51-arena-venue-inventory`; atlas row 09; queue row 9.
Executed: r51 (1 run). **Background:** r43 still on NHL 20 (its first title; the walk is I/O-bound on H:).

## R52 — Equipment 4.x component sets (prompt step 5; 17:55–17:59 MDT) — handoff [10](10-gen7-equipment-component-sets.md)

Plain-language: for each gen7 equipment family the census says exactly which texture files every shipped id has (and which ids ship
without the full set), that meshes are shared per part rather than per id, and that PS3 and Xbox 360 carry the same sets — the input a
reassignment validator and a new-id registration need.

Changed paths (new): handoff 10, `data/r52-…`, probe `r52_…`, claim `research-r52-gen7-equipment-components`; atlas row 10 + Q-R38-1 note;
queue row 10. Executed: r52 (3 runs — grammar refinement and the PS3 Legacy path). **Background:** r43 (NHL 20 first) still running.

## R53 — Players 3.x gen7 asset containment (prompt step 5; 18:00–18:02 MDT) — handoff [11](11-gen7-player-portrait-head-containment.md)

Plain-language: on every gen7 title the roster's portrait and head ids are checked against the shipped files — heads are complete from
NHL 10 on, NHL 09 lacks five bespoke heads, the pool-head numbering changes era (10 000+ vs 60 000+), and the `hasportrait` flag is a DB
switch that disagrees with the files on up to 127 players.

Changed paths (new): handoff 11, `data/r53-…`, probe `r53_…`, claim `research-r53-gen7-player-assets`; atlas row 11; queue row 11.
Executed: r53 (2 runs). **Background:** r43 (NHL 20) still running.

## R54 — Misc 5.4 PS4 song catalogue (prompt step 5; 18:03–18:05 MDT) — handoff [12](12-ps4-songs-catalogue.md)

Plain-language: the PS4 17–21 installs name every selectable team song (title key, composer, genre) in a CSV, which answers the "track
titles" unknown for PS4; the team rows' song cells and the music banks do not join that catalogue by its ids, so it is a listing, not a
binding.

Changed paths (new): handoff 12, `data/r54-…`, probe `r54_…`, claim `research-r54-ps4-songs-catalogue`; atlas row 12; queue row 12.
Executed: r54 (3 runs), one shell check of the music banks' id spaces. **Background:** r43 (NHL 20) still running.

## R55 — League blockers status (prompt step 3; 18:05–18:07 MDT) — handoff [13](13-league-blockers-status-and-deferred-cards.md)

Plain-language: the four named league blockers are dispositioned — two measured (NHL 07 / 08 textures; archive dependencies), two shown
to be outside the data files (CHL participant selection; NHL 09 settings, which are front-end label formats rather than a table) with
observation cards.

Changed paths (new): handoff 13, claim `research-r55-league-blockers-status`; atlas row 13 + Q-R36-1 note; USER-CARDS (L-CHL-1, L-SET-09).
Executed: `grep` over the NHL 21 data-file tree (JSON / CSV / XML), reads of `league.csv` / `team.csv` / `competition.json` /
`league_state.json`; `xex_image.py` on the NHL 09 `default.xex` to the scratchpad (20.7 MB, removed), a string scan and the
`p1bs_gen7_setting_table.py` anchor walk on it (no table). Not executed: NHL 08's executable, launches, writes.

## R56 — Teams 2.6 gen7 kit selectors (prompt step 5; 18:08–18:11 MDT) — handoff [14](14-gen7-kit-selector-join.md)

Plain-language: the gen7 jersey table's three selector numbers are the kit texture file names — every row resolves on every title from
NHL 11 on, on both consoles — so the developer can list, validate and re-point kits statically; custom teams and NHL 09 / 10 are the
refusals.

Changed paths (new): handoff 14, `data/r56-…`, probe `r56_…`, claim `research-r56-gen7-kit-selectors`; atlas row 14; queue row 13.
Executed: r56 (1 run) after shell key discovery on Legacy. **Background:** r43 (NHL 20) still running.

## Checkpoint 18:14 MDT (durable; resumable)

Published so far: handoffs 01 (corrections), 02 (Xbox `DW8` join + resize bounds), 04 (v3.1 script ids), 05 (`CTX0` header words), 06 (PS4
typed name parameters), 07 (EA-Opus preview), 08 (PS4 goal-horn catalogue), 09 (arena venue inventory), 10 (equipment component sets),
11 (player portrait / head containment), 12 (PS4 songs catalogue), 13 (league blockers status + cards), 14 (gen7 kit selectors). Pending:
handoff 03 (GNF per-title census) — r43 is on NHL 20 (its first title; ≈ 140 000 files on the slow H: drive), then 16 / 17 / 18 / 19 / 21 /
15; each title's JSON lands under `data/` as it completes and handoff 03 is written from whatever has landed at the final checkpoint.
Claims: `research-r42-corrections-r1-r3`, `-r42-rw4xb2-slot-join`, `-r45-…`, `-r46-…`, `-r47-…`, `-r48-…`, `-r49-…`, `-r50-…`, `-r51-…`, `-r52-…`,
`-r53-…`, `-r54-…`, `-r55-…`, `-r56-…` (all `ready`). Scratchpad holds only `dump_xb2.py`, the two smoke-test JSONs and the r42 / r43 / r44
logs (regenerable; removed at closeout). No background process other than r43.

## R43 — GNF per-title census, first checkpoint (NHL 20 landed 18:16; written up 18:18 MDT) — handoff [03](03-gnf-per-title-census-and-resize-bounds.md)

Plain-language: the PS4 texture-header rules hold on the real NHL 20 install on every file whose format is modelled — BC6H included now —
and the pitch rule is tested on every file; NHL 20's biggest shipped texture is 8 192 × 4 096. NHL 15's first pass found 1 115 files that
are two textures back to back, which the writer must refuse by name. The other titles are still walking and will be added to the same
handoff.

Changed paths (new): handoff 03, `data/r43-gnf-nhl20.json` (+ the first-pass `r43-gnf-nhl15.json`), claim `research-r43-gnf-per-title`
(`active` until the remaining titles land); atlas row 03 + Q-R31-1 note + Q-R43-1; queue row 14.

## R58 — PS4 context records (parity, prompt step 6; 18:24–18:29 MDT) — handoff [15](15-ps4-ctx0-context-records-decoded.md)

Plain-language: the PS4 games' commentary context files parse with the same model as the last-generation consoles, and every reference
in them resolves on all seven titles — so the Contexts card can be shown for PS4 too. Two undecoded pieces remain (a per-record block table
and the bytecode), as on gen7.

Changed paths (new): handoff 15, `data/r58-…`, probe `r58_…`, claim `research-r58-ps4-ctx0`; atlas row 15; queue row 15. Executed: r58 (4 runs
incl. two development runs); one record dump in the shell. **Background:** r43 on NHL 17 (16 done in 56 s, 20 done).

## R59 — Misc 5.3 overlay inventory (prompt step 5; 18:30–18:33 MDT) — handoff [16](16-scorebug-overlay-inventory.md)

Plain-language: the scorebug and other in-game overlays are inventoried per title — on PS4 as Scaleform movies plus separate texture
files (the textures are already replaceable), on the older consoles as numbered Ion packages; nothing is decoded.

Changed paths (new): handoff 16, `data/r59-…`, probe `r59_…`, claim `research-r59-overlay-inventory`; atlas row 16; queue row 16. Executed:
r59 (1 run) + two archive listings in the shell. **Background:** r43 still on NHL 17.

## R43 — GNF per-title census COMPLETE (all seven titles landed by 05:26 MDT, 22 September) — handoff [03](03-gnf-per-title-census-and-resize-bounds.md)

Plain-language: the PS4 texture rules now hold on every one of the seven games, measured file by file, not sampled — between 48 thousand
and 145 thousand files each, all passing. Two games (NHL 15 and NHL 19) ship files that hold two textures back to back; the other five ship
none, so a writer only has to handle that form there. The biggest texture EA ships grows to 8 192 × 4 096 from NHL 18 on.

Changed paths: handoff 03 (complete table + refreshed summary, rules and bounds sections), `data/r43-gnf-nhl1[5-9].json` /
`-nhl21.json` (new), claim `research-r43-gnf-per-title` → `ready`; atlas row 03 + Q-R31-1 closed + Q-R43-1 note.

## R60 — X360 slot extent and packed-tail oracle (prompt step 2; 18:38–05:28 MDT) — handoff [17](17-rw4xb2-slot-extent-and-packed-tail-oracle.md)

Plain-language: for Xbox 360 textures the space reserved for the pixels is always at least as big as the pixels themselves and always a
multiple of 8 KiB, so the developer’s "fit it or regrow it" resize rule is safe. How EA lays out the *small* mip levels inside that space
is measured (a table of how many 4 KiB pages the tail occupies for each shape, the same on all four titles examined) but not reduced to a
formula — that stays a developer implementation of the console’s tiled addressing, with this table as its check.

Changed paths (new): handoff 17, `data/r60-rw4xb2-slot-data-extent.json`, probe `r60_rw4xb2_slot_data_extent.py`, claim
`research-r60-rw4xb2-slot-extent`; atlas row 17 + Q-R60-1; queue row 17. Executed: r60 (2 trial runs into the scratchpad, deleted; 1 final
run over NHL 09 / 10 / 13 / Legacy) + shell analyses. **Background: none** (r43 finished).

## R61 — EA-MP3 block structure, preview blocked (prompt step 4; 05:28–05:32 MDT, 22 September) — handoff [18](18-ea-mp3-block-structure-preview-blocked.md)

Plain-language: the speech on NHL 15–17 is EA’s own MP3 variant, and this handoff measured exactly how it is stored: nine MP3 frames per
block, but only the first one keeps its header. That is why the preview trick that works on NHL 18–21 cannot work here — tested on all
24 vectors, none decode. The page can still list these sounds with their length and bitrate and say the preview is unavailable; the next
piece of format work is rebuilding the missing headers, which would also unlock the last-generation class.

Changed paths (new): handoff 18, `data/r61-ea-mp3-block-structure.json`, probe `r61_ea_mp3_block_structure.py`, claim
`research-r61-ea-mp3-structure`; atlas row 18 + Q-R28-1 update; queue row 18. Executed: three exploratory scripts in the scratchpad
(deleted) + one probe run; temporary `.mp3` / `.wav` removed by the probe. **Background: none.**

## R62 — gen7 arena / ice asset join (prompt step 5, Teams; 05:33–05:36 MDT, 22 September) — handoff [19](19-gen7-arena-venue-asset-join.md)

Plain-language: an arena’s art id in the database is the file name of that arena’s model and jumbotron on disk — true for every arena row
on every last-generation title, with nothing missing. The centre-ice logo and the rafter banners turn out to belong to the team, not the
arena, which matches what the PS4 data shows. The remaining ice layers cannot be pinned to a specific field from the data, so they are
recorded as shipped files with no established binding rather than guessed.

Changed paths (new): handoff 19, `data/r62-gen7-arena-asset-join.json`, probe `r62_gen7_arena_asset_join.py`, claim
`research-r62-arena-asset-join`; atlas row 19 + Q-R62-1; queue row 19. Executed: three probe runs (two superseded; the ranking and the
`icesurface` grammars were corrected) + two shell listings. Scratchpad: `r62-test.json` (removed at closeout). **Background: none.**

## R63 — PS4 portrait containment (prompt step 5, Players; 05:37–05:39 MDT, 22 September) — handoff [20](20-ps4-player-portrait-containment.md)

Plain-language: on PlayStation 4 a player’s art id is the name of their portrait file, and — unlike the older consoles — the database and
the shipped files never disagree: no player is marked as having a portrait that is missing. Each game also ships thousands of portraits no
player uses, which is a pool rather than a problem. So the player page can show portraits on PS4 with no special handling, and replacing
one is the existing texture route (NHL 15 is the exception: its portraits are PNG).

Changed paths (new): handoff 20, `data/r63-ps4-player-portrait-containment.json`, probe `r63_ps4_player_asset_containment.py`, claim
`research-r63-ps4-portrait-containment`; atlas row 20; queue row 20. Executed: one trial run (two titles, scratchpad JSON deleted) + one
full run + two shell listings. **Background: none.**

## R64 — PS4 arena / venue asset inventory (prompt step 5, Teams parity; 05:40–05:42 MDT, 22 September) — handoff [21](21-ps4-arena-venue-asset-inventory.md)

Plain-language: the PlayStation 4 games keep each arena in a folder named after its market rather than after a number in the database, and
nothing in the arena row spells those folder names — so the rule that works on the older consoles does not transfer, and the PS4 arena page
has to use the vault reference that earlier work already decoded. The shared ice pieces (centre logos, centre lines, power rings) are
numbered and can be listed and replaced today.

Changed paths (new): handoff 21, `data/r64-ps4-arena-asset-join.json`, probe `r64_ps4_arena_asset_join.py`, claim
`research-r64-ps4-arena-asset-join`; atlas row 21 + Q-R64-1; queue row 21. Executed: one trial run (NHL 21, scratchpad JSON deleted) + one
full run + one shell survey. **Background: none.**

## Final checkpoint — 22 September 2026, 05:43 MDT

**State:** the campaign is at handoff 21 and every handoff is published with its claim (`research-r42` … `research-r64`, all `ready`),
atlas row, developer-queue row and STATUS section. The prompt's priorities are covered: corrections (01), resize bounds and descriptor
exceptions (02, 03 complete on all seven PS4 titles, 17), league blockers (13), audio (04–08, 12, 18), Teams (09, 14, 19, 21), Players
(11, 20), Equipment (10), Miscellaneous (16). Visual / runtime questions are deferred to the user cards, unchanged.

**Coverage and residuals at this checkpoint.** X360 textures: the descriptor → slot join is 100 % on 08–Legacy and 10 666 / 10 675 on
NHL 07; the slot is never smaller than its content (0 / 89 267) but the packed-mip *byte layout* is bounded, not derived (Q-R60-1), and
the 110 NHL 08 jersey libraries stay undecodable (Q-R44-1). PS4 textures: complete, no residual beyond the multi-stream form (Q-R43-1,
now known to be NHL 15 / 19 only). Audio: EA-Opus previews decode; EA-MP3 is structurally measured and blocked (header reconstruction is
the named next step, shared with gen7 EALayer3 v1); EA-XMA and codec `0x09` untouched; no encoder anywhere. Contexts: gen7 and PS4 parse
end to end; the selection rule and the bytecode are unobserved. Arenas: gen7 joins at 100 %, PS4 has no file-name rule (Q-R64-1). Players:
PS4 portraits contain cleanly; gen7 keeps its 93 / 127 flag contradictions.

**Cleanup:** the session scratchpad is empty — the r42 / r43 / r44 / r60 logs, the two smoke-test JSONs, `dump_xb2.py`, the r60 / r62 /
r63 / r64 trial JSONs, the publishing scripts and the r61 temporary audio directory are all removed. **No background process of this lane is running** (r43 and r60 both wrote their final JSONs; other lanes' Python processes on this machine were never touched).
Durable evidence (handoffs, `data/*.json`, probes, claims) is retained; `docs/research/` and `local/` remain untracked, as before.

**One exact next task:** reconstruct the absent MPEG frame headers for the EA-MP3 class (handoff 18 §3) — synthesise the eight missing
headers per data block from the first one and re-derive the frame boundaries from the granule side info, validating against the 24
vectors in `data/r61-ea-mp3-block-structure.json`; the same routine should then be tried on the gen7 EALayer3 v1 samples.


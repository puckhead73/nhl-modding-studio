# Receipt 39 (consolidated) — overnight 27 Sep: XMA helper and previews, pair-quota schedules, style-label fix, more audio, old-title logo resizing, portrait / head reassignment

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing game-verified;
nothing published.** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged; nothing staged, committed or pushed).
Direction: [overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md). Detailed
receipts: [32](32-xma2-helper-build.md), [33](33-xma-listing-previews.md), [34](34-schedule-pair-quotas.md),
[35](35-style-label-scoping.md), [36](36-audio-admissions-r82-r131.md), [37](37-old-logo-resize.md),
[38](38-portrait-head-reassign.md), and after this receipt's bundle [40](40-goalie-gear-file-positions.md).

## 1. What users can now do (plain language)

- **Hear Xbox 360 XMA audio (diagnostic previews).** Studio has its own XMA helper (a minimal, licence-clean build of
  FFmpeg 8.1.3), shipped beside Studio in the review bundle. With it:
  - Xbox 360 announcer and play-by-play (NHL 09 / 10), menu music (NHL 09–12), sound banks on NHL 10 → Legacy, and
    now every Xbox 360 NHL 09 sound bank preview.
  - Without the helper the card says so and nothing pretends to play; a running decode can be cancelled.
  - Every research sample decodes to the exact length, and every sample research hashed is bit-identical.
- **More PS3 audio:** menu music of PS3 NHL 09, 10, 12 and 13 previews from inside the game's archives.
- **Generate a league schedule from your own games-per-pair table** (League → Source schedule → Generate… → pair
  quotas). Odd counts ask who hosts the extra game. The table keeps its size, no club plays twice on a generated
  day, and apply / undo / save work like any edit.
- **Fixed:** skater style names on the Lines page and roster table now match each title. Before, NHL 08 / 09 showed
  a different name than the player page, and NHL 10 called forwards "Two-Way Defenseman".
- **Fixed:** schedule generation no longer overwrites the PS4 NHL 18–21 Austrian table's tagged final block; it
  refuses instead.
- **Resize team / league logos on NHL 07 → 11** (Xbox 360 and PS3 package form), not only NHL 12+. Crops and odd
  forms refuse with the reason.
- **Point a player at another shipped portrait or head** (Player assets → Portrait & head → Reassign). Check shows
  the files that make it valid or why not; Apply is one reviewed, undoable edit.
- **Goalie masks on NHL 08 / 09:** the paint number is `maskcolor`, and the fields say so.

## 2. Packages and review bundle

| # | Package | Result |
|---|---|---|
| 32 | XMA2 helper build (toolchain installed with the user's approval) | built, reproducible, 83 / 83 PCM bit-identical |
| 33 | XMA speech / music listing, `.ast` XMA streams, Cancel, missing-helper states | done (dev) |
| 34 | user-supplied pair-quota schedules + round-tag defect (+ R123 Q11 / Q12 addendum) | done (dev) |
| 35 | style-label scoping (defect R122 S1 / S2) | fixed (dev) |
| 36 | PS3 NHL 09 / 10 / 12 / 13 music (R82); X360 NHL 09 banks (R131) | done (dev) |
| 37 | logo resize on the NHL 07 → 11 package form (R132) | done (dev) |
| 38 | portrait / head reassignment (R124); goalie-mask G1 (R125) | done (dev) |
| 40 | goalie gear file check: NHL 12 pads position (defect), NHL 08 / 09 mask paint (after the r39 bundle) | fixed / done (dev) |

**Review bundles:** `E:\nhlms-review-build-r33-2026-09-27` (receipts ≤ 33, built 05:48) and the final bundle of this run
(§4). The 26 Sep bundle is unchanged.

## 3. Toolchain and helper provenance

- MSYS2 base `msys2-base-x86_64-20260611.tar.xz` (SHA-256 `a2d047e8…c34221`, signature verified against the key the
  MSYS2 site publishes), extracted to `C:\nhlms-toolchain\msys64`; packages via pacman (GCC 16.2.0-4, binutils 2.47,
  mingw-w64 CRT / winpthreads 14.0.0, NASM 3.02, make 4.4.1). No PATH / registry / service change. Removal: delete
  `C:\nhlms-toolchain\`.
- FFmpeg `ffmpeg-8.1.3.tar.xz` (SHA-256 `7138d28c…78a3`, signature verified with the release key
  `FCF9 86EA 15E6 E293 A564 4F10 B432 2F04 D676 58D8`), unpatched; configure line and scripts in `tools/xma2-helper/`.
- Helper `nhlms-xma2-helper.exe` SHA-256 `dc3533ed3234782703ebeab72034d9298b920b3e1fbb7c3fb9a9d2adfd818c21` (two clean
  builds identical); `-L` = LGPL 2.1+; `CONFIG_GPL / VERSION3 / NONFREE / NETWORK` = 0; imports only Windows DLLs; the
  bundle keeps licences, the signed source tarball and the build record beside it.

## 4. Final review bundle — `E:\nhlms-review-build-r39-2026-09-27` (use this one)

| Item | Value |
|---|---|
| Executable | `nhl-studio.exe` 49 524 736 bytes, SHA-256 `cd00650b36e7c70f8373d22a411acfbde1ee4b5feb411c585047f23bd93a025b` (= `target\release` at copy) |
| Built | 27 Sep 2026 07:39:24 → 07:42:04 MDT, `npm run build -- --no-bundle`, existing release cache reused |
| Inputs | HEAD `8d60a6d` + 852 dirty inputs, manifest SHA-256 `d987eee5…02ba`; re-hashed after the build: identical |
| `defs\` | 121 files, tree `2f763a6e…0cd8` · `helpers\` 21 files, tree `1caa5f4d…f5c4e`, helper `dc3533ed…8c21` · frontend 55 files, tree `10ccb38d…a4d7` |
| Launch | run `nhl-studio.exe` from that folder (`README-REVIEW.txt`); keep `defs\` and `helpers\` beside it; no environment variable |
| Contents | receipts through 38 (receipt 40, a display-only file-check fix, came after it) |
| Records | `review-build-r39/build-identity.json`; review plan §2 "Third review bundle"; tracker rows 34–38; MORNING-REVIEW cards point here |

Never launched by the developer. The 26 Sep and r33 bundles are unchanged.

## 5. Checks (final integration, `docs/validation/evidence/overnight-development-2026-09-24/logs/r39-*.log`)

Bounded aggregate (not the whole workspace), 07:20–07:40:
- `tdb-snd` 22 + `helper_runner` 5, `tdb-rx2` 120, `tdb-enums` 7, `tdb-speech` 5, `tdb-domain-validation` 16, `tdb-speex` 3: green.
- studio-domain: lib 225, `schedule_edit` 14, `style_label_scoping` 3, `view` 16, `player_reassign` 1, `nationality_labels` 4,
  `import_change_set` 3, `equipment_goalie_gear` 1, `gen7_music_r82` 1, `gen7_sfx_banks` 1, `x360_nhl09_xma_banks` 1,
  `easpeex_preview` 1, `eaxma_speech_preview` 1: green after **three stale assertions from tonight's own changes** were
  corrected. Two said PS3 NHL 10 music is refused, which receipt 36 deliberately changed; the bindings root-count
  tripwire went 164 → 165. The first aggregate run shows them failing; the rerun is green.
- studio-api: lib 31, `registry` 7, `schedule_edit` 1; bindings in sync.
- Frontend: vitest 261 tests, `tsc` exit 0 (`r39-frontend.log`).
- Real-tree / private-fixture cases, run per package with `--include-ignored` and logged in receipts 32–38: R84 83 / 83,
  R92 vectors, R82 12 / 12, NHL 09 full sweep, logo descriptor 4 634 / 4 634 and fixture sweep 168 packages, goalie gear
  on every anchor + genuine NHL 20, reassign on Legacy + PS4 21, schedule quotas on anchors (Legacy Swiss; PS4 NHL 18 / 21 Austrian refusal) with the
  existing genuine-NHL 20 schedule case still passing (the quota generator itself was not run on NHL 20).
- Not run: the full workspace suite, repo-wide `cargo fmt` / `clippy` (baseline defects, C0.2), any desktop / game /
  listening run.

## 6. Important failures and limits

- Nothing was desktop-observed, listened to or game-tested; every card is open.
- Found and fixed during the run: receipt 30's recipe named a non-existent option and lacked threads / static flags;
  the first helper build had no muxer (configure only warned — the script now fails on that); non-reproducible PE
  time stamps; `.ast` terminal zero padding; R92's hashes cover the WAV file; the old Xbox logo packages'
  page-aligned members; a stream shipped twice identically.
- Process slips: three claims (33, 37, 38) were written at the first checkpoint instead of before the first edit.
- The pair-quota generator diverges from research R123 R2 (kept rows keep their values but are merged by date, as
  Studio's round robin always did) — for the coordinator.
- Release: the helper's LGPL distribution obligations (receipt 32 §6) are identified, not settled.

## 7. Awaiting independent review

Receipts 32–38 in full; research R123 acceptance (and the R2 divergence); the S3 decision (style labels on NHL 11–13).

## 8. October 1 blocker list (ranked by impact)

> **Superseded in part, 27 Sep 2026** ([beta decisions](../../plans/beta-20261001-scope-decisions.md)): rows 3 and 4 are
> **no longer beta blockers** (league and team creation / cloning / deletion are deferred; executable patching is final-release
> backlog), and row 5 is decided (the helper ships inside the one release archive with its source and notices). Row 2's R2
> divergence is corrected in receipt 41. The current list is in the latest consolidated receipt.

| # | Affected operation / targets | Lane | Smallest next action | Kind |
|---|---|---|---|---|
| 1 | Every delivered preview and write (all 27 targets) — nothing user-accepted | user | run the cards on the final bundle: V-72 → V-79, A-SPX-2, A-XMA-2 → A-XMA-4, A-EAL3-2; game: T-3b, P34-1, P1-A-q | user observation |
| 2 | Coordinator acceptance of receipts 28–39 (incl. R123 R2 divergence, S3) | coordinator | review | review |
| 3 | League / team creation, cloning, deletion (all) | user → research | decide in / out of the beta; if in, one runtime experiment on a copy (grown table) | user decision + research |
| 4 | gen7 rules / season settings (X360 / PS3 10 → Legacy) | user | decide whether exe patching is in scope | user decision |
| 5 | Helper distribution (all XMA previews in a release) | user / coordinator | decide the LGPL arrangement (source hosting, about-box / EULA wording, file name) | build / distribution |
| 6 | Grouped schedules in the stock shape / Austrian two-phase (PS4 18–21 and European tables) | coordinator → developer | accept R123 Q9 / Q10 | implementation after acceptance |
| 7 | Goalie gear meaning (all) | research → developer | G2 team mask slots, G3 PS4 (`maskid`, `masktype`) pair validation | implementation |
| 8 | XMA two-segment in-bank records; NHL 11 / 13 / 14 `.ast` XMA shapes; Recompiled audio | research | vectors for those shapes / a Recompiled source route | research |
| 9 | Art-id re-key / archive rename; board ads (gen7); NHL 08–11 conference names | research | contracts / decode / name source | research |

## 9. Cleanup

Scratch patch scripts deleted (scratchpad empty); every test temp root of this run removed by its guard; no process
of this run left running. **Kept on purpose:** `C:\nhlms-toolchain\` (the helper's build dependency; `build\` may be
deleted — the script recreates it), the review bundles under `E:\`, the helper bundle under
`C:\nhlms-toolchain\out\` (source of the bundles' `helpers\`).

## 10. Next development task

**Goalie gear G2 (research handoff 18 §3 / R125b; B11):** show each team's goalie mask slots
(`exhibitionteams.goaliemask<k>_{starter,backup}`, NHL 10 → PS4 21) on the team's uniform surface as numbered paints with the
same file check as `maskid` on the gen7 loose trees, reporting (not refusing) the shipped exceptions (248 / 249 on PS4 16+).
Receipt 40 already fixed the NHL 12 pads position and added the NHL 08 / 09 paint check. This is unless the coordinator
returns a defect or accepts R123 (then the stock-shaped / phased schedule option takes precedence).

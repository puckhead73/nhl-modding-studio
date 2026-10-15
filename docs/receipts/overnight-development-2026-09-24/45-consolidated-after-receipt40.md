# Receipt 45 (consolidated) — dispatch after receipt 40: beta decisions applied, schedule preservation, release archive, goalie mask paints, NHL 08–11 group labels, Hockey Shop NHL 12 pads

**Developer-verified; not Codex-accepted; nothing launched, listened to, game-tested or published.** HEAD
`8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged; nothing staged, committed or pushed). Direction:
[dispatch after receipt 40](../../plans/development-20260927-after-receipt40.md) and the
[beta decisions](../../plans/beta-20261001-scope-decisions.md). Detail: receipts [41](41-schedule-preservation.md),
[42](42-team-goalie-mask-paints.md), [43](43-release-archive-candidate.md), [44](44-group-names-nhl08-11.md), and §3 below.

## 1. Plain-language summary

- **Beta scope applied.** The October 1 blocker list no longer includes:
  - league and team creation, cloning and deletion (deferred);
  - executable patching (final release).

  Ordinary league and team editing and data-file settings stay in scope. The ledger and receipt 39 carry dated
  notes; old receipts are not rewritten.
- **Schedule generation keeps what it doesn't own.** A game involving a club outside the chosen participants
  (the All-Star game, or games against a club left out) now keeps its row and every stored value. New games
  never land on a date where one of their clubs already plays such a game. When that's impossible, Studio
  refuses and names the row. Card V-76's arithmetic is corrected: 264 games at four per pair, and 300 with
  three groups of four playing six games inside each group.
- **A release-candidate archive exists and is verified.**
  - One zip holds Studio, its definitions, the XMA helper with its exact FFmpeg source, licences, notices and
    build record, the third-party notices and a hash manifest.
  - It works from a folder path with spaces, with nothing else installed.
  - Its extraction is the new review bundle.
- **Team goalie mask paints** are on Uniforms → Helmet, with the file check on Xbox 360 / PS3. PS4's shipped
  exceptions are reported, not refused.
- **League labels on NHL 08–11.** Divisions are labelled by their clubs. Conferences on NHL 10 / 11 are
  inferred by position. An NHL 07 display defect (every club in "Northeast") is fixed.
- **Hockey Shop on NHL 12.** The goalie-pads store sync and install now use the game's file naming. The old
  reading found 2 pad ids instead of 44, and an install would have named new files wrongly.

## 2. Packages

| # | Package | Result | In a bundle |
|---|---|---|---|
| 41 | schedule preservation (both generators) + collisions + V-76 | fixed (dev), red → green | r43 |
| 42 | team goalie mask paints (G2) | done (dev) | r43 |
| 43 | release-candidate archive, verification, r43 bundle, page draft / licence audit | done (dev), local only | (is r43) |
| 44 | NHL 08–11 group labels (R133); NHL 07 header division defect | done / fixed (dev) | **after** r43 |
| 45 §3 | Hockey Shop title naming (receipt 40's open limit) | fixed (dev) | **after** r43 |

## 3. Hockey Shop NHL 12 pads (this receipt)

- **Change:**
  - `hockey_shop::sync` takes the title id and uses `for_title`;
  - install plan and apply use the request's new optional `game_id` (`InstallStoreItemRequestDto`, binding
    regenerated);
  - the Hockey Shop page passes the project's title;
  - `studio_api::hockey_shop` is now public so tests can reach it.
- **Evidence** (`logs/r45-hockey-shop-nhl12.log`; `hockey_shop_title_spec` 1 / 1, `--ignored`, real X360 NHL 12 tree,
  temporary store):
  - the title reading finds 44 pad ids, the old reading 2;
  - sync added 41 store items, all of them title ids; 3 have no decodable texture (existing behaviour);
  - a **planned** create install (never applied), from donor 0 to id 4, names both cloned files with the target
    at the second position.
- **Frontend:** vitest 263 / 263, `tsc` exit 0. The first `tsc` run failed: `null` was given where the
  optional field is `undefined`. That was fixed and re-run (`logs/r45-frontend.log`). Bindings `--check` in sync;
  studio-api `bindings` 25 / 25.

## 4. Archive and bundle

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20260927\NHL Modding Studio rc-20260927-r43.zip`: 45 755 047 bytes, SHA-256 `7e81d48b56c300030884064ff6d07d91fdd4b2171a2a72226b154a2b8e618776` |
| Review bundle (current) | `E:\nhlms-review-build-r43-2026-09-27\NHL Modding Studio rc-20260927-r43\nhl-studio.exe`: SHA-256 `0ee1c38c3cb3fdcfbdedd6976ac66b1cca1a27ce26c07645046149b4dd498b80` |
| Inputs | HEAD `8d60a6d` + 861 dirty inputs (`f1465c8a…66c6`), identical after the build; built 12:56:39 → 12:58:48 MDT |
| Contents | receipts **through 42**: 40 (pads / mask paint file checks), 41, 42. **Not included:** 44 and §3 above |
| Helper / source | `dc3533ed…8c21` / `7138d28c…78a3` |
| Earlier bundles | r39, r33 and 26 Sep unchanged |

## 5. Checks executed (all logged under `docs/validation/evidence/overnight-development-2026-09-24/`)

- Schedule (`logs/r41-*`):
  - red run first (3 / 3 failing on the old merge), then 4 / 4 green;
  - `schedule_edit` 14 / 14;
  - the genuine NHL 20 install-database case 1 / 1 (private fixture), which has 0 kept rows and so exercises
    the unchanged path;
  - domain `schedule` unit tests 3; API `schedule_edit` 1.
- Goalie paints (`logs/r42-*`):
  - anchors for X360 07–11 / Legacy and PS4 15 / 21;
  - real X360 trees 10 → Legacy: every non-zero team paint has files;
  - genuine NHL 20: 248 / 249 reported;
  - API bindings 25; vitest 263; `tsc` 0.
- Archive (`review-build-r43/*`):
  - `verify.py check` ok; self-test 13 / 13 refused;
  - extracted-root `defs` test 1 / 1;
  - 83 / 83 R84 vectors through the archive's own helper, run from `E:\tmp` without the override variable.
- Group labels (`logs/r44-*`):
  - `group_names_r133` 2 / 2;
  - domain `leagues` 11, `membership` 5 (two expectations updated, first run's failures logged), `ps4_groups`,
    `view`;
  - API `membership` 6, `leagues` 3.
- Hockey Shop (§3).
- **Not run:** the full workspace suite, repo-wide fmt / clippy (C0.2 baseline), anything desktop, listening or
  in-game. **Genuine NHL 20 versus anchor coverage:** schedule and goalie-paint cases ran on the genuine NHL 20
  fixture as stated. Everything else is anchors or X360 trees; `anchors/ps4/nhl20` is the NHL 19 base and is
  not used as NHL 20.

## 6. October 1 blocker list (corrected; ranked by impact)

| # | Affected | Lane | Smallest next action | Kind |
|---|---|---|---|---|
| 1 | every delivered preview and write — nothing user-accepted beyond NHL 21 E1 | user | run the cards on the r43 bundle (V-72 → V-81, A-SPX-2, A-XMA-2 → 4, A-EAL3-2; game T-3b, P34-1, P1-A-q) | user observation |
| 2 | coordinator review of receipts 28–45 (incl. the receipt 41 preservation rule and the archive) | coordinator | review | review |
| 3 | release publication: Studio licence file (README "to be determined" vs Q46e MIT), About / notices entry, 15 dependencies without a packaged licence file, final distribution review of the helper arrangement | user / coordinator → developer | decide the licence file; then the About entry and notice gaps | release |
| 4 | grouped / stock-shaped / Austrian two-phase schedules (European tables, PS4 18–21) | coordinator → developer | R123's dedicated review (Q9 / Q10) | implementation after acceptance |
| 5 | goalie gear meaning: PS4 (`maskid`, `masktype`) pairs (G3); how team slots combine with `maskid` | research → developer | a G3 contract; runtime card G-MASK-1 | contract / runtime |
| 6 | XMA two-segment records; NHL 11 / 13 / 14 `.ast` shapes; Recompiled audio | research | vectors / a source route | research |
| 7 | art-id re-key / archive rename; gen7 board ads (R127 §6 edge decoded, not yet consumed); NHL 08–11 division names (untraceable without disassembly) | research / developer | re-key contract; a board-art gallery package | research / implementation |

Removed from the list (decided): league and team creation / cloning / deletion (outside the beta), executable
patching (final release), the helper packaging route (in the archive).

## 7. Awaiting independent review and observation

Receipts 41–45 (and 28–40 as before). The preservation rule's conservative date holding and refusals. The archive
layout and notices. The inferred conference labels. Every card listed above.

## 8. Cleanup

- Removed:
  - scratch scripts (the scratchpad is empty);
  - the assembly stage and the check extraction under `E:\nhlms-release-candidate-20260927\`;
  - the self-test scratch;
  - the tests' temporary copies and store (removed by their guards).
- Removed to free disk: the regenerable `target\debug\incremental` cache (≈ 32 GB; E: had 408 KB free). E: now has
  about 20 GB free.
- Older temporary folders under `H:\TEMP` (dated 7–19 Sep) are **not** from this run and were left.
- No process of this run is running. **Kept:** the archive, the r43 / r39 / r33 / 26 Sep bundles, and
  `C:\nhlms-toolchain\`.
- Claims: `dev-schedule-preservation`, `dev-team-goalie-masks-g2` (claim written late, recorded in receipt 42),
  `dev-release-archive`, `dev-group-names-nhl08-11`, `dev-hockey-shop-title-spec`, all `ready`.

## 9. Next task

**Presentation 5.2 — a read-only gen7 arena board-art gallery (research R127 §6).** List each arena venue
package's rasters with the materials that reference them. R127 §6 decodes the data-level material → raster
edge: the u32 at +256 of the `0xEF0001` chunk is the descriptor's chunk index. Label board-ad, power-ring,
jumbotron and glass materials by name, and claim no replacement or runtime selection. This applies unless the
coordinator returns a defect in receipts 41–45 or accepts R123, in which case that takes precedence.

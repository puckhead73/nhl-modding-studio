# Receipt 60 (consolidated) — dispatch after receipt 45: release notices, Hockey Shop writes, grouped schedules, venue art, audio, PS4 gear, reassignment, Recompiled

**Developer-verified; not Codex-accepted; nothing launched, listened to, game-tested, staged, committed or published.** HEAD
`8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged). Direction: [dispatch after receipt 45](../../plans/development-20260928-after-receipt45.md)
and the [coordinator review](../../validation/development-20260928-receipt45-review.md). Run: 28 Sep 2026, 04:54 → about 08:10 MDT.

## 1. Plain-language summary

- The r59 review build adds several things users can do:
  - generate a league schedule from groups;
  - install Hockey Shop items on NHL 12 knowing the write does only what was reviewed;
  - browse arena board art with pictures (Xbox 360 and now PS3);
  - hear far more Xbox 360 sounds, including on Legacy Recompiled;
  - see PS4 goalie mask pairs and skater gear with the game's own names and pictures;
  - reassign portraits and heads on PS3 NHL 09 → 14 and Legacy Recompiled.
- Three wrong behaviours were found and fixed:
  - Hockey Shop on NHL 12 could clone shared pad meshes under a new pad number.
  - Valid Xbox sounds were refused.
  - **Legacy Recompiled projects could not edit weight, ratings, birth dates or contract dollars.**
- The app now has an About & Licences page. The archive's licence gates stop a candidate calling itself release-ready. **It still
  cannot be released:** the MIT copyright holder's name and the rights to 53 artwork files are unrecorded.
- None of this is user-accepted yet. The cards below are how it closes.

## 2. Packages (each links its receipt)

| # | Package | Result | Bundle |
|---|---|---|---|
| [46](46-release-notices-mit.md) | MIT metadata; About & Licences; 15 missing licence texts resolved (4 not shipped, 11 fetched from exact sources); linked-helper components; release gates; WebView2 / prerequisite wording | done (dev); holder name and artwork rights missing | r48+ |
| [47](47-hockey-shop-write-regression.md) | Hockey Shop NHL 12 applied create / overwrite on a copy; project title authoritative; reviewed fingerprint; rollback | **defect fixed** (meshes read as pads) | r48+ |
| [48](48-schedule-grouped-single-phase.md) + [obs 48](observations/48-R123-validator-gaps-and-phase-decision.md) | R123 single-phase grouped generation with the coordinator's corrections | done (dev); Austrian phased withheld | r48+ |
| [49](49-venue-board-art-gallery.md) + [obs 49](observations/49-R127f-slot1-past-block.md) | read-only gen7 venue-art gallery | done (dev) | r53+ |
| [50](50-xma-inbank-residuals.md) | X360 in-bank XMA residual shapes | **defect fixed** (valid `0x7FFF` packets refused) | r53+ |
| [51](51-ps4-goalie-mask-pairs-g3.md) | PS4 goalie mask pairs (G3) + a Rust Attrib/VLT vault reader | done (dev) | r53+ |
| [52](52-ps4-skater-gear-catalogue.md) | PS4 skater gear vs Creation Zone lists, pictures | done (dev) | r53+ |
| [53](53-ast-xma-streams-11-13-14.md) | X360 NHL 11 / 13 / 14 `.ast` streams | done (dev) | r53+ |
| [54](54-ps4-goalie-mask-thumbnails.md) | PS4 mask pictures | done (dev) | r56+ |
| [55](55-archive-held-reassign.md) | portrait / head reassignment on PS3 NHL 09 → 14 and Recompiled via archive members | done (dev) | r56+ |
| [56](56-ast-xma-streams-10-15-legacy.md) | X360 NHL 10 / 15 / Legacy stream shapes | done (dev) | r56+ |
| [57](57-ps4-creation-zone-names.md) | PS4 gear / mask names through the title's own dictionary | done (dev) | r59 |
| [58](58-venue-art-ps3-previews.md) | PS3 venue pictures; X360 NHL 11 / 12 / 14 and PS3 NHL 14 in the gallery | done (dev) | r59 |
| [59](59-recompiled-id-and-audio.md) | Recompiled title-id mismatch; Recompiled sound banks from its own archives | **defect fixed** (Recompiled player edits refused) | r59 |

**Research handoff 07 (NHL 07 / 08 generic heads): no change needed.** Studio lists NHL 07 / 08 containment as "not measured", never
"missing", and still refuses reassignment there, as the handoff asks.

## 3. Corrections completed (dispatch items)

- **§1 release:**
  - MIT is declared in every manifest. `LICENSE` is withheld, never a placeholder.
  - The About page ships.
  - 0 missing third-party texts.
  - Assembly marks the candidate not release-ready and names its blockers. `verify.py` refuses 19 broken copies plus release mode.
  - The "everything included" wording is corrected (WebView2 prerequisites).
  - Receipts 44 / 45 are in r48 and later.
- **§2 Hockey Shop:** applied write regression on a disposable copy; title-context audit; the mesh-as-pad defect is fixed.
- **§3 R123:**
  - stored indices are preserved;
  - generated / retained collisions are rejected;
  - invalid odd-host rules and incomplete per-pair lists are refused;
  - the date-sorted comment is corrected;
  - the Austrian writer stays withheld;
  - validator gaps are recorded (observation 48).
- **§4 board art:** read-only gallery (49), with PS3 pictures and four more titles (58). No writer.

## 4. Evidence, kept separate

- **Source inspection:** the research handoffs of 26–28 Sep consumed per receipt. The Recompiled title-id mismatch was found by
  reading, then reproduced red (59).
- **Automated results** (all `--include-ignored` where fixtures exist; exact counts in each receipt):
  - `studio-domain --lib` 226 passed / 2 ignored · `studio-api --lib` 34 / 34
  - vitest 276 / 276, 46 files · `tsc` clean · bindings `--check` in sync
  - Package suites:
    - player_reassign 4 / 4 · ps4_skater_gear 2 / 2 · ps4_goalie_mask_pairs 2 / 2 · API ps4_goalie_mask_pair 2 / 2
    - venue_art 2 / 2 · venue_r127f 1 / 1 · API arena_assets 5 / 5
    - gen7_sfx_banks 7 / 7 · gen7_xma_ram_residuals 5 / 5 · encoding_contract 10 / 10 · API gen7_asset_member_route 5 / 5 ·
      team_logos 4 / 4
  - Genuine NHL 20 (CUSA15439, `E:\nhl-studio-fixtures`) is in 48, 51, 52, 54 and 57. `anchors/ps4/nhl20` was treated as NHL 19
    base.
- **Package verification:** r59 `verify.py check` passes, and `selftest` refuses all 20 cases. The three bundle tests pass from
  `E:\tmp` against the extracted bundle: defs beside the exe, every notice file present, and the R84 XMA vector test through the bundled helper (`dc3533ed…`).
- **Not run:**
  - a full `cargo test --all`;
  - `cargo fmt` / `clippy` (known C0.2 baseline failures);
  - Windows CI;
  - any app launch, desktop observation, listening or game run.
- **First failures, kept:**
  - Three suites once failed only because `NHL_STUDIO_FIXTURES` was unset (59).
  - Old assertions intentionally superseded, each named in its receipt: 53 → 56 (Legacy loop refusal), 30 → 59 (Recompiled
    refusal), 49 → 58 (PS3 sizes only).

## 5. Archive and review bundle

| Item | Value |
|---|---|
| **Current bundle** | `E:\nhlms-review-build-r59-2026-09-28\NHL Modding Studio rc-20260928-r59\nhl-studio.exe`, SHA-256 `940a24c097cd8d49d76ae3335ae63d6cc083227ff6482512574b410bdaccf4d6` |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r59.zip`, SHA-256 `aaf1e4fbce2115e4a7b01aa25c87e263980d9bdf8a6f5e2447ef2c9ec4173401`, 620 files |
| Inputs | HEAD `8d60a6d` + 949 dirty inputs (`f90e166b…17b4`), identical after the build; built 07:50:14 → 07:52:40 MDT |
| defs / frontend / helper / FFmpeg source | `20c3b183…cf8b` / `6d5ce1f9…9845` / `dc3533ed…8c21` / `7138d28c…78a3` |
| Contents | receipts **through 59** |
| Kept | r56, r53, r48 (and r43 and earlier) bundles and zips, unchanged; evidence under `docs/validation/evidence/overnight-development-2026-09-24/review-build-r{48,53,56,59}/` |
| Status | internal review candidate; **not release-ready** by its own record |

## 6. Release / licence items still missing

1. **The MIT copyright holder's name.** Owner: user. Fill `tools/release-archive/studio-licence.json` from a recorded answer. It is
   never invented.
2. **Rights to 53 artwork files embedded in the executable** (17 box-art tiles, 3 platform tiles, 30 strategy GIFs, logo, backdrop,
   icon). Owner: user / coordinator. Record the rights, or replace the files.
3. **Final distribution review** of the helper arrangement and the complete notices. Owner: coordinator. No legal verdict is claimed.

## 7. October 1 blockers (ranked by impact)

| # | Affected | Owner | Smallest next action | Kind |
|---|---|---|---|---|
| 1 | Every delivered preview and write; nothing user-accepted beyond NHL 21 E1 | user | Run the r59 cards: V-82 → V-93, A-XMA-5 → 8, and the earlier V / A cards (MORNING-REVIEW); game cards P1-A-q, T-3b, P34-1, R36-PS3-KIT-1 | observation |
| 2 | Coordinator review of receipts 46–60, especially the release gates (46), Hockey Shop write contract (47), grouped schedule safety (48) and the Recompiled fix (59) | coordinator | Review | review |
| 3 | Release publication | user → developer | Holder name and artwork rights (§6) | decision |
| 4 | Austrian phased schedules (European tables) | user → coordinator | Card P1-A on a copied NHL 21 save; then a phase decision | observation / decision |
| 5 | PS4 head reassignment (B10); PS4 heads are vault-keyed | developer | The next task below | implementation |
| 6 | Goalie mask selection (slot vs personal mask), mask display labels | user / research | Card G-MASK-1 | observation |
| 7 | Board-ad replacement (B13); archive re-key | coordinator → developer | A dispatch that lifts the withholding for the R137 §5 same-size contract | withheld |
| 8 | Recompiled beyond reads: loader precedence, saves, roster ceilings | user / research | A Recompiled runtime card | observation |
| 9 | NHL 07 / 08 generic heads | user | Card P34-G08 (handoff 07) | observation |

## 8. User decisions and observations needed

- The copyright holder's name for `LICENSE`.
- The artwork rights decision.
- The r59 cards (above).
- P1-A (Austrian phases), G-MASK-1, P34-G08 (game cards on copies).

## 9. Cleanup and retained dependencies

- **Removed:**
  - every scratch script of this session (the scratchpad is empty);
  - the verifier self-test scratch (removed by itself; confirmed absent).
  - The tests' temporary copies are removed by their guards.
- **Left, not this run's:**
  - `H:\TEMP\nhlms-league-campaign-all-97012`, `nhlms-logos-gen7-106396`, `nhlms-p3-op-99420` (18–19 Sep);
  - `nhlms-logos-gen7-102420`, `nhlms-logos-gen7-106856`, `nhlms-p3-op-24084`. These are dated 28 Sep 04:48, before this run
    began at 04:54, so this run cannot claim them.
- **Kept:**
  - review bundles r33 → r59 (63–68 MB each) and `E:\nhlms-release-candidate-20260928\` (r48 / r53 / r56 / r59 zips, 243 MB);
  - `C:\nhlms-toolchain\`, `E:\nhl-studio-fixtures`, the shared `target\` cache (not deleted).
  - E: has about 12 GB free.
- **Not touched:** `zz_diag.rs` (excluded), research files, coordinator records, CLAUDE.md, capability CSVs, QA generators.
- **Claims:** all `ready`: `dev-release-notices-mit`, `dev-hockey-shop-write-regression`, `dev-schedule-grouped-single-phase`,
  `dev-venue-board-art-gallery`, `dev-xma-inbank-residuals` (receipts 50 / 53 / 56), `dev-ps4-goalie-mask-pairs-g3` (51 / 54),
  `dev-ps4-skater-gear-catalogue`, `dev-archive-held-reassign`, `dev-ps4-cz-names`, `dev-venue-ps3-previews`,
  `dev-recompiled-id-and-audio`.
- No process of this run is running.

## 10. Next task

**PS4 head reassignment (Players 3.4, B10) per R124 row 78 with R15.**
- **Check:** a target head id must exist as a vault `ge_nhl_skaterhead` `SkaterNameAttribute` value in the open title's own
  identity-checked vault (receipt 51's reader).
- **Write:** on PS4 17–21, write `headtexture` with `headid` as one change, with undo.
- **Disclose:** when the target is another player's bespoke head.
- **Refuse:** an id with no vault record (e.g. NHL 15's 10391).
- **Test:** genuine NHL 20 and one other PS4 title, on disposable copies.
- **Precedence:** a coordinator-returned defect in receipts 46–59 comes first.

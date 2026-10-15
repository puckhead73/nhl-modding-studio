# Receipt 65 (consolidated) — dispatch after receipt 60: recovery correction, Recompiled regression, PS4 heads, release inputs

**Developer-verified; not Codex-accepted; nothing launched, listened to, game-tested, staged, committed, uploaded or sent.** HEAD
`8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged). Direction: [dispatch after receipt 60](../../plans/development-20260928-after-receipt60.md),
[receipt-60 review](../../validation/development-20260928-receipt60-review.md).

## 1. What users can do now (r64)

- **PS4 players can be given another head the game ships** (NHL 15–21, including genuine NHL 20).
  - Studio checks the head against the open game's own head catalogue.
  - On NHL 17–21 it changes both stored head numbers together, as one undoable edit.
  - The check tells you whether you are choosing a shared generic head or someone else's face.
- **A reassignment only applies what you checked**, on every title. If the row changed in between, nothing is written.
- **A failed Hockey Shop install can no longer destroy an original file.** If undoing it fails, the original stays on disk and the
  error says where.
- **Legacy Recompiled player edits are proven through the app's own path.** Weight, a rating, the birth day and the contract are
  covered, with review, undo, redo, save, reopen and import.
- The build and its notices say plainly that this is an **internal review build**. The artwork stays in by your decision, and each
  image group is listed for replacement before any external release.

## 2. Packages

| # | Package | Result | Bundle |
|---|---|---|---|
| [61](61-hockey-shop-rollback-f1.md) | F1: Hockey Shop rollback keeps and names recovery copies; `create_new` ownership; unowned files never touched | **defect fixed** | r64 |
| [62](62-recompiled-public-edits.md) | Recompiled `legacyrecomp` edits through discovery → extraction on a copy → open → edit / review / undo / redo / save / reopen / import; wrong title refused | regression added (test only) | code in r59+ |
| [63](63-ps4-head-reassign.md) | PS4 head reassignment against the open title's vault; paired `headid` + `headtexture` on 17–21; reviewed-plan fingerprint on every reassignment | done (dev) | r64 |
| [64](64-internal-bundling-and-r64.md) | internal-bundling decision recorded (rights unchanged), per-group backlog, `distribution: internal-review` gate; NHL 11–13 companion-style lock; r64 | done (dev) | is r64 |

## 3. F1 recovery evidence (receipt 61)

- Seven transaction fault tests pass, among 11 `hockey_shop` unit tests in all:
  - failed restore after an overwrite with a backup: the `.bak` survives with the original and is named. The reviewed code deleted
    it; the coordinator's harness showed `backup_survives=false`;
  - failed restore without a backup: `.nhlms-original` is written and named;
  - a pre-existing backup survives;
  - an ordinary rollback retires only the new backup;
  - create mode: an unchanged clone is removed, a changed one is kept and reported;
  - an unowned `.nhlms-tmp` is refused before anything is touched;
  - a failed rename (a read-only destination) leaves no temporary file.
- The real NHL 12 applied create / overwrite regression passes 2 / 2.
- This is best-effort recovery in memory. It is **not** crash-safe and does not protect against a concurrent writer.

## 4. Recompiled / public path and PS4 heads

- **Recompiled (62):**
  - Discovery on the registered install and on its `game` folder: no loose database, extraction needed.
  - `extract_install` runs on a scratch copy of `cacheboot.big` (61 files). The database's SHA-256 is `105d2186…`, the one R128
    recorded; the archive carries its own metadata.
  - The four encoded cells go through display, review (at least 4 pending cells), 4 undo, 4 redo, save → reopen and the import
    (+1 player, one undo).
  - `nhl09` on Recompiled refuses both the edit and the import with nothing pending. The alias still works.
  - Nothing is inferred about roster ceilings, sidecars or precedence.
- **PS4 heads (63):**
  - genuine NHL 20 and NHL 21 on copies: bespoke target (the duplicate face is disclosed), pool 60000, absent 60211;
  - fingerprint refusal on a changed row; exceptional-row warning; paired write; one undo / redo; save / reopen;
  - missing `headtexture`: refused, nothing written;
  - foreign vault (NHL 19 project, NHL 20 install): "source mismatch";
  - NHL 15: `headid` only, 10391 refused.

## 5. Tests (all `CARGO_INCREMENTAL=0`; `NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures` where needed)

| Suite | Result |
|---|---|
| `studio-api --lib` (incl. 11 `hockey_shop`) | 40 / 40 |
| `studio-api` `hockey_shop_title_spec` (`--include-ignored`) | 2 / 2 |
| `studio-api` `recompiled_public_edits` (new) | 1 / 1 (30.5 s) |
| `studio-api` `ps4_head_reassign` (new) | 1 / 1 (223 s; a first version took 633 s and was trimmed) |
| `studio-domain` `player_reassign` (`--include-ignored`) | 4 / 4 |
| `studio-domain` `style_label_scoping` | 4 / 4 (new NHL 11–13 lock) |
| frontend vitest / `tsc` / bindings | 276 / 276 · clean · in sync (`PlayerReassignPlanDto` regenerated) |
| r64 `verify.py check` / `selftest` / bundle tests from `E:\tmp` | pass · 21 / 21 refused · 3 / 3 |

**First failures, kept:**
- `recompiled_public_edits` and `ps4_head_reassign` first failed to compile: two DTOs have no `Debug`. They were fixed with match
  helpers.
- The first NHL 11–13 lock probe read the roster table's `QgFs`; the companion there is `p_style` (`mQoE`), and it was corrected.
- One shell command failed to parse before running; it was redone with files.

**Not run:** a full `cargo test --all`, fmt / clippy (C0.2 baseline), Windows CI, any app launch, listening or game run.

## 6. Bundle

`E:\nhlms-review-build-r64-2026-09-28\NHL Modding Studio rc-20260928-r64\nhl-studio.exe` (`b7ab3ce5…77ba`), extracted from
`E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r64.zip` (`6f88b5f7…56f5`).
- It holds receipts through 64; 952 dirty inputs, identical after the build.
- `distribution: internal-review`, `release_ready: false`.
- r59, r56, r53, r48 and earlier are kept unchanged.
- New cards: V-94 (PS4 heads), V-95 (stale apply).

## 7. Remaining beta scope and blockers (ranked; nothing here is excluded by silence)

| # | Item | Owner | Smallest next action | Kind |
|---|---|---|---|---|
| 1 | User observation of every delivered preview and write (r64 cards V-82 → V-95, A-XMA-5 → 8; game cards P1-A-q, T-3b, P34-1 / R15-HEAD-1, R36-PS3-KIT-1) | user | run the cards on r64 | observation |
| 2 | Coordinator review of receipts 61–65 (F1 fix, PS4 head write contract, internal-bundling gate) and of earlier unreviewed parts of 46–59 | coordinator | review | review |
| 3 | **Austrian phased schedule generation** | coordinator (contract) ← user card P1-A | phase-semantics card on a copied NHL 21 save, then a contract decision | withheld (in scope) |
| 4 | **Board-art replacement** (R137 §5 same-size writer; loose + archive pairs; PS3 11–13 archive-only) | coordinator → developer | a dispatch lifting the withholding with the review's corrections (paired sources; tested mutation per byte range) | withheld (in scope) |
| 5 | **Archive re-key** (art-id rename inside archives) | coordinator | contract decision | withheld / parked (in scope) |
| 6 | Goalie mask selection at runtime (slot vs personal), G-MASK-1 | user | card | observation |
| 7 | PS4 hair (`hairid` via `GenericHairStyle`, NHL 17–21), not yet offered | developer | next task below | implementation |
| 8 | Recompiled beyond reads: loader precedence, saves, roster ceilings | user / research | runtime card | observation |
| 9 | NHL 07 / 08 generic heads (P34-G08) | user | card | observation |

Outside beta (unchanged): league / team creation / cloning / deletion and executable patching. Projects / Builds are deferred.

**Release inputs:**
- **MIT copyright holder: no recorded answer.** `LICENSE` is withheld and the single missing value stays in
  `tools/release-archive/studio-licence.json`.
- **Artwork:** internal bundling is permitted (user, 28 Sep). The 53 files keep `rights: unrecorded`, and external release is
  refused until the backlog in `docs/release/artwork-replacement-backlog.md` is resolved.
- **Coordinator:** the final distribution review.

## 8. Cleanup

- **Removed:** this session's scratch scripts (the scratchpad is empty); the self-test scratch (removed by itself). The tests' copies
  (`nhlms-recomp-edits-*`, `nhlms-ps4-heads-*`, `nhlms shop txn *`) were removed by their guards and are confirmed absent.
- **Left, not this run's:**
  - `H:\TEMP\nhlms-league-campaign-all-97012`, `nhlms-logos-gen7-{102420,106396,106856}`, `nhlms-p3-op-{24084,99420}`;
  - `H:\TEMP\nhlms_ps3_dds_portrait` (created 7 Sep). Its two files were rewritten today by an existing `studio-domain` lib test in
    `assets.rs` that reuses the folder. That behaviour is recorded, not changed.
- **Kept:** bundles r33 → r64 and the r48 / r53 / r56 / r59 / r64 zips; `C:\nhlms-toolchain\`; the fixtures; the shared `target\`
  cache. E: has 11 GB free.
- **Claims `ready`:** `dev-hockey-shop-rollback-f1`, `dev-recompiled-public-edits`, `dev-ps4-head-reassign`,
  `dev-artwork-internal-bundling`. No process of this run is running.

## 9. Next task

**PS4 hair reassignment (NHL 17–21).**
- **Check:** a target `hairid` must be the value of exactly one `GenericHairStyle` record in the open title's identity-checked vault
  (R15 §1: 54–59 database hair ids resolve on 17–21; the class is absent on 15 / 16).
- **Reuse:** receipt 63's catalogue, fingerprinted plan and apply.
- **Test:** genuine NHL 20 and NHL 21 on copies.
- **Precedence:** a coordinator-returned defect in receipts 61–64 comes first.

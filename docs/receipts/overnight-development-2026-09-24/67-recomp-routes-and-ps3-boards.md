# Receipt 67 (consolidated) — dispatch after receipt 66: Recompiled correctness and read routes, PS3 board replacement (C37), r67

**Developer-verified; not Codex-accepted; nothing launched, listened to, game-tested, staged, committed, uploaded or sent.** HEAD
`8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged). Direction: [dispatch after receipt 66](../../plans/development-20260928-after-receipt66.md),
[research-7 developer supplement](../../plans/development-20260928-research7-addendum.md), [receipt-66 review](../../validation/development-20260928-receipt66-review.md).
Claim `dev-recomp-routes-and-ps3-boards`.

## 1. Plain-language summary

- **Hockey Shop no longer pretends to install into Legacy Recompiled.** Recompiled keeps its gear inside archives, and nobody has
  shown that it reads loose files. Sync, install planning and install apply are now refused for Recompiled.
  - The refusal says why, and nothing is read into the store or written into the game folder.
  - This is a temporary guard. Recompiled equipment editing stays in beta scope.
- **The portrait / head card now checks Recompiled's archives** instead of claiming to.
  - A player's file is marked present when an archive holds it.
  - It is marked missing only when every archive could be read.
  - This install has two small archives Studio cannot list, so missing ids honestly show "unknown".
- **Recompiled play-by-play works:** the speech list, contexts, team and player name calls, and sample previews. They read the
  archive members and give the same results as the Xbox 360 Legacy files.
- **A dictionary that fails to load at open is now shown** on the Dashboard, instead of names silently appearing as keys.
- **PS3 board ads can be replaced** (NHL 11–15 and Legacy):
  - the image must be the board's own size; DXT1 or DXT3; every smaller copy (mip level) is regenerated;
  - on NHL 14 / 15 / Legacy the loose package and its archive copy are written together, as one change with full undo;
  - on NHL 11–13 only the archive copy exists, and no loose file is invented;
  - it goes through the normal pending-changes review and the copied-target save;
  - the Venue art card shows **Replace…** on PS3 board materials.
  - **One behaviour for you to review:** DXT1 boards are written opaque. A transparent area in your image would otherwise become
    black holes.
- **New internal-review bundle r67.** r66 and earlier are kept.

## 2. Recompiled Hockey Shop guard (supplement §1)

- `hockey_shop::refuse_recompiled` refuses by title (`legacyrecomp`) inside `sync`, `plan_install` and `apply_install`.
- `Registry::recompiled_context` also refuses by platform: the open project, or the attached roots declared as `recomp`. This
  covers a request that names `nhllegacy` against a Recompiled project or root.
- The message names the missing archive writer and the unproven loose loader (research handoff 15, card R-RECOMP-LOOSE-1).
- **Test** `recompiled_hockey_shop_guard` (1 / 1) uses an owned tree holding one real X360 Legacy glove (id 15, 10 files).
  - Refused, each: sync; plan; apply with no fingerprint, a forged one, and the genuine X360 plan's fingerprint; project-less by
    title; project-less by platform; the domain apply.
  - The tree and the store stay byte-identical.
  - The control: NHL Legacy (X360) on the same tree plans an allowed overwrite, then installs with its `.bak`.
- Also run: `hockey_shop_title_spec` (NHL 12, real tree) 2 / 2, and the 13 `hockey_shop` unit tests.

## 3. Containment through the archive index (supplement §2)

- `player_containment_with_archives`: loose first. A gen7 directory that is absent falls back to the install's archive members
  (`fe/ion/artassets/playerheads/<bucket>/p<id>.big`; `rendering/player/<prefix><id>.<ext>`).
  - A complete index decides present or missing.
  - Unreadable archives make ids without a member **unknown**, and "unreferenced" counts are withheld.
  - No index, or no archive, keeps the old unknown.
- The Registry builds the index for archive-held titles (PS3 NHL 09–14, Recompiled), outside the workspace lock.
- The contract text now says the check happens when the install is attached.
- **Unit test** (5 / 5 in `player_containment`): present, missing, unknown, and loose-first.
- **Public path, real install:** Recompiled attached both as the install folder and as its `game` folder, compared with the X360
  Legacy loose tree.
  - 246 bespoke heads and 246 bespoke portraits are present, and 5 385 pool heads.
  - Shipped portraits and heads are 9 224 and 904, equal to the X360 tree.
  - 1 129 portraits missing on X360 show **unknown**, because `gamedata/gps.big` (a BIG4 with no `.bh`) and
    `gamedata/anim/faceposer.big` (an EB variant the lister does not read) cannot be listed.
  - A folder with no archives shows everything unknown.

## 4. Recompiled speech route (handoff 16) and remaining read routes (handoff 12)

- `speech_catalogue::AudioDir` is loose first. On `legacyrecomp` + `Platform::Recomp` only, it falls back to archive members.
  - Search order: `audioboot.big`, `nocache.big`, then every other archive, under `game/` or the root.
  - A path found in two archives is refused unless the bytes are equal.
  - Missing required members name the archives searched; `keywords` stays optional.
  - Provenance is `archive:member (form, stored bytes)`.
  - It is used by the catalogue, team / player name calls and contexts.
- `gen7_speech_preview::read_member_unwrapped` is the shared unwrap for stored / chunkzip / chunklzx / refpack. A corrupt wrapper is
  refused, never read as stored. `archive_beside` adds the `game/` prefix for bank and stream archives.
- **Public path** (`recompiled_speech_routes_read_the_archive_members`, both roots):
  - the unwrapped source hashes equal the X360 Legacy loose route;
  - catalogue counts equal: 3 982 events, 1 718 sentences, 59 940 bank samples, and 4 183 contexts;
  - the name calls equal: team 1 has 29 samples, player 2 has 2 announcer samples;
  - pbp sample 58 decodes to identical PCM;
  - negative cases: `nhl09` on Recompiled is refused; an empty root is refused; a disposable `audioboot.big` copy with a corrupted
    chunklzx body is refused by both catalogue and preview.
- **Dictionary disclosure:** `Workspace::attach_dictionary_at_open` records the failure, shown as `ProjectSummaryDto.dictionary_problem`
  on the Dashboard. Test 1 / 1.
- **Title admission** is per operation (speech, containment, Hockey Shop). There is no global alias. The member route is admitted
  by title and platform. The archives' whole-file identities (R128) are **not** hashed per read, for cost reasons. The member
  hashes are reported instead.

**Remaining Recompiled read coverage** (research handoff 12 §2):

| Route | Status |
|---|---|
| Database, dictionary, sound banks, portrait / head reassignment, logo table | working (earlier receipts) |
| Containment card | this receipt |
| Speech catalogue, contexts, name calls, speech preview | this receipt |
| Portrait / head preview, texture preview | **not built** — needs a member route in the player-asset scan (handoff 17 contract) |
| Team header logo | not built (loose only) |
| League imagery | not built (no `legacyrecomp` row; alias decision open) |
| Kits / uniforms, arena / venue art, equipment previews | not built (`Unmeasured` / loose only) |
| Hockey Shop | refused (§2) |
| Roster save runtime compatibility / limits | user card |

## 5. PS3 board-ad replacement, C37 (supplement §3–5; handoff 10 §5 / §8)

**Admission, `tdb_rx2::venue_board`.**
- `admit` implements §8.1 in order: predecessor block; slot word; descriptor inside the header region; equal format words;
  DXT1 / DXT3 only; positive and bounded dimensions; level-count bound; entry word; exact declared chain; pairwise overlap
  (touching and empty spans allowed); single owner.
- `discover` reports the format and admits nothing.
- **The 29 research vectors** were rebuilt byte for byte in Rust from the research builder, and all pass: 29 / 29, with the
  recorded reasons.
- **Census parity** (`ps3_board_census`, read only, release) reproduces research R142b exactly:

| Title | Packages | Admitted edges | Other checks |
|---|---|---|---|
| NHL 11 | 51 | 136 (all DXT1) | the only refusal is `adbd_41` (slot word) |
| NHL 12 | 57 | 226 | |
| NHL 13 | 61 | 232 | |
| NHL 14 / 15 / Legacy | 64 each | 243 each | every loose package equals its member |

  - No DXT5 board anywhere.
  - Research counted 67 loose packages on the pair titles; the archives list 64. A loose package without a member is refused by the
    planner, because a pair needs both copies.

**Planner, `studio_domain::board_replace`.**
- Topology by title:

| Titles | Topology | Archive |
|---|---|---|
| NHL 14 / 15 / Legacy | pair | `cacherender.big` |
| NHL 12 / 13 | archive-only | `cacherender.big` |
| NHL 11 | archive-only | `stadium.big` |

  X360 / Recompiled are refused as withheld; PS3 NHL 10 as unmeasured; PS4 as unsupported.
- The member must be exactly one entry with no shared span. chunklzx is refused. A diverged pair is refused, and so is a loose file
  on an archive-only title.
- Every declared level is regenerated from the image with a 2 × 2 box filter and encoded with `compress_bc`. No bytes outside the
  entry may change (asserted).
- The member is re-wrapped as native chunkzip and decode-checked.
- The review discloses every material using the raster, the source identities, the topology and the runtime caveat.
- **DXT1 is encoded opaque** (a decision to review): texpresso's BC1 turns translucent source pixels into transparent black. When the
  image has alpha, the plan discloses that it is dropped.

**Transaction.**
- `AssetMemberPlan` gains `paired_loose` and `loose_must_be_absent`; the copied-target group carries both.
- A new `txn::loose_file_artifact` is expected **present** at exactly the reviewed identity, and joins the archive's change set,
  depending on it.
- `CopiedTargetPlan.absent_paths` is re-checked in `verify_plan_inputs`, both at staging and before apply.

**API and UI.**
- `Registry::ps3_board_replacement_preview` and the Tauri command `ps3_board_replacement_preview`. The command was registered only
  after the transaction tests below passed. Apply is the existing `asset_member_apply_review`.
- `VenueRasterDto.descriptor`.
- `BoardReplace.tsx` on the Venue art card: PS3 board materials with DXT1 / DXT3 rasters. X360 stays read-only.
- UI test 4 / 4.
- Archive-only titles have no gallery (their packages are not loose), so on NHL 11–13 the route is API-only for now.

**Transaction tests** (`ps3_board_replacement`, release, 755 s, owned copied-target roots on H:, sources only read).
- **NHL Legacy pair:** `stadium_321_container_0` descriptor 124 (DXT1 256 × 512, 10 levels, 3 materials) and `stadium_13_container_0`
  descriptor 30 (DXT3 512 × 128, `adbd_staples`) are reviewed into one change set: `primary`, `archive-asset-0`, `loose-asset-0-0`
  and `loose-asset-0-1`.
- **NHL 13 archive-only:** the same two rasters, with change set `primary` and `archive-asset-0`.
- **Refused at preview:** a wrong size; a non-board raster; a diverged pair, or a loose file on NHL 13.
- **Faults:**
  - a failure after the archive persisted, and (pair) after the first loose file persisted: both roll back to the original hashes;
  - a failure after the last output **plus** a failure during the archive's revert: never committed, and `copied_target_recover`
    rolls back to the originals.
- **Stale sources:** each refuses with nothing written.
  - a metadata-preserving flip in the loose file (pair);
  - a loose file appearing after review (archive-only);
  - a metadata-preserving flip inside the member's stored bytes.
- **Apply committed:**
  - every declared level of both rasters equals `compress_bc`'s output for that level at its offset;
  - this test file's own spec DXT1 / DXT3 decoder, independent of texpresso, is within tolerance: per-level mean error DXT1
    1.5 → 22.6 and DXT3 1.6 → 10.1. The bound follows block coverage: mean ≤ 10 at ≥ 16 texels per axis, ≤ 24 below;
  - bytes outside the rasters are unchanged, and every edge resolves as before;
  - loose equals the unwrapped member (pair);
  - **12 844 (Legacy) / 10 967 (NHL 13) other members keep their stored bytes.**
- **Reopen and restore:** a fresh registry's plan reads the new bytes. Restore returns the archive to its original hash (Legacy
  `6df2b3eb…fa0f` → `d7e430c4…04d1` → restored; NHL 13 `9391e67f…c14b` → `a402302a…e7f1` → restored) and both loose files to their
  originals.
- **Not covered:**
  - a foreign file at the engine's own backup or temporary names. That is the engine's existing tests
    (`missing_or_corrupt_backups_block_rollback_and_restore_without_overwriting`, `tests_r2`), not re-run here;
  - crash (process-exit) interruption of this route specifically;
  - NHL 11 / 12 / 14 / 15 writes (admission census only);
  - which copy the console loads, and the runtime display.

**First failures, kept:**
- The copied root must be the install folder (`USRDIR`); the first fixture put it one level up.
- The debug build hashed gigabytes too slowly, so the test was stopped and moved to release; its owned scratch was removed after a
  containment check.
- The DXT1 translucent-pixel finding (above).
- The small-level tolerance was first set to the large-level bound.

## 6. Display caches (dispatch §3)

- The PS4 goalie-mask and skater-gear display caches now include the identity-definitions file's SHA-256 and the defs root in their
  key, next to the title and the install.
- `boot.big` is still keyed by length and time: a recorded display limitation. No write consumes those caches.

## 7. Tests (all `CARGO_INCREMENTAL=0`; `NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures` where needed)

| Suite | Result |
|---|---|
| `tdb-rx2 --lib venue` (29 vectors, writer span, box levels) | 6 / 6 |
| `studio-api` `ps3_board_replacement` (release; two real topologies + refusal-by-name) | 3 / 3 |
| `studio-api` `ps3_board_census` (release) | 1 / 1 |
| `studio-api` `recompiled_hockey_shop_guard` · `recompiled_read_routes` | 1 / 1 · 2 / 2 |
| `studio-api` `hockey_shop_title_spec` (`--ignored`) · `asset_member_route` · `copied_target` | 2 / 2 · 5 / 5 (2 ignored) · 15 / 15 |
| `studio-api --lib` (export root count now 173) | 42 / 42 |
| `studio-domain` `player_containment` · `speech` / `archive_index` lib · `dictionary_attach_disclosure` | 5 / 5 · 7 / 7 · 1 / 1 |
| frontend vitest (all) · `tsc` · bindings `--check` | 278 / 278 · clean · in sync |
| r67 `verify.py check` · `selftest` · bundle tests from `E:\tmp` | ok · 21 / 21 refused · 3 / 3 |

**Not run:**
- full `cargo test --all`, and fmt / clippy (C0.2 baseline);
- Windows CI;
- `studio-domain player_reassign` real-tree cases after this receipt's changes (receipt 66 ran them; `plan_reassign` is unchanged);
- any launch, listening or game run.

## 8. Bundle r67

| Item | Value |
|---|---|
| Bundle | `E:\nhlms-review-build-r67-2026-09-28\NHL Modding Studio rc-20260928-r67\nhl-studio.exe`, SHA-256 `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65` (50 876 928 bytes) |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r67.zip`, SHA-256 `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a`, 620 files, 46 132 357 bytes |
| Inputs | HEAD `8d60a6d` + 965 dirty inputs (`3d1f8d88…349d`), identical after the build; built 21:10:01 → 21:13:30 MDT |
| defs / frontend / helpers | `20c3b183…cf8b` (unchanged) / `8d2bdc4a…e60e` / `1caa5f4d…f5c4e`; helper `dc3533ed…8c21`; FFmpeg source `7138d28c…78a3` |
| Contents | r66 plus receipt 67 (§2–§6) |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r67/` |

**New cards:**
- **V-98:** Recompiled Hockey Shop refusal.
- **V-99:** Recompiled speech / containment on the card.
- **V-100:** PS3 board Replace… → pending → copied-target save / restore on a copied NHL Legacy.
- **Runtime R37-BOARD-1:** which copy PS3 Legacy loads, and how the board draws (the user's call).

## 9. Remaining beta scope and blockers (ranked)

| # | Item | Owner | Next action | Kind |
|---|---|---|---|---|
| 1 | User cards on r67 (V-82 → V-100, A-XMA-5 → 8; game cards incl. R15-HEAD-1, R15-HAIR-1, R37-BOARD-1, R-RECOMP-LOOSE-1) | user | run the cards | observation |
| 2 | Coordinator review of receipt 67 (C37 transaction, DXT1-opaque decision, Recompiled guard / routes) and earlier unreviewed parts of 46–65 | coordinator | review | review |
| 3 | Recompiled read routes still missing (§4 table) — next: portrait / head preview member route (handoff 17) | developer | bounded unit | implementation |
| 4 | Recompiled equipment editing (guarded off until an archive writer or proven loose loader exists) | coordinator / user card | R-RECOMP-LOOSE-1, then a contract | withheld (in scope) |
| 5 | PS3 board: gallery for archive-only titles (NHL 11–13) and NHL 11 / 12 / 14 / 15 write runs | developer | archive-member venue listing; per-title runs | implementation |
| 6 | **Austrian phased schedule generation** | coordinator ← user card P1-A | contract decision | withheld (in scope) |
| 7 | X360 board paths, DXT5, resizing, archive re-key | coordinator | gates unchanged | withheld |
| 8 | Goalie mask runtime (G-MASK-1); NHL 07 / 08 generic heads (P34-G08); Recompiled saves / limits | user | cards | observation |
| 9 | Display caches keyed by `boot.big` length / time (display only) | developer | shared content-validated layer when touched | recorded |

Outside beta (unchanged): league / team creation / cloning / deletion and executable patching. Projects / Builds are deferred.
**Release inputs:** the MIT holder is unrecorded; the artwork stays internal-only; the coordinator's distribution review.

## 10. Cleanup

- **Removed:**
  - my stopped test runs' owned roots (`nhlms-r67-board-nhl13-*` twice, after containment and reparse checks);
  - every test's own scratch (removed by its guards; none remain);
  - the scratchpad logs;
  - the selftest scratch (removed by itself).
- **Left, not this run's:** `H:\TEMP\nhlms-league-campaign-all-97012`, `nhlms-logos-gen7-{102420,106396,106856}`,
  `nhlms-p3-op-{24084,99420}`, `nhlms_ps3_dds_portrait`. The research lane's C38 review folder is gone; it was not removed by me.
- **Kept:**
  - bundles r33 → r67 and the r48 → r67 zips;
  - the assembler's `stage` folder;
  - `C:\nhlms-toolchain\`, the fixtures, the shared `target\` cache (debug and release).
- E: has 1.9 GB free.
- The claim is `ready`. No process of this run is running.

## 11. Next task

**The Recompiled portrait / head preview member route (research handoff 17).** Build a member route in the player-asset scan
(`nocache.big` / `cache.big` / `cacherender.big`), with present / not-shipped / unknown kept apart, and test it against the X360
Legacy loose tree on both root layouts. A coordinator-returned defect in receipt 67 comes first.

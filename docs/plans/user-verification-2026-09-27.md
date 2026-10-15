# Studio verification plan — first comprehensive user pass

Prepared 26 September 2026 for the next user review. Baseline: developer receipts 23–26 and earlier delivered work; receipt 27's conditional additions are recorded below. Inclusion in a stable executable still requires the build checkpoint. The user decides what passes and what further evidence is unnecessary. Screenshots are optional; a direct observation is sufficient when the user accepts it.

This is a checklist of delivered functionality, expected refusals and open observations. It is not an instruction to implement or test features that only exist in research. NHL 21 E1 league-name acceptance remains closed.

Companion documents:

- [Per-title progress tracker](E:/Repositories/nhl-modding-studio/docs/plans/user-verification-2026-09-27-tracker.md)
- [Result and issue template](E:/Repositories/nhl-modding-studio/docs/plans/user-verification-2026-09-27-results.md)

## 1. How to run this without turning tomorrow into a marathon

**First: Studio behavior. Second: persistence on copies. Third: listening. Last: game acceptance.** Finish a title's Studio checklist while it is open. Defer its game experiments to a separate queue rather than repeatedly switching applications.

Suggested order:

1. NHL 21 PS4: learn the shared workflows and inspect the widest modern interface.
2. NHL Legacy Xbox 360: inspect the main older-console workflow and Xbox texture limits.
3. NHL Legacy PS3: compare platform-specific assets and sound-bank previews.
4. NHL 13 PS3: exercise archive-backed kits, nameplates and audio.
5. NHL 15 PS4, then NHL 08 Xbox 360: cover important schema/encoding exceptions.
6. Remaining titles in the tracker, grouped by platform; finish with Legacy Recompiled and patch-source variants.

Budget roughly 60–90 minutes for setup and the first full title, then 15–30 minutes per additional title for an abbreviated parity pass. These are planning estimates, excluding long archive saves and game boots. A comprehensive 27-target pass is several sessions of work, not a quick morning check. Tomorrow's best stopping point is a completed first six-title representative pass plus a clear issue list.

Run the shared negative/lifecycle cases once initially, then repeat where a title takes a materially different route. **Repeat the positive title checklist on every title/platform**: a shared backend does not prove correct labels, mappings or paths everywhere.

## 2. Preparation and a stable build

Have the development session prepare this at its next clean checkpoint, after tonight's work:

> Prepare a stable user-review build from the current working tree, without committing or discarding changes. Record build time, HEAD, dirty-source manifest, executable/launch method and the receipts included. Update this plan with only the newly delivered user-visible changes and superseded expectations. Prepare an isolated review workspace, source/dictionary inventory and required marked test images. Record exact copy roots and restoration methods for writable tests. Do not deploy, launch games, modify originals, or start the user's review. Keep the review build stable while development continues; avoid building a second huge cache unnecessarily.

Before opening Studio:

- [ ] Record the build identity. HEAD alone is insufficient while most work is uncommitted.
- [ ] Use the prepared desktop build. If launching from source, the verified command is `npm run dev` at `E:\Repositories\nhl-modding-studio`, not an assumed script in `app/`. A live dev checkout must not change underneath the review.
- [ ] Prepare a writable **review copy** for each title being edited. Attach that copy's asset root, not the original install, because some loose texture operations write immediately.
- [ ] Keep immutable originals and a manifest of baseline hashes for files that tests can change. Do not use committed anchors as writable targets.
- [ ] Use the title's own database, metadata and dictionary. The anchor labelled NHL 20 is an NHL 19 base copy: actual NHL 20 must come from the CUSA15439/private-fixture source. Treat NHL 19 base and patch 56 as separate inputs; NHL 15 PS4 base and patch are separate too.
- [ ] Record which root supplies the DB, dictionary and assets. Do not assume Recompiled's root contains `_compiled`; use its actual registration.
- [ ] Reserve enough space for the chosen archive, output, rollback and temporary files. Test one large title at a time rather than duplicating the entire library.
- [ ] Use clearly marked replacement images: `TEST <title> <family>` plus coloured corners, at the original dimensions. Keep a second contrasting image for replacement/undo checks. For resize tests prepare a supported smaller image; never overwrite your reference image.
- [ ] Prepare a small valid schedule export and a separate malformed copy; do not edit the only original export.

### Stable review build — record (prepared 26 Sep 2026 by the developer session; receipt 28 §6)

**Artifact verified by hash only.** It was built and copied, never launched: no app, game or emulator
run is part of this preparation, and nothing here is user acceptance.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-2026-09-27\nhl-studio.exe` — 49 183 232 bytes, SHA-256 `4dc5c540769b86858dd3be28255fdbe5e2282b3a146d20a51f4ca0b48455c502` (identical to `target\release\nhl-studio.exe` at copy time) |
| Launch method | double-click / run `E:\nhlms-review-build-2026-09-27\nhl-studio.exe` directly. **Not** `npm run dev` (that follows the live checkout) and **not** `target\release\nhl-studio.exe` (the next release build overwrites it). Requires the WebView2 runtime already used by earlier builds. |
| Built | 26 Sep 2026, 12:14:44 → 12:17 MDT; `npm run build -- --no-bundle` at the repo root (`tauri build`: `npm --prefix app run build` = `tsc && vite build`, then the release Rust build), reusing the existing `target\release` cache (no second cache) |
| HEAD | `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (branch `feat/phase5-asset-tooling`) + the dirty working tree below |
| Dirty source inputs | 828 entries (`crates/`, `app/src`, app package files, `src-tauri/`, `defs/`, Cargo / npm manifests) with SHA-256: `docs/validation/evidence/overnight-development-2026-09-24/review-build/dirty-source-manifest.tsv`; re-hashed after the build: **no drift**. Present in the tree but **not compiled**: `crates/tdb-speex/` (not a workspace member) and `crates/tdb-snd/src/easpeex.rs` (not referenced) — Speex work after this build |
| Frontend | embedded in the executable; `app/dist` at build: 55 files, tree SHA-256 `ae467dcfe11ba810af6274e64064d4c42b10a97e195c4a52c335ba530249e8a1` (`frontend-dist-manifest.tsv`) |
| `defs/` | snapshot beside the executable (`E:\nhlms-review-build-2026-09-27\defs`, 121 files, tree SHA-256 `200f04d97f9122b760587e4f40f73c81c8057be432fc62e06f99a84d7dd94a00`, equal to the repo at copy time; `defs-snapshot-manifest.tsv`). The release app resolves `defs` from its resource directory (the executable's folder) and only falls back to the build-time repo path if that folder lacks `defs\enums` — so **do not move the executable without its `defs` folder**, or the review would read the live, changing repo `defs/`. Resolution not launch-verified. |
| Included receipts | developer receipts through **28** (receipt 27: goalie consistency, tournament browse labels, gloves / helmets / skates; receipt 28: title-scoped nationality and birth-state country labels, the stored-code box, test / wording corrections). **Not included:** EA-Speex and XMA previews (not delivered at build time), goalie gear 4.5, any later receipt |
| Source / dictionary identities | `source-dictionary-inventory.tsv` (157 rows): every committed anchor database / metadata / dictionary file and the genuine NHL 20 private fixture (`E:\nhl-studio-fixtures\databases\nhl20-ps4-cusa15439`, main DB `1e200478…a57b`). The committed `anchors/ps4/nhl20` is an NHL 19 base copy — use the CUSA15439 source for NHL 20 |
| Copied asset roots | **none prepared** by this session (22 GB free on `E:`; one title at a time, as §2 says). Prepare a copy per title before any write test; record its root here |
| Restoration | the build folder is self-contained: delete `E:\nhlms-review-build-2026-09-27` to remove it. Review copies: restore from the originals by the baseline hashes you record; never write to anchors or the private fixture root |

Superseded expectations for this build: the receipt-27 notes in §5 P2 / P6 and §6 now apply (receipt 27
is included). **§6 identity note:** the NHL 10 / NHL 11–12 nationality correction **is included**
(card V-72 in the developer morning list); NHL 08 / 09 show the NHL 07 country table as before.
**Known difference in this build:** it labels the British national-team code "Great Britain" on NHL
10–12. A later correction (receipt 28 §4, not in this build) restores the operator-confirmed "England"
there, because the roster fixes the code but not the exact label. Treat "Great Britain" on NHL 10–12 in
this build as expected, not as a new defect.
New check (V-73): labelled dropdowns carry a stored-code box that accepts any in-range code.

### Second review bundle — record (prepared 27 Sep 2026 by the developer session; receipts through 33)

**Artifact verified by hash only; never launched by the developer.** The first bundle above is kept
unchanged for results already recorded against it; use this one for everything delivered after receipt 28.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r33-2026-09-27\nhl-studio.exe` — 49 594 368 bytes, SHA-256 `d44ad451372525ceaaa3602aa39b937e8ecbde2cc6e1476ecbe6a6e15953e7b8` (identical to `target\release\nhl-studio.exe` at copy time) |
| Launch method | run `nhl-studio.exe` from `E:\nhlms-review-build-r33-2026-09-27` (see `README-REVIEW.txt` there). Keep `nhl-studio.exe`, `defs\` and `helpers\` together; do **not** set `NHL_STUDIO_XMA2_HELPER` — Studio finds `helpers\nhlms-xma2-helper.exe` beside itself. WebView2 runtime as before |
| Built | 27 Sep 2026, 05:47:59 → 05:50:38 MDT; `npm run build -- --no-bundle` at the repo root, reusing the existing `target\release` cache |
| HEAD | `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` + the dirty tree below |
| Dirty source inputs | 841 entries with SHA-256 (`review-build-r33/dirty-source-manifest.tsv`, manifest SHA-256 `39504158…8cd8`); re-hashed after the build: **identical** (`dirty-source-after.tsv`); script `tools/review-build/manifest.py` |
| Frontend | embedded; `app/dist` 55 files, tree SHA-256 `9a596302…5ade` |
| `defs/` | snapshot beside the executable, 121 files, tree SHA-256 `7f0a98d2…e47b`, equal to the repo at copy time (includes the "England" wording on NHL 10–12) |
| XMA2 helper | `helpers\nhlms-xma2-helper.exe` SHA-256 `dc3533ed…8c21` (FFmpeg 8.1.3, LGPL-only minimal build, receipt 32) + `helpers\nhlms-xma2-helper-notices\` (licences, corresponding source, build record); 21 files, tree SHA-256 `1caa5f4d…f5c4e` |
| Included receipts | through **33**: 29 (EA-Speex previews), 30 (XMA route), 31 (goalie gear 4.5, X360 NHL 14 announcer, "England"), 32 (the helper), 33 (XMA speech / music banks, `.ast` XMA streams, Cancel, missing-helper states) |
| Source / dictionary identities | unchanged from the first bundle's `source-dictionary-inventory.tsv` (anchors unmodified; the NHL 20 private fixture's `nhl_game.db` re-hashed `1e200478…a57b`) |
| Copied asset roots | none prepared by this session (≈ 17 GB free on `E:`); prepare one title at a time |
| Restoration | delete `E:\nhlms-review-build-r33-2026-09-27` to remove it; the first bundle is separate |
| Identity file | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r33/build-identity.json` |

Expectations that differ between the two bundles: the British national-team code reads **"England"** on
NHL 10–12 in this bundle (first bundle: "Great Britain", expected there). Audio previews: this bundle adds
EA-Speex (A-SPX-2), Xbox 360 XMA previews (A-XMA-2, A-XMA-3) and the missing-helper state (V-75); goalie
gear numbers (V-74). The first bundle shows Xbox 360 XMA entries as framed only — expected there.

### Third review bundle — record (prepared 27 Sep 2026, 07:39–07:42 MDT; receipts through 38) — superseded by the fourth bundle below (kept unchanged for results recorded against it)

**Artifact verified by hash only; never launched by the developer.** The two earlier bundles stay unchanged for results
already recorded against them.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r39-2026-09-27\nhl-studio.exe` — 49 524 736 bytes, SHA-256 `cd00650b36e7c70f8373d22a411acfbde1ee4b5feb411c585047f23bd93a025b` (identical to `target\release\nhl-studio.exe` at copy time) |
| Launch method | run `nhl-studio.exe` from `E:\nhlms-review-build-r39-2026-09-27` (see `README-REVIEW.txt`); keep `nhl-studio.exe`, `defs\` and `helpers\` together; no environment variable needed |
| HEAD / inputs | `8d60a6d` + 852 dirty inputs (`review-build-r39/dirty-source-manifest.tsv`, manifest SHA-256 `d987eee5…02ba`), re-hashed after the build: **identical** |
| `defs/` | 121 files, tree SHA-256 `2f763a6e…0cd8` (includes the style-label scope of receipt 35) |
| XMA2 helper | `helpers\nhlms-xma2-helper.exe` `dc3533ed…8c21` + notices / licences / source (unchanged from the r33 bundle) |
| Frontend | 55 files, tree SHA-256 `10ccb38d…a4d7` |
| Included receipts | through **38**: 34 (pair-quota schedules, round-tag refusal), 35 (style labels per title), 36 (PS3 09 / 10 / 12 / 13 music; X360 NHL 09 banks), 37 (logo resize on NHL 07 → 11 packages), 38 (portrait / head reassignment; goalie mask paint notes) — plus everything in the r33 bundle |
| Identity file | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r39/build-identity.json` |
| Restoration | delete `E:\nhlms-review-build-r39-2026-09-27` |

Expectations that differ from the r33 bundle: skater style names on the Lines page / roster table (V-77); Xbox 360 NHL 09
sound banks now listed and previewable (A-XMA-4); PS3 NHL 09 / 10 / 12 / 13 music previewable (A-EAL3-2); pair-quota
generation (V-76); old-title logo resizing (T-3b); reassign portrait / head (V-78, P34-1); goalie mask notes (V-79).

### Ninth review bundle — record (prepared 28 Sep 2026, 13:39–13:50 session clock; receipts through 64) — **use this one**

**The extracted, verified internal review candidate** (`distribution: internal-review`); never launched by the developer; not
release-ready by its own record (copyright holder; artwork bundled internally by the 28 Sep decision, rights unrecorded). r59 and
earlier bundles stay unchanged.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r64-2026-09-28\NHL Modding Studio rc-20260928-r64\nhl-studio.exe` — 50 621 952 bytes, SHA-256 `b7ab3ce5d5a03cb1255a0a1b78d76a04a4608511a5935c997146206fd73577ba` |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r64.zip`, SHA-256 `6f88b5f7…56f5` |
| HEAD / inputs | `8d60a6d` + 952 dirty inputs (`review-build-r64/dirty-source-manifest.tsv`, `80bb06f6…cd65`), identical after the build |
| Included receipts | through **64**: r59 plus 61 (Hockey Shop failed undo keeps originals), 63 (PS4 head reassignment; applies need the checked plan), 64 |
| Identity | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r64/` and the archive's `BUILD-RECORD.json` / `MANIFEST.sha256` |
| Restoration | delete `E:\nhlms-review-build-r64-2026-09-28` |

New cards (MORNING-REVIEW): **V-94** PS4 head reassignment, **V-95** reassignment apply after a change. Earlier cards run unchanged on r64.

### Eighth review bundle — record (prepared 28 Sep 2026, 07:50–08:00 MDT; receipts through 59) — superseded by the ninth bundle

**The extracted, verified internal review candidate; never launched by the developer; not release-ready by its own record** (copyright
holder; artwork rights). r56 and earlier bundles stay unchanged.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r59-2026-09-28\NHL Modding Studio rc-20260928-r59\nhl-studio.exe` — 50 555 904 bytes, SHA-256 `940a24c097cd8d49d76ae3335ae63d6cc083227ff6482512574b410bdaccf4d6` |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r59.zip`, SHA-256 `aaf1e4fb…3401` (the bundle is its verified extraction) |
| HEAD / inputs | `8d60a6d` + 949 dirty inputs (`review-build-r59/dirty-source-manifest.tsv`, `f90e166b…17b4`), re-hashed after the build: **identical** |
| `defs/` / helper / frontend | `20c3b183…cf8b` / `dc3533ed…8c21` / `6d5ce1f9…9845` |
| Included receipts | through **59**: everything in r56, plus 57 (PS4 gear / mask names), 58 (PS3 venue pictures; X360 NHL 11 / 12 / 14 and PS3 NHL 14 in the gallery), 59 (Legacy Recompiled: player edits unblocked, sound banks from its archives) |
| Identity | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r59/` and the archive's own `BUILD-RECORD.json` / `MANIFEST.sha256` |
| Restoration | delete `E:\nhlms-review-build-r59-2026-09-28` |

New cards for this bundle (MORNING-REVIEW): **V-91** PS4 gear names, **V-92** PS3 venue pictures, **V-93** Recompiled player edits,
**A-XMA-8** Recompiled sound banks. Earlier cards run unchanged on r59.

### Seventh review bundle — record (prepared 28 Sep 2026, 07:01–07:10 MDT; receipts through 56) — superseded by the eighth bundle

**The extracted, verified internal review candidate; never launched by the developer; not release-ready by its own record** (copyright
holder; artwork rights). r53 and earlier bundles stay unchanged.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r56-2026-09-28\NHL Modding Studio rc-20260928-r56\nhl-studio.exe` — 50 486 272 bytes, SHA-256 `6eea54c1d5f9eee1c5cd9819866b9c8905e57b2d5b429bc2456773480252dfcd` |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r56.zip`, SHA-256 `d918ee6a…ae1e` (the bundle is its verified extraction) |
| HEAD / inputs | `8d60a6d` + 947 dirty inputs (`review-build-r56/dirty-source-manifest.tsv`, `aca15f8c…509f`), re-hashed after the build: **identical** |
| `defs/` / helper / frontend | `5e5c8e88…4a83` / `dc3533ed…8c21` / `64a7e387…198c` |
| Included receipts | through **56**: everything in r53, plus 54 (PS4 goalie mask pictures), 55 (portrait / head reassignment on PS3 NHL 09–14 and Recompiled), 56 (NHL 10 / 15 / Legacy stream sounds of every shape) |
| Identity | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r56/` and the archive's own `BUILD-RECORD.json` / `MANIFEST.sha256` |
| Restoration | delete `E:\nhlms-review-build-r56-2026-09-28` |

New cards for this bundle (MORNING-REVIEW): **V-89** reassignment on archive-held installs, **V-90** PS4 mask picture,
**A-XMA-7** listening (NHL 10 / 15 / Legacy streams). Earlier cards run unchanged on r56.

### Sixth review bundle — record (prepared 28 Sep 2026, 06:23–06:26 MDT; receipts through 53) — superseded by the seventh bundle

**The extracted, verified internal review candidate; never launched by the developer; not release-ready by its own record** (copyright
holder; artwork rights). r48 and earlier bundles stay unchanged.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r53-2026-09-28\NHL Modding Studio rc-20260928-r53\nhl-studio.exe` — 50 418 176 bytes, SHA-256 `139656966a5ff8cb58bb69e086432328c4f247b7ccba71ac90a145360894212f` |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r53.zip`, SHA-256 `9899c076…2a64` (the bundle is a verified extraction of it) |
| HEAD / inputs | `8d60a6d` + 944 dirty inputs (`review-build-r53/dirty-source-manifest.tsv`, `c92029ad…bfbf1`), re-hashed after the build: **identical** |
| `defs/` / helper / frontend | `4a8382c4…905e` / `dc3533ed…8c21` / `a8e47efa…1415` |
| Included receipts | through **53**: everything in r48, plus 49 (venue-art gallery), 50 (X360 in-bank XMA residuals incl. a false-refusal fix), 51 (PS4 goalie mask pairs), 52 (PS4 skater gear lists and pictures), 53 (NHL 11 / 13 / 14 `.ast` streams) |
| Identity | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r53/` and the archive's own `BUILD-RECORD.json` / `MANIFEST.sha256` |
| Restoration | delete `E:\nhlms-review-build-r53-2026-09-28` |

New cards for this bundle (MORNING-REVIEW): **V-86** venue art (Arena page), **V-87** PS4 goalie mask pair, **V-88** PS4 skater gear
card, **A-XMA-5** / **A-XMA-6** listening (X360 in-bank intro + loop / surround pairs; NHL 11 / 13 / 14 streams). Earlier cards run
unchanged on r53.

### Fifth review bundle — record (prepared 28 Sep 2026, 05:22–05:25 MDT; receipts through 48) — superseded by the sixth bundle

**The extracted, verified internal review candidate (receipts 46–48); never launched by the developer.** It is **not
release-ready** by its own record: Studio's copyright holder and the rights of the embedded artwork are unrecorded. The r43
and earlier bundles stay unchanged.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r48-2026-09-28\NHL Modding Studio rc-20260928-r48\nhl-studio.exe` — 50 022 912 bytes, SHA-256 `73e16eb56e2fe346082ec451941e1cb908d23d5d5736eed1e6d129de6346015b` |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r48.zip`, SHA-256 `5f59eba7…d25d` (the bundle is a verified extraction of it) |
| Launch method | run `nhl-studio.exe` from the extracted folder; keep `defs\` and `helpers\` beside it; no environment variable. Requires the WebView2 Runtime (README) |
| HEAD / inputs | `8d60a6d` + 916 dirty inputs (`review-build-r48/dirty-source-manifest.tsv`, `de319971…8e24`). Re-hashed after the build: one difference, `src-tauri/Cargo.toml` line endings rewritten CRLF → LF by the Tauri CLI, content identical |
| `defs/` / helper / frontend | `ed34ae89…8c53` / `dc3533ed…8c21` / `03cbd477…27b4` |
| Included receipts | through **48**: 44 (NHL 08–11 group labels, NHL 07 header division), 45 (Hockey Shop NHL 12 title naming), 46 (About & Licences, notices, startup message), 47 (Hockey Shop install hardening, NHL 12 meshes no longer pads), 48 (grouped schedule generation) — plus everything in r43 |
| Identity | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r48/` and the archive's own `BUILD-RECORD.json` / `MANIFEST.sha256` |
| Restoration | delete `E:\nhlms-review-build-r48-2026-09-28` |

New checks for this bundle (cards in MORNING-REVIEW): **V-82** About & Licences (Settings and the start-screen link: status,
blockers, notices, a licence text opens); **V-83** Hockey Shop on NHL 12 pads (a copy of the install only: plan shows "File naming:
NHL 12", Apply after review, `.bak` on overwrite, a changed file between review and Apply is refused); **V-84** League → Schedule →
Generate → groups (Swiss / Finnish on NHL 10 / 11 / PS4: "Use the shipped groups" fills only on an unedited registered copy; odd
counts ask for a host rule; per-pair hosts; preview / apply / undo / save a copy); **V-85** NHL 08–11 division labels by clubs
(receipt 44). Earlier cards run unchanged on r48.

### Fourth review bundle — record (prepared 27 Sep 2026, 12:56–12:59 MDT; receipts through 42) — superseded by the fifth bundle

**The extracted, verified release-candidate archive (receipt 43); never launched by the developer.** Earlier bundles stay
unchanged.

| Item | Value |
|---|---|
| Executable | `E:\nhlms-review-build-r43-2026-09-27\NHL Modding Studio rc-20260927-r43\nhl-studio.exe` — 49 758 720 bytes, SHA-256 `0ee1c38c3cb3fdcfbdedd6976ac66b1cca1a27ce26c07645046149b4dd498b80` |
| Archive | `E:\nhlms-release-candidate-20260927\NHL Modding Studio rc-20260927-r43.zip`, SHA-256 `7e81d48b…8776` (the bundle is a verified extraction of it) |
| Launch method | run `nhl-studio.exe` from the extracted folder (`README.txt`); keep `defs\` and `helpers\` beside it; no environment variable |
| HEAD / inputs | `8d60a6d` + 861 dirty inputs (`review-build-r43/dirty-source-manifest.tsv`, `f1465c8a…66c6`), re-hashed after the build: **identical** |
| `defs/` / helper / frontend | `2f763a6e…0cd8` / `dc3533ed…8c21` / `5f2dc1ba…c107` |
| Included receipts | through **42**: 40 (NHL 12 pads file check; NHL 08 / 09 mask paint), 41 (schedule generation keeps retained rows in place; V-76 recipe corrected), 42 (team goalie mask paints) — plus everything in the r39 bundle |
| Identity | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r43/` and the archive's own `BUILD-RECORD.json` / `MANIFEST.sha256` |
| Restoration | delete `E:\nhlms-review-build-r43-2026-09-27` |

Expectations that differ from the r39 bundle: goalie pads files on NHL 12 (V-74 / V-79 file check); generation with a kept
row (V-81); team goalie mask paints (V-80); the corrected V-76 recipe.

**Write behavior to distinguish:** DB/dictionary/pending edits may require Save or copied-target Apply; loose texture Replace can write the attached file and create `.bak` immediately. Undoing the DB is not a substitute for restoring a loose file or an already saved archive.

## 3. Result labels

For each check use **Pass / Fail / Expected refusal / Blocked by setup / Not run / Not applicable**. Add the exact title, platform, source and build. Keep three separate acceptance columns: **Studio**, **listening**, **game**.

A refusal is a pass only where this plan or the build's documented capability calls for it. An unexpected refusal is a defect or missing capability, not an automatic pass. An unresolved/raw label may be correct; a guessed title-inappropriate label is not.

If an edit writes a wrong file, changes another project, loses recoverability or misreports success, stop that write route, preserve the copied files/logs and continue independent read-only checks. Do not repeatedly apply the failing operation. Ordinary display defects can be logged while the rest of the review continues.

## 4. Shared Studio workflow — run fully on NHL 21, then once on Legacy

| ID | Action | What to verify |
|---|---|---|
| U1 | Open the registered title/copy, inspect title/platform/root, navigate Leagues → Teams → Players → Assets | Correct title and source; no unexpected blank panes, duplicate rows or stale previous-project content. Record missing sections instead of assuming a hidden menu. |
| U2 | Open a league, team, player and asset; search/filter, select a second row, return | Selection, displayed identity and file path agree. Long names and error text remain readable; controls are usable at your normal window size. |
| U3 | Make one small DB edit; inspect Pending Edits; Undo; Redo | Human label, correct record, before/after values; one logical operation has the intended undo behavior. Dirty indicator follows actual changes. |
| U4 | Save to the review copy; close and reopen it | The edit persists, project is clean after a successful save, dictionary association is retained as designed. Then restore the baseline and reopen again. |
| U5 | Edit → save → undo | Undo produces a genuine unsaved change relative to the saved baseline; it must not falsely show clean. Restore afterwards. |
| U6 | Attach the title's own dictionary, detach, reattach | Resolved text → explicit keys/unresolved state → resolved text. Other projects' dictionaries never supply labels. |
| U7 | Prepare a review, change an input or make another edit, then attempt the old Apply | Stale review refused or invalidated; it cannot silently apply a recomputed plan. Cancel/discard returns to a usable screen. |
| U8 | Start a slow copied-target review/apply; use status, Close and Cancel | Window remains responsive; Close cannot abandon an active operation; reason is visible and Cancel remains reachable. After cancellation settles, inspect the actual outcome and file state before proceeding. |
| U9 | Switch projects/teams quickly during preview/loading, including identical team IDs | Late results never replace another target's content, errors or pending state. No action goes to the previously selected target. |
| U10 | Close normally and reopen; where available try a refused close and Retry | Navigation follows acknowledged closure; no lingering inaccessible review or hidden running save. |

A forced crash, process kill, permission fault or interrupted disk write is **not** part of this first manual pass. Those recovery scenarios need a separately prepared disposable harness.

## 5. Repeat this core pass on every title/platform

Use one NHL team, one non-NHL team, a skater and a goalie. Record their names/IDs so a defect can be repeated. For older titles the names naturally differ; use the title's own roster rather than expecting a modern club.

### Leagues — L1–L7

- [ ] **L1 Listing/identity:** league names, raw codes/keys and evidence notes match this title. PS4 has editable league name keys; gen7 uses the dictionary route where mapped. NHL 07's unresolved league-label source is an expected limitation, not permission to invent a rename.
- [ ] **L2 Rename:** on a copy, preview and apply one supported existing-key rename/repoint; inspect pending, undo/redo, save/reopen and restore. On NHL 21 this checks Studio usability only; it does not reopen accepted E1.
- [ ] **L3 Groups:** inspect conferences/divisions or pools. Trace/inference/derived labels stay distinguishable; unknowns remain disclosed. On NHL 08–11, raw group codes can be correct. Test one valid membership change, inspect all affected fields, undo. In a representative PS4 title try a conflicting conference/division pair and confirm no change.
- [ ] **L4 Schedule browse/edit:** choose a bound league schedule; inspect resolved home/away clubs and dates. Swap home/away in one game or make a valid date change, preview, apply, pending, undo. Try an invalid calendar date: refusal without mutation.
- [ ] **L5 Export/import:** export the schedule, reimport unchanged and expect a no-op. On a separate export, make one supported change; inspect mapping and exact preview before applying. An unknown team key must request explicit mapping or refuse, never silently select a club. Save/reopen the edited copy, then restore.
- [ ] **L6 Generate:** where currently implemented, preview/apply a balanced round robin fitting the existing table, inspect totals and undo. NHL/AHL/grouped formats may still refuse: newer research is a proposal, not proof the generator has shipped. Record the build's actual capability. Never interpret stock anomalies as a request to repair the source automatically.
- [ ] **L7 Presentation:** open all available league-logo variants and presentation-imagery families. Thumbnails, language twins, source paths and dimensions should make sense; missing files versus unsupported decode must be distinct. Exercise one same-size replacement via the title's supported route under section 7.

### Localization and stored defaults — D1–D5

- [ ] **D1 Repoint and short text:** inspect key → text, change to another existing text, then a short supported custom text; preview, pending, undo, save/reopen on the copy.
- [ ] **D2 Growth/new key:** on one eligible dictionary per platform/era, preview a longer string and a new key. An anomalously ordered dictionary may refuse insertion while permitting existing-key text growth. Read the actual classification; do not assume all locales of a title share it.
- [ ] **D3 Glyph/rebuild:** try a character absent from the current encoding tree. Preview should offer the implemented rebuild or explain refusal. Missing font glyph warnings are separate from dictionary encoding. Inspect “this row/all sharers” scope where sharing exists; no unrelated label should change silently.
- [ ] **D4 Team text:** inspect a team's dictionary keys beside DB strings. Edit one supported team-text key, confirm only the chosen scope, undo and restore. Dictionary changes and inline City/Name changes are different tests.
- [ ] **D5 Game Settings:** PS4 NHL 15 DB catalogue; NHL 16–21 archive CSV catalogue. Inspect stored defaults and exclusions, preview one eligible change, apply, pending, undo, save/reopen. NHL 18–21 paired global defaults must be presented as a paired operation. On gen7, a clear unsupported/research-limited page is expected. A stored default is not a promise about a creation screen or an existing save.

### Teams, arenas and kits — T1–T8

- [ ] **T1 Names/colours:** inspect available identity fields; edit one inline name and one colour where present. Colour: Enter then blur must not duplicate the write; acknowledge → undo → reapply the same colour must work. Check correct colour after reopen. NHL 08–10 may have no team-colour fields.
- [ ] **T2 Roster/relationships:** inspect roster, retired numbers, draft trades and strategies where exposed. Try a roster move into a full destination: clear refusal, source unchanged. For a successful move use a deliberately prepared copy with a legitimate free slot; do not release a player from your real roster just to test it.
- [ ] **T3 Existing art-ID swap:** only where the current UI offers the established swap route: preview two teams, confirm affected shared rows, apply once, inspect both, undo. Creation of a fresh ID or arbitrary re-key is not this operation and stays outside the pass unless a new accepted contract ships tonight.
- [ ] **T4 Team logos:** inspect each family offered for the title, including thumbnails and missing/unknown states. Replace one marked family at a time, inspect pending and reopen, restore. Repeat supported storage routes separately, not every image in every family.
- [ ] **T5 Arena assignment/parameters:** later titles show assignment disclosure including current tenants and vacancy; test one assignment and undo. NHL 07–12 lacking the team link should not invent one. Inspect grouped parameters, colour swatches, horn notes and read-only unknowns.
- [ ] **T6 Arena assets:** gen7 venue/jumbotron/team-linked imagery is distinguished from unbound ice layers. PS4 venue listing reports all matching vault levels and live folder status; Honda Center can be ambiguous. No automatic choice/repoint should appear. NHL 15's missing Carolina folder is a recorded residual, not an automatic defect.
- [ ] **T7 Kits:** select home, away and an alternate if available. Inspect jersey/pant/sock(s), thumbnails, completeness and spare candidates. NHL 11/12 needs both sock files. PS3 12/13 uses render-archive members. On one disposable tree hide a required component and confirm incomplete/unknown rather than complete; restore it immediately. Do not do this on the original install.
- [ ] **T8 Nameplates:** present on the measured NHL 15/Legacy Xbox+PS3 and PS3 12/13 routes; marked as a naming association, excluded from kit completeness. Other titles may explicitly have no measured association. Inspect nameplate editor preview/save disclosure on a copied texture if exposed; no silent write before confirmation.

### Players and equipment — P1–P6

- [ ] **P1 Bio:** inspect weight, height, birth date, handedness, position and nationality for a known skater and goalie. Change weight to 200 lb: raw 50 on NHL 07–10, raw 80 on NHL 11 onward under the current contract. Undo, save/reopen a second edit and restore. Do not judge correctness solely from a plausible displayed number.
- [ ] **P2 Ratings/Overall:** compare roster and Attributes views. Expected current scale: +50 on NHL 07–10, +36 on NHL 11–Legacy, raw on PS4. NHL 13 uses stored Overall; titles without an established formula may show a dash. Test one real rating edit/undo. **If receipt 27 is included:** goalie consistency shows the stored value without rating tint/offset; 0/1/2 meanings remain unresolved, and edits use the declared range rather than a stock-value clamp. Its fitted Overall input remains unchanged. Record the build disposition first. Floppiness and PS4 goalie formula calibration remain separate unresolved questions.
- [ ] **P3 Text/range errors:** overlong surname refuses with byte limit and no pending change; a within-limit accented name is accepted only if supported. Out-of-range numbers refuse clearly. Birthday edits must be valid calendar values, not merely encodable bits.
- [ ] **P4 Contracts/import:** inspect cap-hit currency units. Witnesses include NHL 08 Iginla $7M and NHL 09/10 Crosby $8.7M, subject to the exact source roster. Export/import a small supported player change through the existing import workflow on a disposable copy; inspect mapping, preview, refusal and undo. Do not use external public-contract values to overwrite a stock record automatically.
- [ ] **P5 Portrait/head/name-call discovery:** inspect a known player and one with missing assets/recordings. Stored portrait flag, file presence and head mapping are separate facts. PS4 heads can remain unknown. A missing recording should be a named state, not a fabricated name.
- [ ] **P6 Equipment:** skater and goalie: sticks, gloves, helmets, skates and goalie slots. Inspect stored IDs, measured file sets and missing components. Legacy labels must not bleed into other titles; raw numeric IDs are correct where no title catalogue exists. Change one in-range ID, inspect pending, undo, save/reopen and restore. Receipt 27 covers skater glove, helmet/visor and skate database fields on 25 equipment-bearing anchors plus genuine NHL 20; NHL 07 custom-player fields and goalie gear are outside that sweep. This does not establish which asset an ID renders.

## 6. Audio and music — inspect every title; listen only to supported routes

**Receipt 27 checkpoint notes (26 Sep):** V-69 maps to P2 above; V-71 maps to P6. For V-70, when receipt 27 is included, open the NHL 17 World Cup or PS4 18–21 Champions Hockey League/Spengler league page and inspect the tournament schedule. These tables are browse-only. World Cup group-stage names use the measured join; CHL/Spengler participant slots stay unnamed, and Spengler opener placeholders carry an interpretation label. Editing/import/generation must remain unavailable. The genuine NHL 20 source is distinct from the mislabeled anchor. No Speex/XMA preview was delivered by receipt 27; the subsequent decoder decision is an implementation direction, not a new review-build capability.

**Known identity issue to track at build selection (corrected after the receipt-3 review):** NHL 10 nationality labels and NHL 11/12's Japan entry remain correction targets, with related birth-country/import paths to audit. C16's blanket NHL 08/09 field-display claim was withdrawn: its TOML-only comparison omitted Studio's existing geography override for their three-bit style schemas. Record whether the build includes the follow-up correction. A plausible country label alone is not a pass. Research observations about field width, zero-height placeholders, style and eye colour do not themselves establish a code fix or a new enum meaning.

For each supported title, first check listing, selection, source identity, codec, count, refusal and project switching. Then listen to a short clip at a comfortable volume. Write down wrong pitch/speed, clipping, noise, missing first syllable, truncation, a click at a join, or a wrong-sounding clip. A plausible clip is not proof that the game selects it.

| ID | Population / action | Expected observation |
|---|---|---|
| A1 | Commentary → events/sentences/contexts on supported gen7 and all PS4 titles | Search, expand and follow candidates; names, conditions and missing-sentence states are readable. Candidate/opaque fields remain labelled. Missing sentence does not mean the event is silent. |
| A2 | Team and player name-call chips on measured EALayer3 tuples | Correct bank/source is requested; diagnostic speech or a codec/coverage refusal. An unsupported announcer codec must not be passed through another decoder. |
| A3 | PS4 NHL 15/16/17 EA-MP3 preview | Compare candidate-A and frames-only on the same sample, especially onset. Record what you hear; do not assume one is correct. Later PS4 codecs may still refuse pending tonight's decoder work. |
| A4 | PS3 NHL 15 loose sound banks | 217 banks on the measured source; `playercalls.abk` record 0: 33,207 samples. Inspect named picker tree and source identities, then listen. Counts differing on another version require source identification, not an automatic failure. |
| A5 | Archive banks: PS3 09/10/12/13 | Expected measured counts: 186/199/219/213 banks. Test `playercalls`: record 342/382/0/0 respectively. Expected sample counts: 81,768 / 49,382 / 33,207 / 33,207. Bank and stream may reside in different archives or a loose stream; paths must be explicit. |
| A6 | PS3 NHL 15 `crowds.abk`, record 0 | Select each stereo pair separately; intro plus one body pass, loop point 29 shown. Listen for continuity and differences between pairs. Inferred pair layout is disclosed. Try one supported six-channel entry too if the prepared card names it. |
| A7 | Xbox 360 banks | Supported titles list/frame but XMA preview remains refused unless a reviewed decoder ships. Xbox NHL 09 bank-route refusal is currently expected. Do not mark these as missing-asset failures. |
| A8 | Arena horn link and chant text | Goal/period horn source links open the intended bank. Chant stays “number only / selection not traced”, with no causal claim from a numeric match. Repeat on PS3 13 archive route. |
| A9 | Gen7 music bank on measured title | Short preview of a stereo track, diagnostic label, correct bank identity; stop/switch releases the previous audio. Full-track listening is unnecessary for the first pass. |
| A10 | PS4 17–21 song catalogue | Filter by title/composer; own dictionary resolves titles/genres. Detach gives keys again. This lists available songs, not a proven assignment to the team. NHL 15 can correctly report no catalogue. |

## 7. File and archive routes — one treatment at a time, copies only

For every storage family exercised: record the original file(s), replacement image, reviewed scope and output path; apply; reopen the resulting copy in Studio; confirm the changed asset; restore via the transaction or prepared backup; verify original hashes. Keep the copy until restoration is confirmed.

| ID | Route | Desktop verification |
|---|---|---|
| W1 | Loose texture | Same-size visibly different replacement; backup/result path; preview after reopening; restore bytes. DB Undo alone does not restore it. |
| W2 | Xbox mipped texture | Precheck must finish before Write. Same-size may be partial: only audited/base levels change, packed tail preserved and disclosed. Changed-size mipped writes refuse. Do not follow old cards that resize an Xbox mipped jersey. |
| W3 | Resize supported PS3/GNF/XPR2 family | Supported smaller image reopens with correct dimensions. Multi-stream GNF and out-of-range dimensions refuse; NHL 07 table-at-EOF resize remains refused unless explicitly superseded. Select actual supported files from the prepared build's manifest. |
| W4 | PS4 archive texture, private copy | Shared siblings are disclosed; replacing one member changes only its intended row. Apply A, then B to the same member; Undo B restores pending A. Save/reopen the output and inspect A. |
| W5 | PS4 whole shared group | Separate experiment: all disclosed siblings in scope. Contradictory whole-group/private ownership must refuse without destroying the already pending plan. Restore before changing scope. |
| W6 | Gen7 EB archive | Correct member/codec/placement; copied-target review, apply, reopen/preview, restore. Include Legacy Xbox and PS3 independently. |
| W7 | BIG4 + `.bh` | Both artifacts are included where required. Use the current ordered-reflow route; inspect a follower member as well as the edited one. Do not use an old card's deliberate stale-sidecar treatment in this first pass. |
| W8 | NHL 07 archive cascade | On an explicitly prepared disposable package only: review the complete archive/index/mirror set, apply and restore all artifacts. If no current prepared route/manifest exists, mark blocked; do not improvise placement. |
| W9 | PS3 12/13 kit archive | Thumbnails and same-size replacement through the current chunkzip/member route, save/reopen and restore. Old “no encoder/no writes” notes are superseded for these measured members. |
| W10 | Disc image/container input | Open, identify, inspect and stage only for the first pass. Live image writes remain outside this plan; a successful staged edit does not qualify image mutation/recovery. |

Also test one stale texture review on a disposable target: prepare, externally replace the reviewed target using the prepared helper, then Write. It must refuse the stale identity, not silently accept the old disclosure. Do not manually swap files in your live installation to exercise this.

## 8. In-game queue — optional after the Studio pass

This section is a queue, not permission to improvise archive placement. Each experiment needs its title's currently prepared candidate, source manifest, launch target and restoration procedure. Old candidates may have been cleaned up or may predate writer fixes. Have them regenerated if needed. Record stock → treatment → restored observations; stop the emulator before replacing files. Preserve existing saves/profiles and use a separate test profile where a fresh mode is needed.

Do not combine unrelated edits into one candidate. An unchanged display is inconclusive: wrong active source, overrides, caches, unsupported treatment and another consumer can all remain possible. A crash does not by itself identify a format field as the cause.

| ID | Experiment | What to observe |
|---|---|---|
| G1 | League name on every unqualified title with a supported path | One reachable league screen before/after/restoration. NHL 21 E1 is already accepted; no repeat required. NHL 07 unresolved source is deferred. |
| G2 | Localization longer text, new key, rebuilt glyph | Separate treatments: existing key reads appended text; new key lookup; rebuilt dictionary and font rendering. One does not establish the others. Use title-owned dictionary and a known queried surface. |
| G3 | League/team logos, one family and placement route | Which screens change; unrelated/sibling logos remain as reviewed; for shared PS4 scope inspect the French/scouting sibling only where the selected row actually shares it. Reflow: inspect a later member too. |
| G4 | Texture resize/partial replacement | Check close and distant views. Xbox preserved tail can show old distant content by design. No mipped Xbox resize experiment. PS3/GNF acceptance is specific to the family/title tested. |
| G5 | Team inline text, dictionary text, colour, existing art swap | Separate saves/treatments; team select, roster, overlay and gameplay. Record which surfaces respond without assigning an untraced cause to those that do not. |
| G6 | Membership/schedule | Start from stock control. One membership or schedule treatment, then a fresh mode and relevant standings/opponent/date screens. Preserve a control and treatment save for analysis. Do not claim that a menu move proves schedule remapping. |
| G7 | PS4 stored default | Observe a new-mode setup option before touching it; use profile/control distinctions from the current settings card. Context/global/paired defaults are separate treatments. A changed menu default does not locate chosen-value persistence. |
| G8 | PS3 12/13 jersey replacement | Dress the exact edited kit; compare stock/treatment/restored. Loader/placement questions remain if unchanged. |
| G9 | Nameplate residual | NHL 13 PS3 Hershey kit corresponding to `name_1_138_2_cm`: observe its stock nameplate first. This is a shipped internal-name mismatch observation, not a rename experiment. |
| G10 | Horn/chant/name-call consumers | Observe stock missing-source horn behavior or club chant per current research card. No numerical match establishes selection. Any audio-ID treatment must have its own prepared copy and control; do not infer silence from a single non-trigger. |

The three-image `rendering/logo/texlib_*` family gets **asset inspection first** (L-LOGO-1): identify what the images depict without calling them rink logos in advance. Further game checks follow a specific hypothesis, not a guessed binding. Boards, tournament selectors and other research-only cards are secondary discovery work after core Studio verification.

## 9. Current-card corrections to use during this pass

- Old “PS3 12/13 kit archive write unavailable” and “loop/multi-channel preview unavailable” expectations are obsolete for the delivered measured routes.
- Ignore old Xbox mipped-resize instructions. Same-size replacement has a disclosed tail limitation; changed-size is refused.
- Do not use old causal verdicts such as “unchanged means the loose copy wins”, “blank means archive rejected”, or “no chant means unused”. Record the outcome and leave alternatives open.
- The moved-team schedule and dictionary-growth gates are distinct; a successful save/reopen is not game acceptance.
- Names for equipment are title-scoped. Derived group names and unresolved asset bindings must retain their evidence labels.
- Research-only schedule generation, re-key/creation and audio replacement are not missing UI regressions merely because their research is published.
- Fresh work completed tonight supersedes this baseline only when included in the recorded review build and its added cards. Otherwise mark it Not run, not failed.

## 10. Closeout for each title and for the day

Per title: undo pending edits, restore applied files, verify baseline hashes, close the project successfully, record the six result columns in the tracker. Keep a failed copy only with its path, reason and owner noted; do not mix it with the next clean run.

Send back the tracker plus concise issue entries. Highest-value first: wrong-target writes or lost state; wrong decoded values; broken save/reopen/restore; incorrect labels/mappings; unusable UI; then cosmetic issues. A direct description is enough—screenshots/logs are optional unless needed to identify the problem.

The review is complete when every applicable cell has a disposition, not when every unfinished feature is forced to pass. The user can explicitly accept an observation, defer a card or declare additional proof unnecessary.

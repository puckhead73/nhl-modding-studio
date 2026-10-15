# 68 — r67 verification freeze (developer receipt)

**Summary.** Feature development is frozen at the user's direction (dispatch
`docs/plans/development-20261001-verification-freeze.md`, review `docs/validation/development-20261001-receipt67-review.md`).
r67 is the fixed internal verification baseline. Nothing was built, relabelled, committed, deployed, installed or launched for this
receipt, and no source, defs or resources were edited; the only files written are this receipt and one ledger row. No further build
is scheduled. Feature work resumes only on an explicitly dispatched, bounded correction.

## 1. Baseline

| Item | Value (re-hashed 2026-10-01) |
|---|---|
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r67.zip`, SHA-256 `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a` |
| Executable | `E:\nhlms-review-build-r67-2026-09-28\NHL Modding Studio rc-20260928-r67\nhl-studio.exe`, SHA-256 `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65` |
| Build inputs | HEAD `8d60a6d` + 965 dirty inputs, manifest `3d1f8d88…349d` (`review-build-r67/dirty-source-manifest.tsv`) |
| Status | `internal-review`, `release_ready: false`; MIT holder pending; 53 artwork files internal-only |

The r67 archive `verify.py check` was re-run on 2026-09-29 (extracted to session scratch, since removed): ok.

## 2. Pending changes preserved

- The dirty checkout is kept as-is: HEAD `8d60a6d`, 7 165 `git status` entries. Nothing is staged.
- **Drift since r67:** the build-input manifest today is 965 entries, `fe0bc171…d86c`. The only differing input is `Cargo.toml`
  (r67 `bef13cfd…a22f7` → now `99c281ff…9d87`, modified 2026-09-30 10:30, not by this session). It adds `[profile.dev]` /
  `[profile.test]` `debug = 0`, `incremental = false`, which does not change the release profile r67 was built with. As the review
  says, the dirty checkout is therefore **not** byte-identical to r67's inputs; identify the binary by its hashes, not by HEAD.
- Recovery evidence, toolchains, fixtures, the shared `target\` cache and `E:\tmp` (including the `fhm-*` corpus edit backups) are
  untouched.

## 3. Bundles and packages (correction to the receipt 67 ledger row)

- **Earlier bundles were removed before the freeze**, at the user's explicit request on 2026-09-29. That covers review builds r33
  → r66 and the unnumbered 2026-09-27 build, the r43 → r66 zips and the assembler `stage` folder. Removed with them were two
  orphaned copied-target staging directories (1.9 GB) from the stopped receipt 67 board runs, and the old test scratch in
  `H:\TEMP`; containment and reparse checks were done. The receipt 67 ledger row stating "r66 / r64 / … / r48 kept" is
  superseded. Their build records under `docs/validation/evidence/` remain. Only r67 survives and nothing more will be deleted
  during the freeze.
- **Historical corpus package (2026-09-30, user request):**
  `E:\nhlms-release-candidate-20260928\NHL Modding Studio historical corpus schema6-20260814.zip`, SHA-256
  `716c69cbab2dec18b7d194b05de8199972f1d11fe644f41c29d663aa59c2c570`. It holds `historical.db` (schema 6, SHA-256
  `50a55d31…6d12`, quick_check ok), a README and the database's checksum. It is a data package, not a Studio build: r67 is
  unchanged and the source `corpus/historical.db` is unmodified.
- Two extracted folders next to the zips (`NHL Modding Studio rc-20260928-r67`, created 2026-09-29 17:54, and
  `NHL Modding Studio historical corpus schema6-20260814`) were not created by this session; they are left in place.
- E: has 132.0 GiB free.

## 4. Limitations carried forward (in scope, not waived)

1. **Speech read-route robustness (review findings 1–2):**
   - `locate_member` collapses open / table / sidecar failures into absence and takes the first path match;
     `read_member_unwrapped` and `bank_from_archives` inherit this;
   - `archive_beside` silently prefers a direct archive when both roots exist;
   - within-archive duplicate ambiguity is unresolved, and nothing ensures the preview uses the catalogue's selection;
   - `AudioDir::open` commits to Loose whenever the directory exists, so a partial loose tree suppresses the archive fallback
     and optional loose read errors read as absence;
   - Research-8's amendments are not fully implemented.

   None of these is a reproduced data-loss defect, and they do not block stock-install verification. They must be triaged
   before these boundary cases are called complete.
2. **PS3 board:**
   - NHL 11–13 have no gallery or entry point (backend only);
   - writes are tested on copied NHL Legacy and NHL 13 only, and the census admission count does not qualify NHL 11 / 12 / 14 /
     15 writes;
   - the 64 archived versus 67 loose package discrepancy is unreconciled;
   - which copy the console loads is unknown (R37-BOARD-1).
3. **DXT1 opacity is accepted for this pass** with its existing disclosure. Alpha is dropped rather than composited, so hidden
   RGB may show. Prefer opaque inputs, and check that the disclosure appears for transparent input. No encoder change.
4. **Recompiled:**
   - portrait / head previews, header logo, league imagery, kits, arena art and equipment previews are still missing;
   - Hockey Shop installation is guarded off. The refusal is a guard, not completed equipment editing.
5. **Withheld gates:** Austrian phased schedules (P1-A contract), X360 board paths, DXT5, resizing, archive re-key.
6. **Display caches** keyed by `boot.big` length / time: display only, recorded.
7. **External release:** not claimed. The MIT holder is pending; the artwork is internal-only and needs rights or replacement;
   and the agreed capability / validation gaps are separate from the packaging gates.

## 5. Cards

- **For r67:** V-82 → V-100, A-XMA-5 → 8, R15-HEAD-1, R15-HAIR-1, R37-BOARD-1, G-MASK-1, P34-G08 (receipt 67 §8–§9 and earlier
  receipts).
- **Run V-100 on a copied NHL Legacy first.** Stop a write test if unrelated bytes change, a save / reopen loses edits, recovery
  fails or originals become unavailable. Keep the affected copy and diagnostics.
- **Do not run R-RECOMP-LOOSE-1** (research handoff 15) as written: it has a pending correction (research `18-after-receipt8-corrections.md`).
  It is not a prerequisite for r67 UI verification.

## 6. Defect handling during the freeze

- **Triage:** for each user report, record reproduction, build hashes, title / platform and severity.
- **Urgent:** data loss, wrong-target writes, failed recovery, unusable startup or core save, and reproducible blockers of the
  agreed pass. Stop the affected workflow only. Unrelated safe verification continues.
- **Queued:** cosmetic issues and new functionality.
- **A dispatched correction:**
  - reproduce it on an owned copy;
  - add a regression test and fix only that issue;
  - run the affected checks;
  - deliver a separately numbered candidate (r68+) with exact hashes, a change list and the list of earlier results that need
    repeating. r67 is never overwritten or relabelled.
- **After delivery,** freeze again.

## 7. Checks and cleanup

- Re-hashed the r67 archive and executable (both match); re-ran `manifest.py inputs` / `compare` (one input, `Cargo.toml`,
  differs as above); took a disk reading.
- Nothing was compiled, tested or launched.
- Scratch: one manifest file in the session scratchpad, removed after this receipt.
- No process running. Claim set to `frozen`.

**Next:** none. Wait for verification results or a dispatched correction.

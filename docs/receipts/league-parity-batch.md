# LEAGUE-PARITY-BATCH — coordinated league-name parity preparation across every installed title (2026-09-18)

**Plain-language summary.** Every NHL title installed on this machine was identified from its own
package data and its own database, metadata and dictionary were located and hashed — inside the
PS4 archives, on the PS3 discs and game-data installs, and in the Xbox 360 and recompiled trees.
That inventory corrected the record in four places: the repository's "NHL 20" anchors are NHL 19's
base-game files (NHL 20 had no fixture until this batch extracted the install's own); NHL 19 ships
seven databases (base + six patches) and a stale loose game tree from July; NHL 15 PS4 uses an
older archive layout Studio cannot open and also ships a patched database; and the PS3 NHL 08/09
dictionary anchors are the Xbox 360 files. For the seven PS4 titles, the same one-cell league
rename that NHL21-E1 used was produced through Studio's own save path from each title's own files,
verified byte for byte, retained as hash-identified candidate archives (eight, NHL 19 twice), and
every working copy was restored exactly; the NHL 21 candidate reproduced the accepted E1 hashes
bit for bit. The E1 install/rollback helper now takes a per-title manifest (its 33 fixture cases
still pass) and its read-only status mode was run against every live install: six titles are
ready to place, NHL 19 is refused until the user resolves the stale loose tree, NHL 15 has no
archive route. For the twenty PS3 / Xbox 360 / Recompiled targets the batch traced where the
league name comes from — a fixed code→key table inside the game executable (read on NHL Legacy
Xbox 360) plus the title's own dictionary — and specified the one missing capability (a dictionary
text re-point writer, LOC-W1) with a byte-exact dry plan per installed title. Nothing was placed,
launched, written to a dictionary, staged, committed or pushed; no anchor changed.

---

## 1. Package and disposition

* **Package:** LEAGUE-PARITY-BATCH (this session's dispatch: coordinated league-name parity
  campaign across every supported title/platform; NHL21-E1 accepted and not reopened).
* **Disposition: prepared for the joint review + runtime pass.** Consolidated matrix and ordered
  runtime checklist: [league-parity-batch-2026-09-18](../validation/league-parity-batch-2026-09-18.md).
  Package directory: `E:\Repositories\nhl-modding-studio\local\runs\league-parity-batch\` (hash list
  `SHA256SUMS.txt`, `87cdb192…`, 77 entries).
* **No** live replacement, game launch, staging, commit, push, anchor replacement or per-title
  approval stop. No source under `crates/`, `app/` or `defs/` was changed.

## 2. What was done (by task)

1. **Identity + sources (read-only).** PS4 `param.sfo` parsed for 11 CUSA installs (7 in scope);
   the archive front tables scanned in every `databigs/*.big` (both EB row layouts) for
   `nhl_game.db` / `-meta.xml` / `eng_us.db` / `eng_us-meta.xml` with member offsets, forms and
   SHA-256; PS3 `PARAM.SFO` + RPCS3 `games.yml` for 9 disc titles and their `dev_hdd0` game-data copies;
   Xbox 360 loose and packed Legacy trees; the recompiled tree. All 105 database/dictionary/metadata
   anchors re-hashed against `docs/audits/data/anchors-manifest.csv` (0 mismatches). Findings:
   record §1–§2 (NHL 19/20 provenance **resolved** — both anchors are NHL 19 base; NHL 20 install
   differs and had no fixture; NHL 19 patch set and stale loose tree; NHL 15 PS4 layout
   `0x00100400` + patch DB; PS3 08/09 dictionary anchors = Xbox 360 files; Legacy PS3 hdd0 DB modified).
   No sibling dictionary was substituted anywhere: each row attached its own title's dictionary
   from its own install.
2. **Native route per target.** PS4: the existing Registry route (E1's), reproduced. Gen7: the
   code→key binding traced as a static enum-name table in the Legacy Xbox 360 executable (22 keys in
   code order, matching NHL 16's `stockleagues.name` list and `gen7-nhllegacy.toml`); keys verified in
   every installed dictionary, resolved by `hashid` (case-folded); the equivalent operation and the
   **minimum localization support** (LOC-W1: 4-byte text-pointer re-point + body CRC, staged as a
   copied-target artifact) defined; the Xbox 360 / Recomp `cacheboot.big` binds in Studio's stager.
   Reader support was explicitly not counted as edit support.
3. **Offline exercise.** PS4: 8 rows through Studio (open → attach own dictionary → resolve by key →
   `set_field` → pending (1 cell) → archive-member review → apply → record/byte audit → candidate
   retained → restore → reopen both) — all OK; NHL 15 loose-only. Gen7: 11 catalog rows on the
   installs' own databases + read-only dictionary re-point dry plans. Deploy helper generalised
   (manifest-driven) and its fixture suite rerun: **33/33**; live `status` (read-only) run on all 7
   archive rows.
4. **Blockers documented, not worked around:** EB-V1 (NHL 15 PS4 layout), LOC-W1 (gen7 dictionary
   writer/route), REG-1 (anchor relabelling — coordinator), P4 (hash-based label fallback), VOC-1
   (research). No localization writer, emulator change or unrelated feature was started.
5. **Deliverables:** one consolidated matrix (28 rows, separate PS3/Xbox rows) and one ordered
   runtime checklist (record §5–§6).

## 3. Files changed

| Path | Change |
|---|---|
| `docs/validation/league-parity-batch-2026-09-18.md` | **new** — inventory, provenance resolutions, routes, offline results, consolidated matrix, runtime checklist, packages, cleanup manifest |
| `docs/receipts/league-parity-batch.md` | **new** — this receipt |
| `docs/plans/league-identity-parity-matrix.md` | one dated banner before §0 pointing at the batch record and naming the two provenance corrections (rows themselves untouched, pending review) |
| `local/runs/league-parity-batch/**` | **new, untracked package** (8.4 GB): `artifacts/` (2.8 GB, 7 candidate archives + 8 loose evidence DBs), `copied/` (5.5 GB restored roots = secondary rollback sources), `sources/` (131 MB install-own members), `manifests/*.json`, `deploy.ps1` (`090c6da1…`), `tests/` (suite + `results.txt`), `reports/` (per-row transcripts, gen7 rows, batch log, cacheboot bind), `prep/` + `probe/` sources, `gen7-offline.sh`, `SHA256SUMS.txt` |

## 4. Key decisions / conflicts

* **Own sources over anchors.** Every PS4 row was built from the install's own archive members
  (hash-bound), not from anchors, so the NHL 20 row is a genuine NHL 20 measurement; anchors were
  used only for provenance comparison and (NHL 09/10 PS3) metadata the discs do not ship.
* **NHL 19 prepared twice** (base and `patch_inxp_56`) so the runtime pass can discriminate the
  loader's source; the stale loose tree is left for the user to move (the helper refuses meanwhile).
* **NHL 15 PS4:** documented as blocked (EB-V1) rather than adding the version to the reader inside
  this package; sources were still extracted with a documented fallback parse so the loose row is
  the install's own bytes.
* **Gen7:** the exact dictionary edit was planned byte-exactly but **not written** (no writer
  exists; starting one is LOC-W1, for Codex to issue).
* **Conflict noted, not resolved here:** the parity matrix §0.2 and the offline campaign's "NHL 20"
  rows describe NHL 19 base bytes; the `tdb-loc` fixture test names the `nhl20` dictionary as NHL 20.
  Relabelling is REG-1 (C0-REG4) — coordinator.

## 5. Checks run

* Batch prep binary (release, package-owned target dir, removed): 8/8 rows `OK`, 0 unexplained byte
  ranges, every restore `restored`, every candidate reopened with the intended key and unchanged
  members; NHL 21 outputs = accepted E1 identities (`343404…` / `4876cd2b…`).
* `tests\run-tests.ps1` against the batch `deploy.ps1`: **33 passed, 0 failed** (one expected-label
  string in the suite updated for the renamed config check).
* `deploy.ps1 -Mode status` (read-only) on nhl21/20/19-base/19-patch56/18/17/16: all `S0-stock`;
  exe identity PASS (`aeb04892…`); NHL 19 rows `REFUSE` on the loose-tree gate (expected).
* 105 anchors re-hashed: 0 mismatches. Index empty; `git diff --cached` empty; no emulator process.
* Not run: `cargo test` (no crate source changed); Studio desktop build (not needed for this package).

## 6. Unresolved questions (for Codex / the user)

1. Anchor relabelling: adopt `anchors/ps4/nhl20` as **nhl19-base** and capture a real NHL 20 set
   (and the PS3 08/09 disc dictionaries)? (REG-1)
2. NHL 19: move the stale loose tree aside for the archive experiments, or run the loose (E2-style)
   experiment instead?
3. Authorise LOC-W1 and EB-V1 as the next implementation packages?
4. Runtime pass order as proposed (18 → 17 → 16 → 20 → 19 patch → 19 base; PS3/X360 G0-only), and
   the evidence class per boot (F12 vs attested user report)?

## 7. Cleanup manifest

* Removed: package build dir `local/runs/league-parity-batch/target/` (1.4 GB); fixture roots
  `tests/fx/`; session scratch scripts and extracted members (`sfo.py`, `eb.py`, `ps3inv.py`, `ext/`).
* Retained (dependency: review + runtime pass): everything in §3's package row; the pre-existing
  `local/runs/league-nhl21-e1/` review inputs are untouched.
* Free space after cleanup: E: 29.1 GB, H: 27.5 GB. No live install file modified/renamed/created;
  no emulator launched; anchors unchanged.

Stopping here for Codex review.

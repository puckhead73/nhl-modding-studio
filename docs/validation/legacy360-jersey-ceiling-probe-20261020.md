# NHL Legacy (Xbox 360) — jersey-index ceiling probe mod (20 Oct 2026)

**Summary.** A tester asked how many kits a team can functionally carry. The jersey registry
(`stockteamjerseys` / `xjgw`) stores the per-team `jerseyindex` in 5 bits, so 32 is the hard
representable ceiling; nothing says the game actually loads 32. This probe builds a `.nhlmod`
through the production project → package → installer path that packs the Anaheim Ducks to exactly
32 kits, so one in-game run answers the question. Evidence level for the ceiling: **implemented +
automated-tested install/uninstall on a disposable tree; not game-verified.**

## What the mod does

- Base: the bundled vanilla `anchors/xbox360/nhllegacy/databases/nhlng.db`
  (sha256 `105d2186…1c230`). The installer's identical-base rule refuses any other database.
- The Ducks (teamid 0) keep their 9 stock kits (indexes 0–8).
- 23 other NHL teams each donate their **last-indexed, non-default** kit. Each donor row is moved
  with `reassign_jersey`: only `teamid` → 0 and `jerseyindex` → 9…31 change. The art id, style and
  variant stay, so the Ducks' new entries show the donor teams' textures. Taking a donor's last
  index keeps its own list contiguous; skipping default kits keeps every donor's home/away intact.
- Donors: BOS, BUF, CGY, CAR, CHI, COL, CBJ, DAL, DET, EDM, FLA, LA, MIN, NSH, NJ, NYI, NYR, OTT,
  PHI, ARZ, PIT, STL, SJ. Untouched NHL controls: WPG, MTL, TB, TOR, VAN, WSH.
- 46 cells, no rows added or removed. Package: `Ducks-32-jersey-ceiling-test-1.nhlmod`
  (39,320 bytes, package id `f000889e…5583`, sha256 `937cdd75…a7663`).

Why 23 and not 29: Legacy's NHL teams carry 4–13 kits each, not 3. With the Ducks' 9 stock kits,
23 donors reach the 32 ceiling. Reaching it with 29 donors would mean deleting six Ducks kits, which
the package format does not carry.

## How it was built and proven

Driver: `crates/studio-api/examples/legacy360_jersey_ceiling_mod.rs` (run with one argument, the
work folder). It creates a fresh Legacy/Xbox project from the bundled baseline, resolves team names
through the Studio (asserts team 0 is the Ducks and every donor is an NHL team), stages the 23 moves,
saves, builds the package, then on a disposable Xbox install tree (placeholder `default.xex` + the
vanilla database): preflight → apply (46 cells committed) → re-read (32 Ducks rows, indexes exactly
0–31, installed bytes equal the project's saved database) → restore (database byte-identical to
vanilla). Output: the package, `receipt.json` and the Studio project under
`tmp/legacy360-jersey-ceiling/` (git-ignored). The disposable install tree and install records were
deleted after the run; the receipt records their outcomes.

## Engine changes this needed

- `EditTransaction::reassign_jersey(record, team, jersey_index)` (tdb-edit), plumbed through
  `EditSession`, `Workspace` (roster mirror like `clone_jersey`) and `Registry`. The jersey team
  field is the row's key, which the scalar seam refuses by design; this is the structural operation
  that legitimately reassigns it. Test: `tdb-edit/tests/txn.rs::reassign_jersey_moves_a_kit_and_keeps_its_asset_id`
  (PS3 Legacy anchor; move, index minting, asset id untouched, three refusals leave bytes unchanged).
- Installer replay (`EditSession::set_project_revert`) now admits `xjgw.mCXg` cells when the
  destination team exists and fits the field; the row was already resolved by the author's baseline
  key + row fingerprint and its current value checked. No Tauri command or UI yet.

## Checks

- `cargo test -p tdb-edit --test txn reassign_jersey` — 1 passed.
- `cargo test -p studio-api --test mod_package` — 4 passed.
- `cargo test -p studio-api --test onb003_corrections` — 9 failures, all in roster-save project
  import ("Save files belong in Save Editor") and the roster sidecar plans, i.e. upstream of package
  replay and unrelated to `xjgw`. Not re-run at HEAD because stashing the other sessions' uncommitted
  work was not safe; recorded as observed, not caused.

## Step 2 (21 Oct 2026): the game reads `cacheboot.big`, not the loose database

The tester installed the package through the mod manager (install record 7, committed to
`db\nhlng.db` in the extracted Xbox 360 folder, bytes equal to the project's saved database) and
saw stock Ducks jerseys. The loose `db/nhlng.db` was extracted by the Studio from `cacheboot.big`
(the extraction marker records the member), and the game loads the archive copy. The package
format and installer only carry cells to the loose database, so on Xbox 360 the mod-manager route
is inert for database edits — a known follow-up.

At the tester's direction the saved database was then written **into** `cacheboot.big` through
the copied-target archive-member route, as the trial for archive-in-place modding:
`crates/studio-api/examples/legacy360_jersey_ceiling_archive.rs`. The project's `working/` folder
(a copied-target root) received a copy of the game's archive; the review staged member
`db/nhlng.db` from the project's primary database; apply committed (txn
`000003-1792563033661-9e9b02fa4385`); the member read back from the rebuilt archive equals the
saved database; archive length unchanged (31,705,408 bytes, sha256 `57378b81…b86aac`, vanilla
`b558b4f6…`). With `--install` the rebuilt archive was placed in the game folder after a verified
backup `cacheboot.big.nhlms-vanilla-backup`. Uninstall: copy the backup back over the archive.
Qualification state reported by the engine: *unqualified* (native loading unverified — that is
what the in-game run tests). Receipt: `tmp/legacy360-jersey-ceiling/archive-receipt.json`.

## Result (21 Oct 2026): game-verified

The tester launched NHL Legacy (Xbox 360, extracted folder, Xenia) with the rebuilt
`cacheboot.big` and reported **all 32 jerseys visible on the Ducks**. Two facts are therefore
**game-verified** on that exact title/platform/build:

1. a database edit written into the `db/nhlng.db` member of `cacheboot.big` is what the game
   loads (and, from the earlier inert install, the loose `db/nhlng.db` is **not** read);
2. a team carrying 32 kits — the `jerseyindex` 5-bit ceiling — lists all 32 in the jersey select.

Not verified by this run: a *grown* jersey table (rows added). The probe moved rows; the table
stayed at 703. Growth is implemented and automated-tested (`clone_jersey_grows_a_packed_registry_and_the_team_ceiling_is_32`)
and needs its own in-game run.

**Revert.** `cacheboot.big` restored from `cacheboot.big.nhlms-vanilla-backup` (sha256 back to
`b558b4f6…`), the backup removed; the loose database uninstalled through the mod manager's own
restore (install record 7 → `restored`, `db/nhlng.db` back to `105d2186…`). The game folder is
vanilla again.

**Follow-ups opened by the result.** (a) On Xbox 360 the mod manager must write database edits
into the archive member, not the loose file; (b) `clone_jersey` now grows a packed registry and
both jersey operations enforce the 32-per-team ceiling; the Add New Jersey dialog shows the team's
count against 32.

## Open questions for the in-game run

1. Does Anaheim's jersey select list and render all 32 entries? A truncated list, missing textures
   past some index, or a crash gives the real functional ceiling.
2. Separately from this probe: the table's own metadata declares `maxinsert="60"` for
   `stockteamjerseys`, and `tdb-core::append_record` can grow a packed table. Whether the game
   accepts a *grown* jersey table is a different test (add rows rather than move them).

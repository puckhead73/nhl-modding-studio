# Receipt 62 — Legacy Recompiled player edits through the app's own path (regression for receipt 59)

**Developer-verified; not Codex-accepted; nothing launched; the registered install was only read.** HEAD `8d60a6d` (unchanged).
Claim `dev-recompiled-public-edits`. Dispatch after receipt 60 §2.

## 1. Plain-language summary

- A Recompiled project opened under the app's own title id (`legacyrecomp`) can now change a player's weight, a rating, the birth
  day and the contract dollars. All of these were locked before receipt 59.
- The test follows the steps a user takes: find the install, extract its database, open it, look at the player, make the changes,
  review them, undo, redo, save a copy and reopen it.
  - The Roster Builder import works on it too.
  - Opening the same database under a wrong title (NHL 09) refuses both the edit and the import, and changes nothing.
- The older spelling `nhllegacy-recompiled` still works for compatibility, but is no longer the only thing tested.
- Nothing is inferred for Recompiled roster-size limits, save files or which archive copy the game loads.

## 2. Test — `crates/studio-api/tests/recompiled_public_edits.rs` (new, `--ignored`)

1. **Discovery on the registered install** (`H:\Emulators\games\XBOX\NHL Legacy Decomp Test`) and on its `game` folder: no loose
   database, `extract_needed` = true.
2. **The app's extraction route** (`extract_install`) runs on an owned scratch copy of `game\cacheboot.big`, never in place. It
   extracts 61 files. Discovery on the copy finds `game\db\nhlng.db`, whose SHA-256 `105d2186…` is the X360 Legacy anchor R128
   recorded, and the archive's own `nhlng-meta.xml`.
3. **Open** with `open_workspace_from_paths(Some("legacyrecomp"), recomp, …)`. For the first skater, weight, birth day,
   `contractdollars` and a six-bit rating on the 36 offset are shown editable, and the header composes weight and birth date.
4. **Edit** each cell to a new in-range value.
   - Pending review is dirty with at least 4 cells.
   - Four undo steps restore the originals; four redo steps re-apply them.
   - Save to a copy, reopen as `legacyrecomp`, and all four values match.
5. **Import seam:** a one-person synthetic corpus lowers with no blocks (`players_after = before + 1`) and commits "created". One undo
   removes it.
6. **Wrong title** (`nhl09` on Recompiled): `set_field` refuses with "not editable…" and the value and pending state are unchanged.
   `import_preview` refuses with "cannot lower the import".
7. **Alias:** `nhllegacy-recompiled` still edits.

Result: **1 / 1 passed** (30.5 s). Receipt 59's red run (`encoding_contract`, the same refusal text) is the failing baseline.

## 3. Limits

- One skater on one registered install. Other Recompiled builds are not covered.
- Save sidecars, roster-save ceilings, loader precedence and in-game results are outside this test.

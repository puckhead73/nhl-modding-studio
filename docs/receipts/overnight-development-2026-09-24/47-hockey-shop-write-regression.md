# Receipt 47 — Hockey Shop NHL 12 pads: applied installs on a disposable copy, title from the open project, reviewed-plan apply, rollback; NHL 12 mesh-as-pad defect fixed

**Developer-verified; not Codex-accepted; nothing launched; the registered install was only read.** HEAD `8d60a6d` (unchanged;
nothing staged). Claim `dev-hockey-shop-write-regression`. Direction: [dispatch after receipt 45](../../plans/development-20260928-after-receipt45.md) §2.

## 1. Plain-language summary

- **Installs on NHL 12 pads were applied and re-read for the first time**, on a disposable copy of 44 real files.
  Create cloned donor 54 to a new id 60, and overwrite replaced id 11's colour texture.
  - Every new file carries the new number in the right place.
  - The texture was spliced in, and the file still parses.
  - No other file changed: not the donor, not the other pad, not the shared meshes.
  - The overwrite kept the shipped original as `.bak`, and a second install did not replace that backup.
- **Defect fixed (found by this regression):** on NHL 12, the shared pad meshes (`goaliepad_0_2.rx2` and so on) were
  read as pads 0, 2 and 3. Receipt 45's "44 pad ids" was really 41 pads plus 3 meshes, and its 3 failed syncs were
  those meshes. Picking one of them as a donor would have cloned shared mesh files under a new pad number. NHL 12
  now reads 41 pads, and sync adds 41 with no failures.
- **The title is no longer the page's guess.**
  - Plan, apply and sync take the open project's recorded title.
  - A request naming another title is refused ("… the open project is nhl12"), and so is an unknown title string.
  - Pads, whose naming differs by title, are refused when no title is known, instead of using the default reading.
- **Apply only does what was reviewed.** The plan carries a fingerprint of the title, the store image and every file
  read or written, with their bytes. Apply refuses without it or when anything changed, so a stale review writes
  nothing.
- **A failed install undoes itself.** Each file is written through a temporary file and swapped into place. If a
  later step fails, the files already written are restored or removed, along with any new backup. A create also
  refuses to replace any existing file.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/equipment_slots.rs` | NHL 12 `goaliepad` shape requires a map suffix; `matchers_differ`; unit test |
| `crates/studio-api/src/hockey_shop.rs` | `KNOWN_TITLES`, `resolve_title`; plan: title, fingerprint, create-collision block; apply: reviewed fingerprint, `WriteTxn` (temp + rename, rollback); unused `atomic_write_with_backup` removed; 2 unit tests |
| `crates/studio-api/src/dto.rs` | plan `title`, `plan_fingerprint`; request `reviewed_fingerprint` (bindings regenerated: 2 files) |
| `crates/studio-api/src/registry.rs`, `commands.rs` | `project_id` on sync / plan / apply; `project_title` from the workspace |
| `app/src/pages/HockeyShop.tsx`, `app/src/lib/client.ts` | pass the project id; plan query keyed on project, roots and title; apply sends the reviewed fingerprint; plan shows "File naming: NHL 12" |
| `crates/studio-api/tests/hockey_shop_title_spec.rs` | 41-id assertion; the applied regression (below) |

## 3. Evidence

- **`hockey_shop_title_spec` 2 / 2 (`--ignored`).** Source: real X360 NHL 12 `rendering/goaliepad`, read only.
  - **Sync:** 41 title ids versus 2 from the default reading; 41 synced, 0 failed; sync without a title is refused.
  - **Applied install:** on a copy under `%TEMP%\nhlms hockey shop nhl12 <pid>\`, a path with spaces, removed by
    its guard. It holds donor 54 (20 files), pad 11, and four shared meshes / ambient maps.
    - **Title context:** omitted, wrong (`nhl13` against project `nhl12`) and label-as-id (`NHL 12`) are all
      refused. Omitted and matching title with the project give the same fingerprint. Planning writes nothing.
    - **Apply refusals:** no fingerprint; a wrong title; a donor byte changed after review. All refused, and the
      tree is unchanged.
    - **Create 54 → 60:** 20 files, all numbered 60 at position 2. Nineteen are byte clones; the colour file
      `fegoaliepad_0_60_0_dm.rx2` differs and parses at the plan's size. There are no `.bak` or temp files, plus the
      install ledger. All 44 prior files are unchanged.
    - **Overwrite 11:** only `fegoaliepad_0_11_0_dm.rx2` and the ledger change; the `.bak` equals the shipped file.
      A second overwrite keeps that `.bak`. Id 60 is now planned as an overwrite. A case-different existing name at
      62 is planned as an overwrite, not replaced.
- **Unit tests:** `hockey_shop` 5 / 5. The first run of the rollback test **failed** on its own guard case: rollback
  tried to remove a pre-existing folder at a destination. The fix refuses a non-file destination before recording
  it; re-run green.
- **Regressions:**
  - `equipment_slots` 11 / 11; domain `equipment` 25 / 25;
  - receipt 40's `goalie_gear_file_positions` 1 / 1 on real trees: NHL 12 pads 54 still lists 20 files;
  - studio-api `bindings` 25 / 25; `tsc` clean.

## 4. Limits

- The written files were not loaded in the game, and the page was not exercised on the desktop.
- Other titles' pads keep the default reading. The shared-mesh question was measured only for NHL 12 here. Other
  slots and titles were not re-swept.
- The fingerprint proves the reviewed inputs are unchanged; it is not a server-held review token. A second Studio
  window writing the same files between the fingerprint check and the write is not guarded (as elsewhere, no claim of
  protection against concurrent external writers).
- Without an open project (the legacy file-pick path), a request's own title is still accepted if it is a Studio title
  id.

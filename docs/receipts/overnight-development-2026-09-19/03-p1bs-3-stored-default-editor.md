# Receipt 03 — D3 / P1-BS-3: one-cell stored-default editing (NHL 15 PS4)

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

On NHL 15 (PS4) the Game Settings tab can now change one stored catalogue default at a time — the global
default of one of the 21 league/competition settings, or the default one screen stores for it — through a
preview that the server holds and an Apply that only names that preview. The write touches nothing but
that one cell and the single checksum that covers it; Studio verifies this after every apply and rolls
back otherwise. Undo, the pending-changes list, saving, reopening, the copied-target transaction and its
restore all work as for other edits. Screens without a stored default, values outside the setting's
range, the excluded rows and stale or foreign previews are refused with the reason. NHL 16–21 still refuse
(their catalogue is compressed text inside an archive; D4). Whether the game shows or uses an edited
stored default is still a user test.

## 1. Package, disposition, scope

- **D3 / P1-BS-3** — delivered for NHL 15 PS4, developer-verified, pending independent review. The
  NHL 16–21 extension is deferred to D4 (packaging), as the brief allows.
- Scope: handoff 22 rev. 3 §2 with the P1-BS-R1 acceptance note taking precedence: 21 explicit ids,
  row identity / declaration / min-max-size / context validation on the opened bytes, stock anomalies
  preserved, no exemption of earlier edits, no ticket needed to reopen an edited file; server-held plan
  under an opaque project-instance-bound ticket + workspace revision; Apply never recomputes; only the
  selected cell bits and the covering body CRC change, header CRCs unchanged; no executable/profile/
  save writes.

## 2. Changed paths and hashes (baseline `8d60a6d`; prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/studio-domain/src/settings_edit.rs` | `ed685b80cc5744f8…` | **new** — `DefaultTarget`, `DefaultEditPlan`, `DefaultEditRefusal`, `plan_default_edit`, `accounted_diff`, `verify_single_cell_diff`, `checksum_offsets` |
| `crates/studio-domain/src/workspace.rs` | `277890a509f82f8d…` | `SettingsReview`, `settings_default_preview`, `settings_review`, `settings_apply_review` (ticket namespace shared with schedule reviews; post-write accounting with rollback) |
| `crates/studio-domain/src/session.rs` | `e2a55538dce692e5…` | `settings_default_plan` (NHL 15 tables only; others refused with the reason) |
| `crates/studio-domain/src/pending.rs` | `52ecdb7c44bf044a…` | pending entity labels for `stocksettings` / `stocksettingdefaults` rows (setting name + screen) |
| `crates/studio-domain/src/lib.rs` | `527ccdd153ab1973…` | module + `SettingsReview` export |
| `crates/studio-domain/tests/settings_edit.rs` | `94e4c0aa6d798b19…` | **new** — 9 cases (1 `--ignored`) |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `04177ec3…` / `1b161b7a…` / `88bdfeac…` / `e4233b55…` / `43a43e48…` / `5017f110…` | `SettingsEditPlanDto` / `SettingsEditPreviewDto` / `SettingsEditApplyDto`; commands `settings_default_preview`, `settings_apply_review`; export roots 109 → 111 |
| `app/src/bindings/{SettingsEditPlanDto,SettingsEditPreviewDto,SettingsEditApplyDto}.ts` | generated | `export-bindings --write`; `--check` clean |
| `app/src/lib/client.ts` | `983c058717284824…` | command typings |
| `app/src/components/StoredDefaultEditor.tsx` | `265a5c7a72eea128…` | **new** — cell/code pickers, Preview, held plan, Apply/Discard, refusal/error/applied states, ownership guards |
| `app/src/components/StoredDefaultEditor.test.tsx` | `02f98b1e7f9838ef…` | **new** — 3 jsdom cases |
| `app/src/pages/GameSettings.tsx` | `4b33bf5452fcacde…` | editor embedded for eligible rows of a database source; CSV rows say why editing waits |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | `6a31a1b8597de952…` | 1.2 rules row: NHL 15 stored-default editing impl+tested; 16–21 refused until D4 |

## 3. Behavior and decisions

- **Eligibility on the opened bytes, every time.** `plan_default_edit` reads the catalogue through the
  D2 reader (so the disclosures are identical), then checks: both written tables packed (`max ==
  current`); the id is one of the 21 (an excluded row is refused by name with its reason, an
  out-of-scope row as not offered, an unknown id as absent); the row exists exactly once; `min == 0`
  and `size == max − min + 1` on the opened file (a synthetic `size 4` and a synthetic `min −7` on row
  72 are refused as `shape`); the target cell exists exactly once (a screen without a row → `no-row`
  with the table-growth wording and "says nothing about which default the game would use"; a synthetic
  duplicate `(3, 72)` → `ambiguous`); `0 ≤ code ≤ max`, `code < size`, and the 3-bit global column's cap
  (unreachable on eligible rows, range refuses first). Earlier edits are ordinary baseline: the second
  target on a file already carrying one edit re-validates the same way and passes.
- **Held review.** The plan is stored under `<workspace namespace>/<n>` with the `WorkspaceRevision` at
  preview time (the namespace/sequence are the P1-A ones, so tickets never alias across instances).
  Apply refuses a foreign ticket ("another project"), an unknown/consumed one ("no longer held"), and a
  moved revision ("changed since this preview"), all before any mutation; an undo that returns the bytes
  to the previewed revision makes the ticket valid again (tested). A no-op review (code == current)
  applies nothing and leaves the file clean.
- **The write.** `set_cells_int_rows` on the one cell (the existing audited, undoable path — one undo
  entry). Afterwards the workspace derives the accounting independently of the plan
  (`accounted_diff`: the record span, the field's bit mask, the covering CRC = the next physical table's
  `prior_crc` by block-offset order or the EOF CRC) and runs `verify_single_cell_diff(before, after)`;
  any other difference (a byte elsewhere, a bit of the record outside the field, a touched header CRC,
  a stale covering CRC, a length change — each exercised against a hand-built bad write) rolls the edit
  back with `undo()` and reports it. On the anchor the accounting holds for both tables: the 4-byte
  `stocksettingdefaults` record (mask `00 C0 03 00`, bits 14..18) and the 56-byte `stocksettings` record
  (byte 51 mask `07`); every `header_crc`, the file-header hash and every other `prior_crc`/EOF word is
  asserted byte-identical, the covering word asserted moved.
- **EDIT-MODEL exemption (recorded).** The five catalogue tables declare no `update="True"`; the E1
  `stockleagues.name` rename set the precedent for editing a static non-`update` table under the direct
  editing model; the same deliberate exemption applies here to exactly `stocksettings.default` and
  `stocksettingdefaults.default` of the 21 ids.
- **Pending / review text.** The pending list shows one cell: "Setting PlayoffSeriesLength (id 72) —
  stored default for screen context 3", field `default`, 3 → 2. The plan's description follows §2.6:
  title/copy, setting, screen (lineage name + id), codes with inferred labels, the unchanged stored
  defaults of the same id, the unchanged siblings, and the "has not been established" sentence; notes
  name the other NHL 15 copy and the accounting rule.
- **Copied-target route.** `stage_workspace` + `tdb_txn::prepare/apply` install the edited database
  (expected prior = the original identity), the installed bytes equal the working bytes and pass the
  same accounting against the original, reopen shows the cell, `tdb_txn::restore` returns the exact
  original bytes. No new save/recovery mechanism.
- **UI.** For an eligible row of a database source the detail card carries the editor: cell picker
  (global + the screens that store one), code picker from the row's choices (or a numeric input when
  no labels are bound), Preview → plan (description, table/record, before → after, copy, notes,
  anomaly count) → Apply (ticket only) / Discard. Refusals and apply errors are shown verbatim; an
  apply error drops the held plan (the server no longer holds anything usable). Completions are bound
  to `(project, row)` via generation + current-context refs; a late preview for a replaced row is
  ignored and a late apply for a replaced project pushes no summary. CSV-source rows state that editing
  waits for the packaging route.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test settings_edit -- --include-ignored` | **9 passed** (incl. the patch 01.06 copy: `(3,72)` 3→2 and the patch-only `(9,86)` 3→1 accounted, 82 excluded there too, restore byte-exact) |
| `cargo test -p studio-domain --test settings_edit --test settings_catalogue --test in_place_save --test copied_target --test schedule_edit` | 8 + 12 + 11 + 7 + 6 passed (4 ignored by name) |
| `cargo test -p studio-api --lib bindings` | 25 passed (root count 111) |
| `cargo run -p studio-api --bin export-bindings -- --check` | "Bindings are in sync." |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run` | clean / **14 files, 105 passed** (3 new) |

Not run: full workspace suite, `zz_diag`, clippy/fmt (baseline defects), desktop launch, any game.

## 5. Remaining blockers / limitations

- NHL 16–21 stored-default editing: **blocked on D4** (native-framed chunkzip encoder + text-member
  artifact route + the one-cell CSV splice); the refusal names it.
- Gen7 / PS3 / Recompiled / 07–09: no data-side representation (D2 reason); executable patching is not
  authorized.
- Runtime: whether a creation screen shows an edited stored default, which copy the game reads, and
  where a chosen value is persisted — user card P1BS-1 (morning review).
- The 3-bit global cap is unreachable on eligible rows (max ≤ 6); it is coded, not separately provable
  on stock data.

## 6. Temporary artifacts / processes

- Session scratchpad `d3_wire.py`, `d3_api.py` — disposable.
- Test temp dirs `nhlms-d3-*` under `%TEMP%` — removed by the tests' `Drop`.
- No new fixtures, no new build tree, no processes left.

## 7. Next

D4 — shared packaging (P1-F): native-framed chunkzip encoder with decoded-content preservation and the
text-member artifact route on the copied-target review, then the NHL 16–21 one-cell CSV splice and the
NHL 18–21 paired global edit.

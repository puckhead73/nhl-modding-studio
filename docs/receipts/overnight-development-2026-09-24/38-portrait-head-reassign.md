# Receipt 38 — reassign a player's portrait or head to another shipped asset; NHL 08 / 09 goalie mask paint

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD `8d60a6d`
(unchanged; nothing staged or committed). Claims `dev-portrait-head-reassign`, `dev-audio-admissions-r82-r131` (the
G1 part). Direction: [overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md)
§4 (Players, then equipment). Research consumed: **R124** (2026-09-26 handoff 17 §4, the refusal list), R53 / R63,
R128 (Recompiled), **R125 G1** (handoff 18). **Not in either review bundle.**

## 1. Plain-language summary

- **New:** on a player's asset page, the "Portrait & head" section has **Reassign portrait / head**. Type the
  number of another portrait or head, press **Check**: Studio shows the files in the attached game tree that
  make it valid, or exactly why it is refused, plus what to know (another player already uses it, it is another
  player's own face, the head becomes / stops being "own head", `hasportrait` stays as stored). **Apply** stages
  one ordinary database edit — reviewed under Pending changes, undoable, saved like any edit.
- It only points at assets the title actually ships: a portrait needs its `p<id>` file; a head needs its mesh,
  from NHL 13 its texture library, and from NHL 12 a row in the game's head registry. Numbers the field cannot
  hold are refused.
- Offered on Xbox 360 NHL 09 → Legacy and PS3 NHL 15 / Legacy (loose trees), and PS4 portraits. **Refused by
  name:** PS4 heads (vault records; not implemented), PS3 NHL 08 → 14 (archive-held, not walked), Legacy
  Recompiled (archive-held), NHL 07 / 08 (not measured), and new assets (the head-conversion workstream).
- What the game then draws is not observed (research card P34-1).
- **Goalie masks on NHL 08 / 09** (research observation G1): the mask **paint** number is `maskcolor`, not
  `maskpattern` (which is 0 on every shipped goalie). Both now carry a note saying so; `maskcolor` and
  `hascustommask` are in the goalie-gear regression (edit, review, refusal past range, undo / redo, save / reopen).

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/player_reassign.rs` (**new**), `lib.rs` | `plan_reassign` (R124 refusal list, evidence, disclosures) |
| `crates/studio-api/src/{registry,commands,dto,lib,bindings}.rs`; `app/src/bindings/PlayerReassignPlanDto.ts` (generated) | `player_reassign(project, roots, bio_record, kind, target, apply)` — the plan is re-derived on apply; a refused plan writes nothing; the write is `set_field("ubPc", …)` |
| `app/src/lib/client.ts`, `app/src/components/PlayerContainmentCard.tsx` | `ReassignRow` (portrait / head): Check, evidence / refusals / notes, Apply, success line |
| `crates/studio-domain/src/attributes.rs`, `view.rs` | `goalie_mask_note` (NHL 08 / 09 `maskcolor` / `maskpattern` / `hascustommask`) |
| tests | `studio-domain/tests/player_reassign.rs` (**new**, 3); `app/src/components/PlayerContainmentCard.test.tsx` (+1); `studio-domain/tests/equipment_goalie_gear.rs` (G1 columns and notes) |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `r37-r38-real.log` — `player_reassign` 3 / 3: anchors — NHL 07 refused (not measured), same id refused, a
  20-bit head id refused ("does not fit the 16-bit field"), no tree attached refused, Recompiled refused, PS4 head
  refused. **X360 NHL Legacy loose tree**: portrait 1289 → 15 (an unused shipped `p15.big`) allowed with the file
  as evidence and the `hasportrait` note; a missing portrait refused by file name; applied as one edit, re-read,
  undone to 1289. Heads: an unused registered head (id 6) allowed with 3 pieces of evidence (mesh, texture library,
  registry row); head 11050 (mesh, no `texlib_11050`) refused; head 194 (mesh + texlib, no `stockplayerheads` row)
  refused; taking Paul Stastny's own head disclosed as "own face — two players would share it". **PS4 NHL 21**: a
  shipped `p<id>.gnf` allowed; a missing id refused.
- vitest `PlayerContainmentCard` 5 (the new case: Check → refusal shown, Apply disabled; a valid target → evidence,
  Apply enabled; Apply sends `apply: true` once and shows "staged as one edit … Pending changes"); `tsc` exit 0;
  bindings `--write` (1 file) then `--check` in sync.
- `r38-goalie-g1.log` — `equipment_goalie_gear` 2 / 2 (every anchor + the genuine NHL 20): X360 / PS3 NHL 08
  `maskcolor[0..255]`, NHL 09 `maskcolor[0..511]`, with `hascustommask[0..1]` and `maskpattern[0..255]`, 10 columns
  edited per title, notes asserted.

## 4. Limits

- Data-level contract only (R124): naming consistency, not an observed selection by the game.
- The picker takes a typed number; it does not list the unused pool (the containment summary counts it).
- PS3 Legacy: `patch.big` may carry a different copy of some portraits (C0-AS7) — disclosed, not resolved.
- PS4 head reassignment and PS3 NHL 08 → 14 stay refused until their sources are read.

## 5. Cards

- **P34-1** (game, research's card; a build with receipt 38, on a copy of X360 NHL Legacy): reassign one skater's head to
  an unused registered head and his portrait to an unused portrait; Save; play one game / open the roster: the new
  face and portrait appear (old face / missing texture / generic face = the naming reading is incomplete).
- **V-78** (desktop): the same player → Check a portrait id without a file → the refusal names `p<id>.big`, Apply stays
  disabled; Undo after Apply restores the id.

## 6. Cleanup

No temporary files (the tests write only the in-memory session); scratch scripts deleted.

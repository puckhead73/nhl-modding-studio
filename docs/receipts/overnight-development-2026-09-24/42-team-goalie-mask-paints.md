# Receipt 42 — a team's goalie mask paints on the Uniforms page (Equipment 4.5 G2)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD `8d60a6d`
(unchanged; nothing staged or committed). Claim `dev-team-goalie-masks-g2`. Research consumed: R125 / R125b
(2026-09-26 handoff 18 §1, §3, observation G2).

## 1. Plain-language summary

- **New:** Team → Uniforms → Helmet now shows the team's **goalie mask paints**. There is one row per numbered
  slot, with the starter's and the backup's stored paint number. Each paint gets the same file check a goalie's
  own mask number gets on the Xbox 360 / PS3 loose game trees. The check shows the `goaliehelmet_<shell>_<paint>_…`
  files the number names, or "no files".
- It appears on NHL 10 → PS4 NHL 21. Slot counts: NHL 10 has 6 slots, NHL 11 → Legacy and PS4 NHL 15 have 7,
  and PS4 NHL 16+ have 9. NHL 07–09 have no team slots, so nothing is shown there.
- **PS4:** paints are catalogue records, not files, so they are shown as numbers. The values research found
  shipped outside that catalogue (248 / 249 on NHL 16+) are labelled "shipped exception". They are reported,
  not refused. Checking PS4 mask records waits for its own contract (G3).
- Not claimed: which uniform uses which slot, how a slot combines with the goalie's own mask, or what the game
  draws. No mask-shell names are invented. Editing stays in the team's fields; the card is read-only.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/team_goalie_masks.rs` (**new**) | slot parse (`goaliemask<k>_<role>`), per-paint status (empty / files / no files / no folder / shipped exception / not checked), the `goalie-helmet` paint position (n2) for the file check, R125b exception list |
| `crates/studio-domain/src/lib.rs` | module |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `TeamGoalieMasksDto`, `TeamMaskPaintDto`; `team_goalie_masks` command; export roots 165 → 166 |
| `app/src/bindings/TeamGoalieMasksDto.ts`, `TeamMaskPaintDto.ts` | generated (`export-bindings --write`; `--check` in sync) |
| `app/src/components/TeamGoalieMasksCard.tsx` (+ test) (**new**), `UniformsTab.tsx`, `lib/client.ts` | the card under Helmet |
| `crates/studio-domain/tests/team_goalie_masks.rs` (**new**) | below |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/r42-*`)

- **Anchors** (`slot_shape_per_title_on_the_anchors`):
  - X360 NHL 07 / 08 / 09 have no slots.
  - NHL 10 has 6 slots (2 136 non-zero paints, 218 distinct); NHL 11 has 7; Legacy has 7 (3 401, 230 distinct).
  - PS4 NHL 15 has 7 slots and no exceptions. PS4 NHL 21 has 9 slots, with 4 shipped exceptions (248 / 249).
  - Without a tree, every gen7 paint says "not checked (no folder)".
- **Real X360 trees, read only** (`--ignored`): every non-zero team paint names files on NHL 10, 11, 12, 13, 14,
  15 and Legacy (2 136 / 3 261 / 3 269 / 3 339 / 3 314 / 3 317 / 3 401), with 0 "no files". Each file carries
  the paint at the second position.
  - NHL 12 and 14 go beyond research's measured set (research did not run them) and hold the same way.
- **Genuine NHL 20** (private fixture): 9 slots; 5 717 numbers shown unchecked; 248 × 2 and 249 × 2 reported as
  shipped exceptions; no file status.
- Domain unit test 1 / 1. studio-api `bindings` 25 / 25 (root count 166). Frontend: vitest 263 / 263 (42 files,
  including the 2 new card cases); `tsc` exit 0 (`r42-frontend.log`).

## 4. Limits

- The PS3 NHL 10–14 trees are archive-held: the card says there is no folder to check.
- PS4 has no catalogue check (G3). Runtime selection is unobserved.
- **Process slip:** the claim was written at the checkpoint, not before the first edit.

## 5. Cleanup

Nothing temporary was created; the tests only read files.

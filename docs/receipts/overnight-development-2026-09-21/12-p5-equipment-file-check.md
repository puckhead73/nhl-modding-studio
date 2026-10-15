# Receipt 12 — Priority 5 (Equipment, gen7): the equipment file check on the Player page

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Real Xbox 360 trees read only; committed anchors opened as-is. Claim:
`dev-p5-equipment-file-check`. Research consumed: 2026-09-21 handoff 12
(`research-r38-gen7-equipment`) §1–3 incl. the NHL 11 addendum.

## 1. Plain-language summary

On the old consoles there is no equipment catalogue table: the catalogue is the set of files in
each family's folder, named by brand id. The research measured the file-name grammar on every
title and showed that every id the rosters assign has its files. Studio already knew most of the
grammar (the Hockey Shop slots); tonight it learned the NHL 11 variant (brand ids on `texlib_<id>`
files, sticks under `stick/`), corrected the goalie stick's database column (it was recorded on
the skater table, so goalies' sticks were invisible to per-player checks), and grew a small card
on the Player → Equipment tab that says, per slot, which files the assigned id owns on this
install and how large the title's catalogue is — with an explicit "no files for this id: the game
has no art for this value" when a database value points outside it. Checked against the real
Legacy and NHL 11 trees with the anchors' rosters: 3 296 and 3 099 assigned ids, every one with
files — the research's 100 % containment, reproduced. Nothing is written; reassignment stays an
ordinary cell edit (the check shows the consequence, it does not block).

## 2. What users can now do

- Player → Equipment (any gen7 title with an install tree): the "Art files on this install" card
  per slot — file count, model label, catalogue size and id range, and the missing-art warning.
- NHL 11 installs: gloves / pads / blocker / trapper / sticks resolve through the texture-keyed
  grammar (the Hockey Shop listing benefits too).

## 3. Implementation

| Component | Change |
|---|---|
| `crates/studio-domain/src/equipment_slots.rs` | `texlib_<id>` matchers on gloves / pads / blocker / trapper (NHL 11 addendum); `alternate_folder` (`stick` → `stick/` when `playerstick/` is absent); goalie-stick spec table `fMVl` (the goalie companion's `stick`, studio `wXFx`) instead of `xmja`; `check_player_equipment_files(rendering, is_goalie, fields, enums) → Vec<EquipmentFileCheck>` (slot, field, folder, id, model label, files, catalogue size / range, note: files / no files / no folder / no id); `list_equipment_slot_assets_in`. |
| `crates/studio-api` (`dto.rs`, `registry.rs`, `commands.rs`, `lib.rs`, `bindings.rs`) | `EquipmentFileCheckDto` (files capped at 24 with `file_count`); `Registry::player_equipment_files`; command `player_equipment_files`; bindings exported; root-count guard 125. |
| `app/src/components/EquipmentFilesCard.tsx` (+ test), `PlayerEquipmentTab.tsx`, `lib/client.ts` | the card under the field grid (install tree + companion required). |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `studio-api/tests/equipment_files.rs::every_assigned_equipment_id_has_files_on_legacy_and_nhl11` (`--ignored`, real X360 Legacy + NHL 11 trees, every 7th roster player) | every assigned non-zero id of every checked slot has files (**Legacy 3 296 / 3 296, NHL 11 3 099 / 3 099**); catalogue sizes per slot equal the research where the family definitions coincide (Legacy gloves 56, skates 33, sticks 75, pads 70, blockers 71, trappers 69, masks 280; NHL 11 skates 17, sticks 28, pads 33, goalie sticks 14, masks 243) and the documented Studio-definition values where they differ (Legacy helmets 21 incl. chinstrap / padding ids, goalie sticks 38 = texture-keyed ids; NHL 11 gloves 19 incl. the generic mesh ids 0 / 1, helmets 9); NHL 11 sticks resolved under `stick/` |
| `equipment_slots` unit tests (10) | the existing grammar cases still hold with the added matchers |
| `EquipmentFilesCard.test.tsx` (2 jsdom cases) | args `{projectId, playerId, roots}`; files / no-files / no-folder states and wording; empty → nothing; error as text |

Runs: [p5-equipment-files.log](../../validation/evidence/overnight-development-2026-09-21/logs/p5-equipment-files.log)
(1 passed, 106 s — the final of four runs; the first three failed on **my** catalogue-size
expectations taken from the research's family definitions (helmet 19 vs Studio's 21, goalie
stick 40 vs 38, NHL 11 gloves 17 vs 19), each diagnosed above and recorded as Studio's definition
in the test; containment never failed); `cargo test -p studio-api --lib` 31 passed; `npx tsc`
clean; card test 2 passed.

## 5. Coverage (Phase 4 matrix, "catalogue / listing" row, gen7)

| X360 07 / 08 | X360 09–15 / Legacy | PS3 08–15 / Legacy | PS4 | Recomp |
|---|---|---|---|---|
| impl (grammar unmeasured there — shows "no folder" / counts as found) | impl; **dev-tested on Legacy + NHL 11** | impl (same grammar, `.rpsgl`) — not exercised tonight; PS3 12 / 13 archive-only → "no folder" | separate (vault chain of 2026-09-20 handoff 09) | by identity |

## 6. Open / omissions

- Visor type and mask shell (`helmet/visor n1`, `goaliehelmet n1`) are not slots; the card does
  not check them.
- Reassignment validation (refuse an id outside the title's file set — handoff 12 §3) is shown,
  not enforced; a new-entry registration flow is not built (unverified at runtime by the research).
- PS3 trees not exercised through this check tonight.

## 7. Cleanup

Scratchpad `equip_check.rs` deleted. No temp roots, no processes.

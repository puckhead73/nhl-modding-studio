# Receipt 40 — goalie gear file check: NHL 12 pads read at the measured position; NHL 08 / 09 mask paint checked

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD `8d60a6d`
(unchanged; nothing staged or committed). Claim `dev-goalie-gear-evidence`. Research consumed: R125 / R125b
(2026-09-26 handoff 18 §1, observation G1). **Not in any review bundle** (after the r39 bundle; display only).

## 1. Plain-language summary

- **Defect fixed:** Player → Equipment → "files" for a goalie's **pads** on NHL 12 looked for the pad number in the
  wrong place in the file names (the first number instead of the second), so it found no files or the wrong ones —
  e.g. pads 54 found 0 files instead of 20. NHL 13 and later were right and stay unchanged.
- **New:** on NHL 08 / 09 the mask paint number (`maskcolor`) now gets the same file check as the other gear
  (the files `goaliehelmet_…_<paint>_…` it names, or a warning when the title ships none).
- Still not shown: the mask shell (`masktype`, several candidate file positions — ambiguous), team mask slots (G2),
  the PS4 (`maskid`, `masktype`) pair (G3). Which file the game draws is not claimed.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/equipment_slots.rs` | `EquipmentSlotSpec` is `Clone + Copy`; `for_title` (NHL 12 pads matchers at position 2); `GOALIE_MASK_PAINT_NHL08_09` (third position of `goaliehelmet_*`, found by the `maskcolor` field name); the file check iterates title-specific specs |
| `crates/studio-api/src/registry.rs` | the slot listing (`list_equipment_slot_assets`) uses the title's spec |
| `crates/studio-domain/tests/goalie_gear_file_positions.rs` (**new**) | below |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/r40-goalie-gear-files.log`)

- `goalie_gear_file_positions` 1 / 1 (`--ignored`, real X360 trees): NHL 12 — every pads file of 12 goalies carries the
  stored id at position 2; NHL 13 and Legacy at position 1; the old first-position reading on NHL 12 pads 54 listed
  0 files, the measured one 20; NHL 09 — mask-paint files of 15 goalies carry `maskcolor` at position 3; NHL 10 has no
  paint slot (its paint is `maskid`, the existing Goalie Helmet slot).
- studio-api `equipment_files` 1 / 1 (`--include-ignored`: every assigned equipment id has files on Legacy and NHL 11);
  studio-domain lib `equipment` 24.

## 4. Limits

- The Hockey Shop's game sync still uses the default spec (it receives a display label such as "NHL 12", not the title id); NHL 12 pads extracted there still follow the first-position reading — a follow-up if the Hockey Shop is used on NHL 12.
- Naming evidence only (R125); runtime selection is unobserved everywhere.

## 5. Next

See receipt 39 §10 (updated).

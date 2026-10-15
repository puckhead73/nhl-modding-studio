# Receipt 52 — PS4 skater gear: helmet, gloves, skates, stick and visor numbers checked against each title's own Creation Zone lists, with thumbnails

**Developer-verified; not Codex-accepted; nothing launched or written.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-ps4-skater-gear-catalogue`. Research consumed: 2026-09-28 handoff 02b (R136), on the vault reader of receipt 51. Read-only.

## 1. Plain-language summary

- **PS4 skaters show what their gear numbers are.** Player → Equipment shows a "Gear catalogue (PS4)" card for helmet, gloves, skates,
  stick and visor type. Each number is checked against the lists the game's own Creation Zone uses (NHL 17–21), with a **picture**
  on demand. Numbers used to be bare.
  - **Outside the list:** a number outside the list, such as helmet 99 on 69 genuine NHL 20 skaters, is reported as "not in the
    catalogue". It is not refused.
  - **NHL 15 / 16** ship no lists, so only the catalogue check is shown, with no pictures.
- **Names are shown as keys.** The list's name is a dictionary key (e.g. `$TXT_VISOR_AVIATOR`), shown small beside the number. Studio
  does not invent a brand name, and the dictionary route for these keys is not established.
- **Only the title's own lists.** A list is read only when its file hashes to the recorded value for the open project's title, and the
  vault only when it is the title's own. An NHL 17 project with an NHL 18 install attached is refused.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/ps4_vault.rs` | shared EB helpers `eb_index` / `eb_read` / `eb_member`; `enum_values` (values of a class's enum-typed attributes); `read_install_vault` uses them |
| `crates/studio-domain/src/ps4_skater_gear.rs` (**new**) | families (field, list, thumbnail folder, vault enum); `SkaterGearCatalogue::load` (vault identity, list SHA-256 check, thumbnail index over the install's archives, NHL 15 / 16 enum-only); `check`; `thumbnail_png` (Creation Zone equipment members only); `skater_census` |
| `defs/ps4/skater-gear-csv.json` (**new**) | per title and family: the `inxp_00.big` list member and its SHA-256, and the named enum (from R136's data, provenance recorded) |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `Ps4SkaterGearDto` / item; cached catalogue; `ps4_skater_gear`, `ps4_gear_thumbnail` |
| `app/src/components/Ps4SkaterGearCard.tsx` (+ test), `PlayerEquipmentTab.tsx`, `lib/client.ts`, 2 bindings | the card |
| tests (**new**) `studio-domain/tests/ps4_skater_gear.rs`; `studio-api/tests/ps4_goalie_mask_pair.rs` +1 | below |

## 3. Evidence (real installs, only `databigs/*.big` read; anchors; genuine NHL 20 fixture)

- **`ps4_skater_gear` 1 / 1 (`--ignored`), every figure equal to R136:**
  - **list rows** (helmet / gloves / skates / stick / visor):
    - NHL 17: 12 / 23 / 10 / 30 / 6;
    - NHL 18: 16 / 17 / 9 / 33 / 7;
    - NHL 19: 16 / 24 / 41 / 86 / 7;
    - NHL 20: 15 / 26 / 101 / 161 / 7;
    - NHL 21: 18 / 44 / 123 / 192 / 7.

    Every row id is in its vault enum and has its thumbnail.
  - **thumbnail counts** per family, equal to R136's table:
    - NHL 17: 21 / 64 / 40 / 98 / 7;
    - NHL 18: 25 / 68 / 42 / 106 / 8;
    - NHL 19: 22 / 46 / 54 / 134 / 8;
    - NHL 20: 22 / 48 / 120 / 203 / 8;
    - NHL 21: 26 / 69 / 149 / 240 / 8.
  - **genuine NHL 20 census:** helmet 6 734 list items plus 69 stored 99; gloves, skates, stick and visor all 6 803 in the lists.
  - **named cases:**
    - visor 3 → `$TXT_VISOR_AVIATOR`;
    - helmet 1's thumbnail `nhl/ui/kettle_assets/creationzone/equipment/helmet/helmet1.gnf` decodes to 164 × 164;
    - NHL 15 stick 71 (one anchor row) → not in the catalogue enum;
    - the NHL 18 install for an NHL 17 project → source mismatch.
  - **First run failed (recorded):** NHL 15 has no `inxp_00.big`. The archive is now opened only when the title has lists.
- **API `skater_gear_and_thumbnails_through_the_registry` 1 / 1 (`--ignored`):**
  - 60 genuine NHL 20 skaters, all five families on list items;
  - a thumbnail previews;
  - an archive name with a path, and a non-thumbnail member (the vault file), are refused.
- **Frontend:** vitest `Ps4SkaterGearCard` 2 / 2; `tsc` clean.

## 4. Limits

- **Not claimed:**
  - which item the game draws, or whether it rejects a value outside the list;
  - stick handedness (not derived);
  - team default columns (numbers only: research found their numbering unreliable).
- **Not done:**
  - localized names (the dictionary route is not checked);
  - an edit picker built from the list rows (contract item 3; the existing numeric editor and its range stay; the card re-checks
    pending values).
- **Performance:** the first check reads the vault and every archive directory (≈ 2–3 s, cached per install).

# Receipt 54 — PS4 goalie mask pictures (G3 contract item 6)

**Developer-verified; not Codex-accepted; nothing launched or written.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-ps4-goalie-mask-pairs-g3` (extended). Research consumed: 2026-09-28 handoff 02 §5 / §6.6 (R135).

## 1. Plain-language summary

- **The PS4 "Mask pair" card can show the mask's picture.** A "picture" button opens it for a goalie whose pair is in the catalogue.
  - **NHL 17–21:** the picture comes from the game's own Creation Zone mask list.
  - **Other masks on NHL 19–21:** the list's own numbering rule (helmet type × 1000 + mask) names a picture. That is labelled "a
    candidate, not a list row".
  - **NHL 15 / 16:** no picture. Their picture numbers do not follow the masks.
- The list is read only when it is the open title's own file, by its recorded hash.

## 2. Files

| Path | Change |
|---|---|
| `defs/ps4/goalie-mask-csv.json` (**new**) | per title 17–21: the `goaliemaskinfo.csv` member, its SHA-256 and the `assetid` rule (ordinal on 17 / 18; style × 1000 + pattern on 19–21), from R135's data |
| `crates/studio-domain/src/ps4_skater_gear.rs` | `creationzone_thumbnails` (shared index), `csv_table` |
| `crates/studio-domain/src/ps4_goalie_masks.rs` | `ThumbBasis`; `MaskCatalogue.thumbnails` (list rows, then name-convention candidates on 19–21); `thumbnail(maskid, type)` |
| `crates/studio-api/src/{dto,registry}.rs` | `Ps4GoalieMaskPairDto.thumbnail_archive / _member / _basis` (unique pairs) |
| `app/src/components/Ps4GoalieMaskPairCard.tsx` (+ test) | the picture button and its basis caption (reuses `ps4_gear_thumbnail`) |

## 3. Evidence

- **`ps4_goalie_mask_pairs` 2 / 2 (`--ignored`):**
  - thumbnail counts equal R135 §5: NHL 15 / 16 0, NHL 17 / 18 28, NHL 19 426, NHL 20 443, NHL 21 453;
  - on genuine NHL 20, list row 0 (assetid 119) names `…/creationzone/equipment/mask/mask119.gnf` for (119, 0), and it decodes;
  - at least one name-convention candidate exists;
  - all receipt 51 figures unchanged.
- **API `goalie_pair_and_team_paints_through_the_registry` 1 / 1:** unchanged result.
- **Frontend:** vitest `Ps4GoalieMaskPairCard` 4 / 4; `tsc` clean; the binding was regenerated.

## 4. Limits

- A name-convention picture is a name join, not a list row. Which picture the game shows for a pair is not observed.
- The 9–11 pictures per title that decode to no catalogue pair are not shown.

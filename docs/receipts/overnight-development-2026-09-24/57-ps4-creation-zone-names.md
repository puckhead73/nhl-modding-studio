# Receipt 57 — PS4 gear and mask names in the title's own words

**Developer-verified; not Codex-accepted; nothing launched or written.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-ps4-cz-names`. Research consumed: 2026-09-28 handoff 08 (R141). Closes receipt 52's "names are shown as keys" limit.

## 1. Plain-language summary

- **PS4 NHL 17–21 gear now shows names such as "BAUER 4500", "Bauer Nexus 2N PRO" and "Aviator"** on Player → Equipment →
  "Gear catalogue (PS4)", instead of keys like `$bauer4500`. The key moves into the name's tooltip.
- **The PS4 mask-pair card names the mask** when its pair is a Creation Zone list row, e.g. "BAUER Yellow" or "Cage 2 Black".
  Standard masks fill the title's own "style pattern" template only when both parts resolve; named masks show their own text.
- Every name is the game's own English text. Studio looks keys up the way the game's key hash does, ignoring letter case. About
  one key in five needs that.
- The dictionary is read only when it is the open title's recorded file (by hash). Otherwise the names are hidden, the reason is
  given in a note, and the checks still work. The two placeholder words on newer mask rows show nothing.

## 2. Files

| Path | Change |
|---|---|
| `defs/ps4/cz-dictionary.json` (**new**) | per title 17–21: the archive (NHL 19: `patch_inxp_56.big`, the last patch copy), member, SHA-256, rows and R141's totals |
| `crates/studio-domain/src/ps4_cz_names.rs` (**new**) | `CzNames::load` (identity-checked), `resolve` (`$` stripped; exact key, else the first row whose stored hash = `loc_hash`), `mask_name` (template rule), `name_census` (R141 comparison) |
| `crates/studio-domain/src/ps4_skater_gear.rs` | `GearCheck.name`; catalogue `names` / `names_problem` |
| `crates/studio-domain/src/ps4_goalie_masks.rs` | list-row keys kept with the thumbnail join; `names`, `names_problem`, `name(maskid, type)` |
| `crates/studio-api/src/{dto,registry}.rs` | `Ps4SkaterGearItemDto.name`, `Ps4GoalieMaskPairDto.mask_name`; the dictionary's identity (or why names are hidden) in the notes |
| `app/src/components/Ps4SkaterGearCard.tsx`, `Ps4GoalieMaskPairCard.tsx` (+ tests) | the name shown; the key behind the tooltip when a name exists |
| `app/src/bindings/Ps4SkaterGearItemDto.ts`, `Ps4GoalieMaskPairDto.ts` | regenerated with `export-bindings --write` |

## 3. Evidence

- **`creation_zone_names_resolve_as_r141` (new, `--ignored`, real installs, read only).** Exact / hash / unresolved cells equal
  R141 §1 on every title:
  - NHL 17: 154 / 19 / 0 · NHL 18: 140 / 34 / 0 · NHL 19 (`patch_inxp_56`): 229 / 49 / 20
  - genuine NHL 20: 363 / 61 / 44 · NHL 21: 437 / 64 / 50
  - Only `GoalieMaskPattern` and `GoalieMaskBrandAbbr` are unresolved. NHL 15 has no record.
  - An NHL 19 lookup against the NHL 20 install is refused.
  - On NHL 20, the handoff's examples resolve (`$bauer4500` by hash, `$TXT_VISOR_AVIATOR` exact, "BAUER Yellow"). A template
    with a placeholder part is not filled. 50 mask pairs are named.
- `ps4_skater_gear` 2 / 2 and `ps4_goalie_mask_pairs` 2 / 2 (receipt 51 / 52 figures unchanged; `NHL_STUDIO_FIXTURES` set).
- **API `ps4_goalie_mask_pair` 2 / 2 on genuine NHL 20.**
  - All 300 gear items of 60 skaters are list rows, and every one has a name (no `$`).
  - Named mask pairs never show a key, template or placeholder.
- Frontend: vitest for the two cards 6 / 6; `tsc` clean; bindings `--check` in sync.

## 4. Limits

- English only. Studio has no locale setting, and the other language files were not measured.
- The mask display list's helmet-type labels ("Starting BAUER", …) are resolvable but not shown yet.
- Names come from the lists, not from gameplay. Which item the game draws is still not observed.
- Included in the r59 bundle (after r56).

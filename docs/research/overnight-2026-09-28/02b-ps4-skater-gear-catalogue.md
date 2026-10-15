# Handoff 02b (28 Sep) — PS4 skater gear (helmet, gloves, skates, stick, visor): stored value → Creation Zone row → typed vault enum → thumbnail

Research lane, 28 September 2026, published 05:02 MDT (system clock). Prompt package 3, the skater-equipment edge after the mask
contract ([02](02-ps4-goalie-mask-pair-contract.md)). Claim `research-q3-ps4-goalie-mask-pairs` (same package). Read-only: vault records
decoded, archive listings, five CSV members per title read. Nothing written or launched.

Machine-readable: [`data/r136-ps4-skater-gear-catalogue.json`](data/r136-ps4-skater-gear-catalogue.json). Probe:
`local/runs/overnight-research-2026-09-28/probes/r136_ps4_skater_gear_catalogue.py`. Sources and vault identities are the same as
handoff 02 §1.

## Plain-language summary

Studio shows PS4 skater helmet, glove, skate, stick and visor numbers as bare numbers (developer receipt 27). On NHL 17 and later, each
title ships a small Creation Zone list per item. Each row carries the number, a dictionary key for its name (for example `$bauer4500`)
and a thumbnail picture named by the same number. The game's own item catalogue uses the same numbers under matching type names
(`HelmetBrand`, `GloveBrand`, …). Every stored skater value is in these lists, except helmet 99 on NHL 17–21 and one stick value on
NHL 15. So Studio can show a name key and a thumbnail for each skater's gear without guessing. NHL 15 and 16 have the catalogue but not
the lists, so there is no name or thumbnail for them. Team default columns do not follow the same numbering reliably. Which item the
game draws is not observed.

## 1. Joins per family (NHL 17–21; NHL 15 / 16 in §2)

| Family | DB column (`exhibitionskaterequipment`) | Creation Zone CSV (in `inxp_00.big`) | Vault enum (name-matched) | Thumbnail |
|---|---|---|---|---|
| helmet | `helmetbrand` | `skaterhelmetstyle.csv`: `assetid`, `renderassetid`, `itemname` | `HelmetBrand` on `ge_nhl_skaterhelmettexturepack` (+ geometry `ge_nhl_skaterhelmet`, also carrying `HelmetVisorType`) | `creationzone/equipment/helmet/helmet<assetid>` |
| gloves | `gloves` | `skaterglovestyle.csv`: `assetid`, `renderassetid`, `itemname` | `GloveBrand` on `ge_nhl_skaterglovestexturepack` | `…/gloves/gloves<assetid>` |
| skates | `skatebrand` | `skatedisplayinfo.csv`: `skateassetid`, `renderassetid`, `skatename` | `SkateBrand` on `ge_nhl_skaterskates` | `…/skate/skate<id>` |
| stick | `stick` | `stickdisplayinfo.csv`: `stickassetid`, `renderassetid`, `stickname` | `StickBrand` on `ge_nhl_skatersticktexturepack` (packs also carry `StickHandedness`) | `…/stick/stick<id>` |
| visor | `visortype` | `skatervisortypes.csv`: `assetid`, `renderassetid`, `itemname` | `HelmetVisorType` on `ge_nhl_skaterhelmet` | `…/visor/visor<id>` |

**Row structure.**
- In every CSV row, id = render id.
- Names are `$`-prefixed dictionary keys (`$bauer4500`, `$ccmfitlite`, `$bauernexus2n`, `$straightitech`, `$TXT_VISOR_AVIATOR`, …),
  plus abbreviation / `logoid` columns.

**Visor enum.** `ge_nhl_skatervisortexturepack.VisorColor` covers the same small value range, but it is **not** the match. The
column-name correspondence is `HelmetVisorType`.

| Title | CSV rows (helmet / gloves / skates / stick / visor) | CSV ids in the named enum | CSV ids with a thumbnail | Thumbnails (same order) | Texture-pack FileRefs present |
|---|---|---|---|---|---|
| NHL 17 | 12 / 23 / 10 / 30 / 6 | all | all | 21 / 64 / 40 / 98 / 7 | all (1 026 / 210 / 142 / 108 / 15) |
| NHL 18 | 16 / 17 / 9 / 33 / 7 | all | all | 25 / 68 / 42 / 106 / 8 | all |
| NHL 19 | 16 / 24 / 41 / 86 / 7 | all | all | 22 / 46 / 54 / 134 / 8 | all |
| NHL 20 genuine | 15 / 26 / 101 / 161 / 7 | all | all | 22 / 48 / 120 / 203 / 8 | all |
| NHL 21 | 18 / 44 / 123 / 192 / 7 | all | all | 26 / 69 / 149 / 240 / 8 | all |

Thumbnails outside the CSV exist on every title (e.g. `helmet0`, `helmet2`, `visor999`, `placeholder_*`). They are listed in the data and
carry no catalogue row.

**Skater rows** (stored values in the CSV ids = in the named enum):

| Source | Rows | Helmet | Gloves | Skates | Stick | Visor |
|---|---|---|---|---|---|---|
| NHL 17 | 5 527 | 5 526 (**1 row = 99**) | all | all | all | all |
| NHL 18 | 6 522 | 6 514 (8 × 99) | all | all | all | all |
| NHL 19 base / patch 56 | 6 389 / 7 430 | 6 335 / 7 373 (54 / 57 × 99) | all | all | all | all |
| NHL 20 genuine | 6 803 | 6 734 (69 × 99) | all | all | all | all |
| NHL 21 | 6 498 | 6 408 (90 × 99) | all | all | all | all |

## 2. NHL 15 / 16

- No skater-gear CSVs ship.
- Every stored value is in the named enum, except stick **71** (one NHL 15 row, in both the anchor and patch 00).
- All FileRefs are present.
- Thumbnails exist (NHL 15 helmet 34, gloves 74, …; NHL 16 19, 57, …), but no row joins a number to them. The goalie-mask case (handoff 02
  §5) shows that thumbnail numbers can be ordinals, so NHL 15 / 16 thumbnails stay **unjoined**.
- Names: none.

## 3. Team default columns (`exhibitionteams.helmetbrand / gloves / stick`)

- Mostly 0.
- The non-zero values fall partly inside the skater CSV ids and partly outside:
  - helmet values 22, 24, 31;
  - glove values 54, 83, 94, 106 / 112 on 19–21;
  - many more outside on 15 / 16.
- Team `stick` is 0 on every team (NHL 19–21).

The same numbering is **not** established for team defaults. Show them as numbers.

## 4. Contract for the developer (read-only inspection; NHL 17–21)

1. **Identity.** Bind the title's own `inxp_00.big` CSV member (SHA-256 in the data) and vault. Never borrow another title's list. The
   files differ between titles for every family except the visor list, which is shared from NHL 18 on.
2. **Stored value v.** If v is a CSV id:
   - show "`<family>` `<v>`";
   - show the name from the key through the title's dictionary if the loc route resolves it, otherwise the key behind disclosure, never
     as a brand name Studio makes up;
   - show the thumbnail `…/<dir>/<dir><v>.gnf` through the existing GNF route.

   If v is not a CSV id:
   - helmet 99 → "stored 99; not in the catalogue" (reported, not refused);
   - anything else → the same wording.
3. **Proposed edit.** Offer the CSV rows as the candidate list. A value outside the rows but inside the named enum is a **warning** (it
   has catalogue texture packs but no Creation Zone row). A value outside both is a stronger warning. The existing numeric range still
   bounds what can be written. Game rejection is unobserved.
4. **NHL 15 / 16.** Show the enum membership check only (no names, no thumbnails).
5. **Stick handedness.** Packs exist per `StickHandedness`, but which player field selects the side is **not** established here. Do not
   derive it.

**Minimal fixture cases** (genuine NHL 20 unless noted):
- a skater with helmet `v` ∈ CSV → thumbnail `helmet<v>.gnf` present;
- a helmet-99 skater (69 rows) → reported;
- NHL 15 stick 71 → "not in the catalogue enum";
- `visortype` 3 → CSV key `$TXT_VISOR_AVIATOR`, `HelmetVisorType` 3 present;
- NHL 17 vs NHL 18 helmet CSVs → distinct files (`1753f9d9…` vs `6f998e28…`; identity binding).

## 5. Not established

- Rendered selection.
- Localized strings (dictionary route not checked).
- Handedness source.
- Team default semantics.
- Why helmet 99 exists (compare `masktype` 99 on goalies, handoff 02; no meaning claimed).

## 6. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r136_ps4_skater_gear_catalogue.py <out.json>   (≈ 40 s)
```

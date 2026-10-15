# Handoff 02 (28 Sep) — PS4 goalie mask (`maskid`, `masktype`) read / inspection contract (G3), measured per source incl. genuine NHL 20 and patches

Research lane, 28 September 2026, published 04:56 MDT (system clock). Prompt package 3. Claim `research-q3-ps4-goalie-mask-pairs`.
It extends 27 Sep handoff 18 §2 (R125c) and answers the G3 residual that developer receipts 40 and 42 name. Read-only:
- vault records decoded;
- archive member listings and two CSV members read;
- nothing extracted to disk beyond scratch copies of those CSVs (removed);
- nothing written or launched.

Machine-readable:
- [`data/r135-ps4-goalie-mask-pairs.json`](data/r135-ps4-goalie-mask-pairs.json): per title catalogue / thumbnails / Creation Zone, and per source goalie and slot classes;
- [`data/r135a-ps4-attrib-sources.json`](data/r135a-ps4-attrib-sources.json): which archives carry vault members, with unwrapped hashes.

Probes in `local/runs/overnight-research-2026-09-28/probes/`: `r135_ps4_goalie_mask_pairs.py` (`cf516e82…`), `r135a_ps4_attrib_sources.py`
(`12a67ed0…`), `ps4_member.py` (`bc596d2b…`).

## Plain-language summary

On PS4 a goalie's mask is chosen by two numbers together: the mask design and the helmet type. Every PS4 title's catalogue has at most one
entry for each pair, so a stored pair either names exactly one mask or none. Every catalogue entry's picture files exist in the game's
archives. A handful of shipped goalies have pairs that name nothing: a helmet type of 99 on NHL 19–21, and a few designs on NHL 16 / 17
that are missing or exist only for another helmet. Studio should show these as they are, not refuse them. A team's stored mask paints have
no helmet type of their own, so each can only be listed with the helmet types it exists for. On NHL 17 and later the Creation Zone's mask
list names a thumbnail for each of its masks. On NHL 19 and later its numbers follow a rule that also names most other mask thumbnails.
None of this shows what the game draws, or whether it rejects a pair.

## 1. Sources (distinct identities)

| Title | Vault (`gccollections.vlt` / `.bin`, SHA-256) | Database sources (SHA-256) |
|---|---|---|
| NHL 15 (CUSA00561) | `d1ec595c…` / `0684ef97…` | anchor `ae9d1d56…`; patch 00 `abe36f58…` |
| NHL 16 (CUSA01909) | `202035cf…` / `b1217f4a…` | anchor `7d95f1db…` |
| NHL 17 (CUSA04326) | `df39aeaa…` / `2da87ee8…` | anchor `de48b02b…` |
| NHL 18 (CUSA07569) | `0a5e019d…` / `cf0cfab2…` | anchor `65eaa6e8…` |
| NHL 19 (CUSA11126) | `6e537bb7…` / `aece3f7a…` | anchor (base) `265e4a68…`; patch 56 `61332e66…`. `anchors/ps4/nhl20` has the **same bytes** as the NHL 19 base anchor (alias, not NHL 20). |
| NHL 20 genuine (CUSA15439) | `31a2727f…` / `e5266477…` | install `inxp_00.big` `1e200478…` |
| NHL 21 (CUSA18182) | `dd4ceff0…` / `d668bbe1…` | anchor `90ee1ec4…` |

**Vault provenance (R135a).**
- `gccollections` / `gcschema` ship **only in `boot.big`** on every title. No patch archive carries them, so NHL 19 base and patch 56
  resolve against the same catalogue.
- The `Extracted` copies of those four files equal the `boot.big` members on all seven installs.
- NHL 19's *other* `Extracted` vault files (numeric names) match **no** shipped archive member (dated 20 Jul 2026, larger than every
  shipped version). They are **not** used here, and research treats them as unidentified.

## 2. The catalogue (typed joins)

| Title | Mask texture packs | Distinct (`maskid`, brand) pairs | Pairs with > 1 pack | Typed texture refs per pack | FileRefs present as archive members | Helmet brands (geometry `ge_gameobject` name) |
|---|---|---|---|---|---|---|
| NHL 15 | 506 | 506 | 0 | 4 | 2 024 / 2 024 | 0–7 |
| NHL 16 | 492 | 492 | 0 | 4 | 1 968 / 1 968 | 0–7 |
| NHL 17 | 526 | 526 | 0 | 4 | 2 104 / 2 104 | 0–7 |
| NHL 18 | 578 | 578 | 0 | 4 | 2 312 / 2 312 | 0–7 |
| NHL 19 | 940 | 940 | 0 | 4 | 3 760 / 3 760 | 0–8 |
| NHL 20 | 974 | 974 | 0 | 4 | 3 896 / 3 896 | 0–8 |
| NHL 21 | 991 | 991 | 0 | 4 | 3 964 / 3 964 | 0–8 |

**Pack and file structure.**
- Each pack (`ge_nhl_goaliehelmettexturepack`) carries `GoalieMaskTexture` (= DB `maskid`), `GoalieHelmetBrand` (= DB `masktype`) and a
  `DebugName`.
- Each pack also has four typed `ge_texture` references:
  - the per-design colour map, `rendering\characters\player\goaliehelmet_<brand>_<design>_cm`;
  - three brand-shared maps (`…_nm`, `…_am`, `…_coeff`).
- A FileRef resolves to archive member `nhl/data/kettle/<FileRef>.gnf` (checked for every ref above). Colour maps sit mainly in the
  large game archive (NHL 20 / 21 `game_90.big`, NHL 19 `game_90-008.big`, NHL 15 `game30.big`), with a few in `inxp_*` and patch archives.

**Geometry and brands.**
- Geometry is keyed by brand alone: `ge_nhl_goaliehelmet` → `ge_gameobject` named `goalie_helmet_itech`, `…_hasek`, `…_cooper`,
  `…_dryden`, `…_liut`, `…_bromley`, `itech_certifiedcage` (brand 6), `goalie_helmet_brodeur` (7), and from NHL 19
  `goalie_helmet_itech_toquepompom` (8). These are **internal asset names**, not display labels.
- Throat protector records (`OnOff` = 1) exist for brands {0, 2} on NHL 15, {0, 1, 2, 6} on NHL 16–18 and {0, 1, 2, 6, 8} on NHL 19–21.

**Handedness.** Mask packs carry none. Handedness belongs to sticks, blockers and trappers (handoff 18 §1).

## 3. Goalie rows by source

| Source | Goalie rows | Unique pack | `masktype` not a catalogue brand | `maskid` only under other brands | `maskid` absent from the catalogue |
|---|---|---|---|---|---|
| NHL 15 anchor / patch 00 | 559 / 559 | 559 / 559 | 0 | 0 | 0 |
| NHL 16 | 566 | 556 | 0 | 3 (e.g. row 78: `maskid` 77, type 0) | 7 (`maskid` 173, 198, 208, 261; e.g. row 0: 198) |
| NHL 17 | 601 | 596 | 0 | 0 | 5 (all `maskid` 208) |
| NHL 18 | 714 | 714 | 0 | 0 | 0 |
| NHL 19 base | 747 | 743 | 4 (type **99**, `maskid` 0: rows 93, 648, 687, 703) | 0 | 0 |
| NHL 19 patch 56 | **849** (+102 rows vs base) | 845 | 4 (rows 95, 661, 696, 712) | 0 | 0 |
| NHL 20 genuine | 773 | 768 | 5 (type 99, `maskid` 0: rows 88, 559, 592, 605, 611) | 0 | 0 |
| NHL 21 | 749 | 744 | 5 (rows 75, 467, 491, 503, 509) | 0 | 0 |

Consistent with R125c's counts. The type-99 rows are now classed as "not a catalogue brand" rather than "another brand only". Rows are
`exhibitiongoalieequipment` record positions; that table carries no `playerid` column.

## 4. Team mask slots (G2 context — a slot has no helmet type)

| Source | Slot columns | Non-zero uses | Value exists for one brand | For several brands | No record |
|---|---|---|---|---|---|
| NHL 15 / patch 00 | 14 (slots 0–6) | 3 317 | 390 | 2 927 | 0 |
| NHL 16 | 14 (slots 0–6) | 3 401 | 396 | 2 995 | 10 (173, 198, 208, 261, **248**, **249**) |
| NHL 17 | 18 (0–8; 7–8 all zero) | 3 511 | 406 | 3 099 | 6 (208, 248, 249) |
| NHL 18 | 18 | 4 065 | 489 | 3 572 | 4 (248, 249) |
| NHL 19 base / patch 56 | 18 | 5 763 / 5 816 | 22 / 26 | 5 737 / 5 786 | 4 (248, 249) |
| NHL 20 genuine | 18 | 5 721 | 22 | 5 695 | 4 (248, 249) |
| NHL 21 | 18 | 5 959 | 26 | 5 929 | 4 (248, 249) |

- 248 / 249 are the two teams with `artabbr` PRSA / PRSB (abbreviations TPR / TPW), slot 0, starter and backup.
- 198 on NHL 16 is `LBC` slot 0. The same design exists from NHL 20 as `goaliehelmet_itech_bili_tygri_liberec_0`.
- **Correction to developer receipt 42's wording** ("PS4 NHL 16+ have 9 slots"): NHL 15 / 16 declare slots 0–6; NHL 17–21 declare
  0–8, with 7–8 zero on every team. The shipped module parses slot names generically (`team_goalie_masks.rs` header: "PS4 0–8 with 7–8
  all zero on 18–21"), so no display defect is claimed. Only the receipt count is off for NHL 16.

## 5. Creation Zone catalogue and thumbnails

| Title | `creation_zone_csv/goaliemaskinfo.csv` (in `inxp_00.big`) | Rows | `assetid` rule | Rows whose (pattern, style) is a catalogue pair | Rows with `mask<assetid>` thumbnail | Thumbnails (`…/creationzone/equipment/mask/mask<N>`) |
|---|---|---|---|---|---|---|
| NHL 15 | absent | — | — | — | — | 89 `.png` (`nhl/ui/assets/…`) |
| NHL 16 | absent | — | — | — | — | 90 |
| NHL 17 | `b7224fed…` | 28 | **ordinal** (3, 4, 5, …), not style × 1000 + pattern | 28 / 28 | 28 / 28 | 90 |
| NHL 18 | `b7224fed…` (same bytes as 17) | 28 | ordinal | 28 / 28 | 28 / 28 | 90 |
| NHL 19 | `0573c36d…` | 38 | **`assetid = stylerenderassetid × 1000 + patternrenderassetid`** 38 / 38 | 38 / 38 | 38 / 38 | 437 + 437 `_xl` |
| NHL 20 genuine | `c2d54977…` | 50 | same rule 50 / 50 | 50 / 50 | 50 / 50 | 452 + 452 `_xl` |
| NHL 21 | `c4920c36…` | 53 | same rule 53 / 53 | 53 / 53 | 53 / 53 | 462 + 462 `_xl` |

**Column meanings.**
- `patternrenderassetid` = `maskid`, `stylerenderassetid` = helmet brand.
- `goaliemaskdisplayinfo.csv` (NHL 17+) has 8 rows: `MaskPrimary{Itech, Hasek, Cooper, CHL}` and `MaskBackup…` for `maskstyle`
  0 / 1 / 2 / 6. These are title-owned display **keys** for four brand values.
- The CSV `stylename` / `patternname` / `name` cells are `$`-prefixed dictionary keys (`$bauer`, `$cage`, `$yellow`,
  `$TXT_STANDARD_MASK_DESC`). Some NHL 19+ rows carry a number in `stylename` and `$GoalieMaskBrandAbbr` in `styleabbreviation`
  (recorded as shipped, not interpreted).

**Thumbnail joins.**
- On NHL 19–21, 426 / 443 / 453 of the thumbnails decode by the same arithmetic (N = brand × 1000 + `maskid`) to a catalogue pair. That
  is a **name join consistent with the CSV rule**, beyond the CSV's own rows.
- 9–11 thumbnails do not decode that way: 61, 114, 115, 216–220 and 999, plus 368 / 369 on NHL 19.
- On NHL 15–18 the numbers are ordinals (proved by the NHL 17 / 18 CSV). Do **not** decode NHL 15 / 16 thumbnails; they have no
  established join.

## 6. Read / inspection contract (for the developer; neutral, numbered)

1. **Identity first.** Bind the title's vault by the `boot.big` member hashes (§1) and the database by its SHA-256. Refuse the catalogue
   check (keep showing numbers) if the vault is absent or unreadable. Never borrow another title's vault.
2. **Pair lookup** (`maskid` → `GoalieMaskTexture`, `masktype` → `GoalieHelmetBrand`). The result is one of:
   - **unique** — show "Mask `<maskid>` · helmet type `<masktype>`". Behind disclosure: the pack `DebugName`, the colour-map FileRef with
     its archive and member, and the brand geometry name.
   - **not a catalogue brand** (99) — "stored helmet type 99; no catalogue helmet".
   - **only under other brands** — "this design exists for helmet types {…}".
   - **absent** — "no catalogue record".
   - **ambiguous** — never measured. Keep the class: list all packs and pick none.

   The last three are **reported, never refused**, for stored values.
3. **Proposed edits.** Check the value fits the declared field width (the existing numeric range). Then check catalogue availability and
   show the result as a **compatibility warning**. A pair with no catalogue record is not proven to be rejected by the game.
4. **Team slots.** Show "paint `<v>` (exists for helmet types {…})" or "no record" (248 / 249, etc.). Do not choose a brand. How slots
   combine with a goalie's `masktype` / `maskid` is runtime (card G-MASK-1).
5. **Labels.** No brand names as primary text:
   - resolve the display keys (`MaskPrimaryItech`, …) and the `$…` CSV keys through the title's own dictionary if the loc route has them,
     otherwise show "helmet type k";
   - keep geometry names behind disclosure.
6. **Thumbnails** (optional preview):
   - NHL 17 / 18: CSV row → `mask<assetid>`.
   - NHL 19–21: CSV row → `mask<assetid>`; other packs → `mask<brand×1000+maskid>` **labelled as a name-convention candidate**.
   - NHL 15 / 16: no join.
   - No thumbnail → preview the colour map through the existing GNF route, or show none.
7. **Handedness.** Not applicable to masks. The throat protector per brand is a candidate only.

**Minimal fixture cases** (genuine NHL 20 unless noted):

| Case | Input | Expected |
|---|---|---|
| unique | row 1: `maskid` 125, type 0 | pack `goaliehelmet_itech_generic_8_texpack`; colour map `…_generic_8_cm` present |
| type 99 | row 88: `maskid` 0, type 99 | reported "not a catalogue brand"; no refusal |
| absent | NHL 16 row 0: `maskid` 198, type 0 | reported "no catalogue record" (the design exists from NHL 20) |
| other brand only | NHL 16 row 78: `maskid` 77, type 0 | reported with the brands that carry 77 |
| slot, no record | team `index` 216 (PRSA) slot 0: 248 | reported, not refused |
| stale source | NHL 19 patch 56 database with the NHL 20 vault | refused as a source mismatch |
| thumbnail | NHL 20 CSV row 0 (`assetid` 119 = 0 × 1000 + 119) | `creationzone/equipment/mask/mask119.gnf` |

## 7. Not established

- Which pack the game draws.
- Whether it rejects a missing pair.
- How team slots and goalie values combine.
- What brand 99 means.
- Whether the Creation Zone list limits user choice to its rows.
- What the NHL 15 / 16 thumbnail numbers map to.
- Localized strings for the display keys (dictionary route not checked here).

## 8. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r135a_ps4_attrib_sources.py <out.json>    (≈ 45 s first run)
python -B local/runs/overnight-research-2026-09-28/probes/r135_ps4_goalie_mask_pairs.py <out.json>  (≈ 65 s)
```

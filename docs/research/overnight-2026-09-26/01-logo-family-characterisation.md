# Handoff R107 (P1) — the `rendering/logo/texlib_{x}_{y}_{z}` family characterised neutrally on X360 / PS3 Legacy and NHL 15: every member holds exactly three rasters (`logo_{x}_{y}_{z}_0` 256×128, `_1` 128×64, `_2` 128×128, DXT5), a shape shared by no kit or centre-ice raster; 339–348 ids, 127–132 of them without kit files; the member tuples overlap the kit file tuples without containing them; no typed consumer string found in 604–766 readable loose files per tree — purpose not assignable statically; competing hypotheses and one bounded user card

Research lane, 26 September 2026, 01:08 → 01:10 MDT (real clock). Review: [research-20260926-r102-r106-review.md](../../validation/research-20260926-r102-r106-review.md)
package 1. Claim `research-p1-logo-family-characterisation`. Read-only over the loose trees of X360 / PS3 Legacy and NHL 15 (loose =
unwrapped member on the R104 samples for these trees; no precedence inferred). Descriptor layouts from the developer's reader
(`crates/tdb-rx2/src/texlib.rs`, `texlib_ps3.rs`), read, not edited. Nothing game-verified; no visual acceptance performed.

Machine-readable: [`data/r107-logo-family-characterisation.json`](data/r107-logo-family-characterisation.json) (probe
`local/runs/overnight-research-2026-09-26/probes/r107_logo_family_characterisation.py`).

## Plain-language summary

The files long called "rink logos" were opened and their pictures measured without assuming what they are. Every one of them, on both
consoles and both titles, holds the same three pictures: a wide one (256 by 128), a small wide one (128 by 64) and a small square one
(128 by 128). No jersey, pant, sock, nameplate or centre-ice file has pictures of those shapes. The family covers more teams than have
kits (about 130 extra art ids) and overlaps the kit combinations without covering all of them. Nothing in the game's readable data files
names this family, so what the three pictures are used for cannot be decided from the files. Three readings remain possible; one short
visual check by the user would narrow them.

## 1. Raster shapes per family (all members; `data/r107-…json` `families.*.roles`)

| Family | X360 Legacy members | rasters per member | named rasters (role → dimensions, format) | PS3 (same members) |
|---|---|---|---|---|
| **logo** `texlib_{x}_{y}_{z}` | 1 926 (NHL 15: 1 784) | **3 on every member** | `logo_{x}_{y}_{z}_0` 256×128 DXT5; `_1` 128×64 DXT5; `_2` 128×128 DXT5 | 1 928 (1 786); same dims, DXT5, mips 8 / 7 / 8 |
| jersey `texlib_{s}_{a}_{v}` | 905 (683) | 6 (901), 7 (3), 1 (1) | `jersey_…_cm` 1024² DXT1, `_sm` 1024² DXT1, `jersey_…_{k}_nm` 1024² fmt 0x71; `font_…_cm` 512² DXT5, `_nm` 512² 0x71, `_sm` 512² DXT1 | same dims; PS3 signatures DXT5 / `83f1` |
| nameplate `name_{s}_{a}_{v}_cm` | 904 (683) | 1 | 256² ARGB8 | 256² ARGB8, 9 mips |
| pant | 959 (684) | 3 (931), 5 (28) | `pant_…_cm` 512² DXT1, `_nm` 512² 0x71, `_sm` 256² DXT1 (+ `_rm` / `_roughness` 4×8 on 28) | same dims |
| sock | 946 (683) | 1 | `sock_…_cm` 1024×512 DXT1 | same |
| centre ice `centerlogo_{id}_cm` | 231 (226) | 1 | 1024² DXT3 | 1024², signature `83f2` |

PS3 signatures `0x83F1` and `0x83F2` are not in the developer reader's documented pair (`0x83F3` DXT5, `0x6007/0x8035` ARGB8). On the same
members X360 reports DXT1 and DXT3 respectively, so `83f1` ↔ DXT1 and `83f2` ↔ DXT3 is a **cross-platform inference, not a decode**.
X360 format byte `0x71` (normal-map rasters) is recorded raw.

## 2. Identities and tuple overlap

| | X360 / PS3 Legacy | X360 / PS3 NHL 15 |
|---|---|---|
| logo ids / kit ids / centre-ice ids | 348 / 222 / 231 | 339 / 213 / 226 |
| logo ids without kit files · kit ids without logo files | 127 · 1 | 132 · 6 |
| logo ids that are also centre-ice ids | 212 | 202 |
| logo tuples ∩ jersey tuples (common / jersey-only / logo-only) | 796 / 109 / 1 130 (PS3: 1 132) | 567 / 116 / 1 217 (PS3: 1 219) |
| ∩ pant · ∩ sock · ∩ nameplate (common / kit-only / logo-only) | 840 / 119 / 1 086 · 829 / 117 / 1 097 · 796 / 108 / 1 130 | 568 / 116 / 1 216 · 567 / 116 / 1 217 · 567 / 116 / 1 217 |

The logo family is **larger than and overlapping with** the kit families; it is **not** a superset (109–119 kit tuples per family have no
logo member on Legacy X360) and **not** a full grid. The (y, z) occupancy is in the JSON (`logo_bc_grid`).

## 3. Consumer search (scoped negative)

Readable loose files under each tree root (< 8 MB; texture / mesh / audio / archive containers excluded): 740 (X360 Legacy), 604 (PS3
Legacy), 766 (X360 NHL 15), 629 (PS3 NHL 15), searched for `rendering/logo`, `logo/texlib`, `texlib_%`, `logo_%`, `%d_%d_%d` (and the
backslash forms) — **0 hits**. Executables (encrypted) and archive members were not searched. The loader resolves this family somewhere
not visible to this search.

## 4. Readings (hypotheses — none established)

- **H1 — kit decals** (shoulder patches / helmet or pant logos): the (y, z) values overlap the kit (style, variant) space, and three small
  rasters per member fit a decal set. Against it: 127–132 ids have logo files but no kit files, and some kit tuples lack a logo member.
- **H2 — rink-board / arena logos per team** (the inherited label): three fixed aspect ratios suit board and banner placements. Against
  it: the (y, z) indices track kit style / variant rather than arenas; centre-ice art is a separate family.
- **H3 — front-end / broadcast graphics** (scoreboard / score-bug logos): fixed small sizes suit UI use. Against it: FE crests exist
  separately (`t{id}.big`), and nothing in the FE data names these files.

Descriptor shape alone cannot establish runtime purpose; no static discriminator remains without a consumer reference.

## 5. User card L-LOGO-1 (bounded; visual review of shipped assets only — no game run needed)

- **Observation:** in Studio's texture preview (or any RW4 viewer), open `rendering/logo/texlib_{id}_0_0` and `texlib_{id}_1_0` for one
  NHL club with kits (e.g. an id from the kit set) and for one id **without** kit files (e.g. 32, 33, 77, 79 on Legacy). For each, note what
  rasters `_0` (256×128), `_1` (128×64) and `_2` (128×128) depict.
- **Outcome:** shoulder / helmet / pant decal artwork → H1; board / banner / ice artwork → H2; scoreboard or menu artwork → H3; a mix →
  the family serves several consumers and each raster slot needs its own label. The card establishes depicted content only, not the
  consumer or selection rule.

## 6. Developer-facing wording

List the family neutrally as "team logo set (3 rasters: 256×128, 128×64, 128×128)", keyed by the art id in position 1; show positions 2–3
raw; do not call it "rink logo" and do not offer a kit-linked edit until L-LOGO-1 or a consumer reference resolves the purpose.

```
python local/runs/overnight-research-2026-09-26/probes/r107_logo_family_characterisation.py docs/research/overnight-2026-09-26/data/r107-logo-family-characterisation.json
```
Executed × 3: the first run read the PS3 signature at the wrong halfword (all zeros); the second had a line-joining edit error (exit 1);
the third is the published JSON.

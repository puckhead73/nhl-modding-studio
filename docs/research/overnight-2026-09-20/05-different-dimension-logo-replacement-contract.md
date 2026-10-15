# Handoff R12 — different-dimension logo replacement: every dimension-bearing word per family, the display-quad vs texture distinction, and the bounded contract

Research lane, 20 September 2026 (Priority 1D of the overnight campaign). Claim `research-r12-dimension-contract`. Consumer: developer
**P1-E "Replace with different dimensions (Apt geometry record) — not impl"** on every family, P1-F placement (handoff 03), 1.4 league
imagery. Builds on handoffs [16](../overnight-2026-09-18/16-league-logo-packaging-cross-platform.md) §2 (the 288-byte record), [14](../overnight-2026-09-18/14-phase1-league-logo-assets.md),
[18](../overnight-2026-09-18/18-rw4ps3-texlib-layout.md), [03](../overnight-2026-09-19/03-non-logo-league-imagery.md) (registries). Nothing here is
game-verified; nothing written; no launch. **No writer is proposed.** Measured in the session (read-only) on the 2026-09-19 survey
packages; no new data file — the measurements below are the complete result and are reproducible with the command in §5.

## Plain-language summary

Replacing a league logo with an image of a different size is not a texture-only change: inside each gen7 package two more members carry
the logo's on-screen size as floating-point numbers — the 288-byte geometry record (six words) and the Apt object in member `0` (two
words) — and handoff 16 had only named the first. Measured across the families, those floats are the *display quad* size, not the
texture size: NHL 07 draws a 100×100 quad from a 128×128 texture, so a texture may already differ from its quad. That gives the
developer a clean split: (1) changing the *texture's* pixel dimensions touches only the texture container's own header words (which
Studio's readers already decode per platform) and the package/archive placement (handoff 03); (2) changing the *displayed* size
touches the eight quad floats; (3) what the UI does with a texture whose size no longer matches its quad — stretch, crop, or ignore —
is a runtime question, not a static one. On PS4 there is no geometry record at all: the registry picks a directory (`size64`,
`size256`, `size512`) by name, the GNF header carries the real dimensions, and the same runtime question remains.

## 1. Dimension-bearing words, measured per family (observed on the index-0 packages of `leaguelogos` / `leaguelogosbig` / `leaguelogossmall`)

| Family | Texture member | Header words that differ between 64×64 and 256×256 / 64×64 and 128×64 | Geometry record `<n+1>` (288 B) | Apt Data member `0` |
|---|---|---|---|---|
| X-C (Xbox 12–Legacy), `XPR2` | word 8 = data size (0x4000 → 0x40000); word 72 (packed dims/format: `80800002` → `82000002` / `81000002`); word 76 = format (`0x86` ARGB / `0x54` DXT5); word 80 (packed size: `0007e03f` → `001fe0ff` / `0007e07f`); word 84; DXT5 carries `aaaaaaaa` mip-fill words at 108/124/140/156 | words 25, 40, 48, 56, 57, 65 (byte offsets 100, 160, 192, 224, 228, 260): `[25]`=H, `[40]`=W, `[48]`=W, `[56]`=W, `[57]`=H, `[65]`=H — 64.0 / 256.0 / (128.0, 64.0); word 13 (offset 52) = `<n>` − 1 (object index, **not** a dimension) | two floats W, H at bytes 156/160 (256-byte variant: `leaguelogos`) or 140/144 (240-byte variant: `big`, `small`, `wide`); identify them by value (equal to the record's W/H), not by offset — the two variants differ structurally in 50 other words |
| P-C (PS3 12–Legacy), `DDS ` | standard DDS header: height/width (12/16), pitch-or-linear-size (20), flags (8), pixel-format block (80–104: `DXT5` fourcc vs RGBA masks); DXT5 mip-fill words at 140/156 | same six words, same values | same |
| X-A NHL 10 (`RW4xb2`, raw) | **no header word in the first 160 B differs** between 64×64 ARGB (16 384 B) and 128×64 DXT5 (16 384 B incl. mips); 64 vs 256: word 84 = data size only — dims/format live in the inner texlib descriptor that `tdb-rx2::texlib` decodes (not re-derived here) | same six words | same |
| X-A NHL 07 | 100×104 tiled ARGB in a 128² slot (handoff 05) | six words = **100.0** while the texture is 128 wide → the record is the **display quad**, not the texture size | W = H = 100.0 |
| PS4 15–21 (`GNF` / PNG on 15) | GNF header (dims, mips, pitch, format — `tdb-rx2::gnf`); the registry (`definition.kettle.assetpaths`) maps `PARTS/small_league_logo` → `size64/l#`, `PARTS/league_logo` + `collection_league_logo` → `size256/l#`, `PARTS/large_league_logo` → `size512/l#` — directory names, no dimension fields | none (no Apt package) | none |

The 288-byte record and member `0` are byte-identical between Xbox and PS3 of the same title/family (X-C vs P-C on Legacy), so the
geometry is platform-independent; the texture container is the only platform-specific part.

## 2. Contract (proposal; policy, not evidence)

Operation **T — texture dimension change, display size kept**: inputs = new pixel data W′×H′ (power of two on gen7 as every stock
texture is; NHL 07's 100×104-in-128 shows non-square content in a square slot is authored, not arbitrary), same format family. Rewrite:
the texture container completely (header + base mip + the mip chain the container had — DXT5 `leaguelogossmall` ships mips; ARGB
families ship one level: keep the level count of the source); the package via handoff 03's re-plan; the outer archive via handoff
02/03. **Do not** touch the six record words or the two member-`0` floats. Validation: decode round trip equals the intended pixels; the
container reader reports W′×H′ and the source's level count; member count/names/order unchanged; the 288-byte record byte-identical.
Runtime card R12-DIM-1: a 128² logo in a 64² slot — same on-screen size = the UI scales to the quad (this title/screen); a doubled
on-screen size = the UI reads the texture's dims; missing/black = rejected (no cause identified without the same-size control).

Operation **Q — display size change**: rewrite the six record floats and the two member-`0` floats to W″/H″ (IEEE big-endian), texture
unchanged. Everything else preserved. Runtime: the UI's layout may position the quad by other constants (not measured — the Apt object
carries ~60 further words per variant that were not interpreted); a size change can overlap neighbours. Card R12-DIM-2 pairs Q with T.

Refusals: non-power-of-two on gen7 (unobserved); a format change (ARGB ↔ DXT5) — the format word set differs and the DXT5 mip chain
is required (no witness of a format swap); PS4 `size*` directory mismatch (a 512² image under `size64` is not refused by any static
rule — flag it); any package whose member `0` W/H floats cannot be located by value (they must equal the record's).

## 3. League → imagery bindings (status, unchanged)

Gen7: `l<id>` package index = the league code → art-index table traced per title (coverage matrix 1.4 row 1, research 10/14); the
`divisionlogos`/`trophies` id mapping stays in game code (Q-R3-1). PS4: `l#` keyed by the DB `stockleagues` art id natively; the French
twin (`[loc FRE_FR]`) and the shared-span aliases (region logos = league logos, `l100` = `l5`) are as in handoffs 02/03 of 2026-09-19.
No new binding was found or needed for the dimension question; no contact sheets were generated (nothing new to review).

## 4. Unknowns (explicit)

1. UI behaviour when texture dims ≠ quad (cards R12-DIM-1/2). 2. The other ~60 words of the Apt object (position, anchoring). 3. The
inner RW4xb2 descriptor words for dims on NHL 08–10 (Studio's reader decodes them; not re-listed here). 4. PS4: whether the UI
element scales to the image or to a layout constant.

## 5. Reproduce

Ad-hoc in the session (recorded here in full): for the packages `(nhllegacy, x360|ps3)` and `(nhl10, x360)` of the 2026-09-19 survey,
decode the package (RefPack where wrapped), split members, and diff the first 160 header bytes of the texture member, the 288-byte
record and member `0` between `leaguelogos/l0`, `leaguelogosbig/l0` and `leaguelogossmall/l0`; dump the record's 72 words and member
`0`'s float-like words for the seven packages listed in §1. Script kept in `probes/r12_dimension_words.py` (≈ 5 s).

# Handoff R32 / R33 — Gen7 texture descriptors as functions of the dimensions: RW4ps3 (PS3 08–Legacy) and RW4xb2 / Xenos (Xbox 360 09–Legacy), censused on every loose `rendering/**` texture

Research lane, 21 September 2026, Priority 1.3 ("RW4, RW4ps3 … dimension / mip descriptors"). Claims `research-r32-r33-gen7-descriptors`.
Read-only over the loose `rendering` trees on X:; nothing game-verified; no writer. The developer's C6 receipt refused resizes on these
families because "the descriptor is not regenerated"; this handoff measures which descriptor words depend on the dimensions and gives
the rule that reproduces every shipped descriptor (or the exact exceptions).

Machine-readable: [`data/r33-rw4ps3-descriptor-census.json`](data/r33-rw4ps3-descriptor-census.json) (9 PS3 trees, 102 669 descriptors)
and [`data/r32-rw4xb2-descriptor-census.json`](data/r32-rw4xb2-descriptor-census.json) (10 Xbox trees). Probes
`r33_rw4ps3_descriptor_census.py` (≈ 25 min) and `r32_rw4xb2_descriptor_census.py` (≈ 1 h).

> **Correction R1 (coordinator review of 2026-09-21, applied 2026-09-21 16:58 MDT by the research lane; campaign
> [overnight-2026-09-21-pm](../overnight-2026-09-21-pm/00-ATLAS.md)).** The plain-language summary and §2 below overstated the `DW8`
> finding. What was measured: `DW8` bits 12+ equals **2 × the descriptor's ordinal** on 26 118 / 28 745 Legacy, 22 650 / 25 275 NHL 15,
> 20 057 / 22 129 NHL 11 and 15 985 / 17 680 NHL 09 descriptors (`data/r32b-rw4xb2-slot-and-mip-rule.json`); the 2 627 / 2 625 / 2 072 /
> 1 695 remaining descriptors do **not** follow it and their generator was *not* established by that census. The sentence "the exceptions
> are containers whose descriptor order differs from the data-slot order" was an inference from two example files, not a demonstrated join.
> Read the field as follows until a join is demonstrated: (a) descriptor order, data-slot identity and byte offset are **distinct** things;
> (b) an edit that preserves the texture's identity, its container's file-table entries and their order **keeps `DW8` byte-for-byte**
> (this is what the developer's P2-B resize does and it is unaffected by the correction); (c) adding, removing or reordering slots or
> file-table entries has **no** `DW8` rule here and must be refused. The r32b figures came from a scratch script that was deleted; the
> runnable replacement is `local/runs/overnight-research-2026-09-21-pm/probes/r42_rw4xb2_slot_join.py`, which restates rule (a) and tests
> a candidate join (`DW8` upper = file-table index of the texture's pixel-data entry) on every tree 07–Legacy — its result is published in
> the pm campaign's handoff 02, not assumed here. The same probe re-tests the pitch rule in the developer's tile-aligned form
> (`roundup(w, 128) / 32` for BCn) against this handoff's `ceil(w / 32)`; the developer's form is the one to consume where they differ.
> The NHL 07 statement "6 / 7 275 descriptors obey the `DW12` rule" was produced by the r32 counter that subtracts `DW8` and is
> therefore **unreliable for NHL 07** (where `DW8` upper is 1 on every descriptor); it is re-measured by r42 (pm handoff 02).
> The text below is preserved as written.
> **Resolution (2026-09-21 pm, 17:33 MDT; [pm handoff 02](../overnight-2026-09-21-pm/02-rw4xb2-slot-join-and-resize-bounds.md)):** the join is demonstrated — `DW8` bits 12+ = the **file-table index of the descriptor's paired pixel-data entry**, 100 % on NHL 08–Legacy (14 066 … 29 171 descriptors per title) and 10 666 / 10 675 on NHL 07 (9 null-format records); "2 × index" is the special case of tables that start with the first data entry. NHL 07 obeys the `DW12` rule (9 554 / 9 555); NHL 08 is RefPack-wrapped `RW4xb2` (110 jersey home / away libraries undecodable). The pitch rule is the tile-aligned form (0 misses on tiled descriptors). The preserve-only contract stands for edits that keep the file table.

## Plain-language summary

On PS3 a texture's 56-byte descriptor holds only two dimension-dependent words — the packed width/height word and the mip count byte — and
the pixel slot's length is exactly the linear mip-chain size; everything else is constant per format or a slot index. A PS3 resize is
therefore a descriptor edit of two words plus a re-sized linear slot (cube maps excepted). On Xbox 360 the descriptor is the GPU's fetch
constant: the width/height word, the pitch (`ceil(width/32)`), the mip-count fields, and the mip-chain start — the base mip padded to
whole 32 × 32-element tiles (128 × 128 texels for DXT), divided by 4 096 — are the dimension-dependent fields, and the word Studio treats
as a byte address is a slot reference (2 × the texture's index in the container). The rules reproduce every normal descriptor of NHL 09
through Legacy (one 16-wide strip and tiny textures excepted); NHL 07 places mips differently and NHL 08 uses another container. So the
Xbox resize refusal can be lifted for 09–Legacy for textures of at least 128 × 128 texels, with the tiny-texture and 16-wide cases refused.

## 1. `RW4ps3` (PS3; `\x89RW4ps3` containers, descriptor 0x38 = 14 big-endian words)

| Word | Value | Dependence |
|---|---|---|
| 0 | 0, 2, 4, 6 … (up to 152 distinct on 14+) | slot / index inside the container — not a dimension |
| 1–5, 11 | 0 | constant |
| 6 | `0x8` (52 %), `0x808` (43 %), `0x408` (ARGB8 only: 940 / 941), `0x1008` / `0x1408` (72 = the cube maps) | flag bits: `0x400` ↔ ARGB8, `0x1000` ↔ cube, `0x800` unknown (present on ~45 % of DXT textures; preserve) |
| 7 | `0x00000de1` (2 values on 11+) | constant-ish (preserve) |
| 8 = 9 | format: `0x83f1` DXT1, `0x83f2` DXT3, `0x83f3` DXT5, `0x6007` ARGB8 (6007 / 83f3 verified by Studio; f1 / f2 by chain-size agreement below) | format |
| 10 | `0xffff` (DXT) / `0x8035` (ARGB8) | format |
| **12** | `width << 16 │ height` | **dimensions** |
| **13** | `mips << 24` | **mip count** |

Slot length = Σ over levels of `max(block bytes, ⌈w/4⌉·⌈h/4⌉·block bytes)` (DXT; 8 B DXT1, 16 B DXT3 / 5) or `w·h·4` (ARGB8): **28 681 / 28 753**
on Legacy, 25 210 / 25 282 on 15, 28 191 / 28 263 on 14, 20 245 / 20 250 on 11, 10 / 10, 10 / 10, 8 / 8, 44 / 44, 49 / 49 on 08 / 09 / 10 / 12 / 13;
every failure is a cube map (slot = 6 × the chain; word 6 bit `0x1000`) or, on NHL 11, an ARGB8 128² with a 6× slot (cube). No tiling, no
swizzle, no pitch word → a resize on PS3 = rewrite word 12 and word 13, re-emit the linear chain at the new size, keep every other word.

## 2. `RW4xb2` (Xbox 360; the 56-byte descriptor = 7 constant / colour-key words + the 6-dword Xenos fetch constant `DW7..DW12` + `DW13`)

Field split per the public Xenos fetch-constant layout (checked against Studio's independently verified width / height / tiled / pitch
readings). Census: 09 17 680, 10 16 443, 11 22 129, 12 27 074, 13 25 508, 14 28 069, 15 25 275, Legacy 28 745 descriptors (NHL 07: 7 275
descriptors but 3 356 files whose table lies beyond the 256 KiB read window and a different mip placement — not covered; NHL 08: a different
container, 0 `RW4xb2`).

| Field | Rule | Evidence |
|---|---|---|
| `DW0 DW1 DW5 DW6` | constants `0x00400003 0x01000000 0xffff0000 0xffff0000` (`DW0` = 3 on NHL 07 / part of 09) | every descriptor 09–Legacy |
| `DW7` `pitch` (bits 22–30, 32-texel units) | `ceil(width / 32)` | 28 058 / 28 745 Legacy, 24 594 / 25 275 NHL 15 … (the misses are non-tiled / special widths; listed per shape in the JSON) |
| `DW7` `tiled` | 1 on every DXT / DXN texture; 0 on a few FE 8_8_8_8 | as Studio |
| `DW8` low bits | endian 1 + format (`0x12` DXT1, `0x13` DXT3, `0x14` DXT5, `0x31` DXN, `0x06` 8_8_8_8 — stored as the byte `0x52 / 53 / 54 / 71 / 86`) | as Studio |
| **`DW8` bits 12+ ("base address")** | *(bounded by correction R1 above — a majority observation, not a generator)* **`2 × descriptor index`** in the container (0, 2, 4, 6 …), *not* a byte address: 26 118 / 28 745 on Legacy, 20 057 / 22 129 on NHL 11, 15 985 / 17 680 on NHL 09; the exceptions are containers whose descriptor order differs from the data-slot order (`stadium/ice_intl_container_0.rx2`, `indicator/*`) | `r32b-rw4xb2-slot-and-mip-rule.json` — **corrects Studio's `data_off >> 4` formula**, which reproduces the field only for slot 0 (`data_off = 0`), the only case its fixtures covered |
| `DW9` | `(height − 1) << 13 │ (width − 1)` | as Studio (13 + 13 bits) |
| `DW10` | `0x00000d10`-family constants (filters / swizzle) | per shape in the JSON |
| `DW11` | `mip_max_level` bits 6–9 = mips − 1 (`0x1c0` = 7 → 8 mips, `0x240` = 9 → 10, `0x280` = 10 → 11), other bits 0 | every descriptor |
| **`DW12` bits 12+ ("mip address")** | **`tiled base-mip bytes / 4096`**, where the tiled base = the base mip padded to whole 32 × 32-**element** tiles (128 × 128 texels for BCn, 32 × 32 for 8_8_8_8) × element bytes; independent of the slot | 24 820 / 24 821 normal 2D textures with mips on Legacy (the 1 miss: `ice/texlib_0.rx2` 16 × 256), 20 120 / 20 121 on NHL 11, 16 064 / 16 065 on NHL 09; **cube maps** (`dimension` 3): 6 × that; single-mip textures: 0; tiny textures (≤ 8 × 8 blocks): 0 in 75 / 202 cases (packed into the base tile — a resize should refuse below 128 texels on BCn) |
| `DW13` | `0x08 │ 0x00 │ mip count │ 0x00` | 28 743 / 28 745 Legacy, 16 443 / 16 443 NHL 10 … (byte 2 = mips, not `log2 + 1`: single-mip textures carry 1) |

Consequence: an Xbox 360 resize of a BCn texture ≥ 128 × 128 (and any 8_8_8_8 ≥ 32 × 32) on NHL 09–Legacy needs `DW7` pitch, `DW9`, `DW11`,
`DW12`, `DW13` recomputed by the rules above, `DW8` left as is (the slot reference does not change), the tiled base + mip chain re-emitted
(Studio's `tile_dxt` / `untile_dxt`), and the data-slot lengths in the file table updated — with `DW12`'s tiny-texture and 16-wide
exceptions refused. NHL 07 stays refused (its mip placement is not the same: 6 / 7 275 descriptors obey the rule).

## 3. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r33_rw4ps3_descriptor_census.py docs/research/overnight-2026-09-21/data/r33-rw4ps3-descriptor-census.json   (≈ 25 min)
python local/runs/overnight-research-2026-09-21/probes/r32_rw4xb2_descriptor_census.py docs/research/overnight-2026-09-21/data/r32-rw4xb2-descriptor-census.json (≈ 1 h)
```
The slot / mip rule census (`r32b-rw4xb2-slot-and-mip-rule.json`) was produced by a scratch script over Legacy / 11 / 09 / 15 whose logic is
stated in §2 (**restored as a runnable probe 2026-09-21 pm:** `r42_rw4xb2_slot_join.py`, correction R1 above) (DW8 == 2 × index; DW12 × 4096 == tiled base, 6 × for cubes, 0 for single-mip); r32's own `mip_addr_eq_base_plus_tiled_base`
counter subtracted the DW8 field and therefore under-counts on multi-slot containers — read §2, not that counter.

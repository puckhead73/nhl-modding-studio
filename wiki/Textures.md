# Textures

**Evidence:** measured: full header censuses of every PS4 title's GNF files (seven titles, 49 309 to 149 613 files each), every Xbox 360 title's RenderWare texture descriptors (ten titles) and nine PS3 trees. Mip placement inside Xbox 360 slots is a candidate rule; in-game acceptance of a resized texture is open on every platform.

Three texture containers cover the series: RenderWare texture libraries on Xbox 360 (`RW4xb2`) and PS3 (`RW4ps3`), XPR2 and DDS inside front-end packages, and GNF on PS4. Portraits, logos, kits, equipment and arena art are all one of these; the [Archives and Compression](Archives-and-Compression) page covers the wrappers around them.

## Xbox 360 RenderWare texlib (`RW4xb2`)

**Container.** Magic `\x89RW4xb2`; a file table pairing data entries (tag `00 01 00 31`) with 56-byte descriptor entries (`00 02 00 03`). The descriptor carries the six-word Xenos fetch constant (DW7 to DW12) plus DW13. DXT blocks are stored **16-bit byte-swapped and tiled**.

| Descriptor field | Rule (measured on every loose texture of every title) |
|---|---|
| DW0, DW1, DW5, DW6 | constants `0x00400003 0x01000000 0xffff0000 0xffff0000` (DW0 = 3 on NHL 07 and 08) |
| DW7 pitch (bits 22 to 30, units of 32 texels) | tiled: `roundup(w,128)/32` for block formats, `roundup(w,32)/32` for 8_8_8_8; the only misses are untiled 32×32 8_8_8_8 rasters on NHL 13 to Legacy |
| DW8 low bits | format: `0x52` DXT1, `0x53` DXT3, `0x54` DXT5, `0x71` DXN, `0x86` 8_8_8_8 |
| DW8 bits 12 and up | the **file-table index of the descriptor's pixel-data entry** (10 666 of 10 675 on NHL 07, exact on every later title). Not a byte address and not "2 × ordinal". |
| DW9 | `(height−1) << 13 \| (width−1)` |
| DW11 bits 6 to 9 | mips − 1 |
| DW12 bits 12 and up | tiled base-mip bytes ÷ 4 096 for 2D rasters of 128 texels or more on block formats (cube maps 6×; single level 0); tiny textures are mixed, so refuse below 128 texels |
| DW13 | `08 | 00 | mip count | 00`; two stock descriptors per title on NHL 13 to Legacy declare an impossible 16 mips on 128² textures, so never trust it as a bound |

Any edit that keeps the file table's entries and order preserves DW8; inserting, removing or reordering entries would have to rewrite it and is refused. NHL 07's file table sits at the end of the file with untagged payload entries. NHL 08's loose `.rx2` files are RefPack streams wrapping the same layout, except **110 jersey home and away libraries** that do not decode as RefPack after the first literal run and are refused (the archive copies are identical, so this is the shipped form).

Shipped maxima: width 2 048, height 4 096 (NHL 13 to Legacy LCD strips), up to 13 real mips, slot up to 5 603 328 bytes. Each shipped shape has a single slot length on every title, always a multiple of 4 096 (512² DXT1 with 10 mips = 196 608; 1 024² DXT1 with 11 mips = 720 896); an unshipped shape has no oracle.

**Mip placement** (candidate rule, inferred): on 359 classifiable levels across 25 shapes, every level sat where the "own tile for large levels, packed tail for small ones" rule predicts, with DW12 equal to the level-1 tile base on all 103 records. Packed tails, levels below 4×4 blocks, 16-wide strips, DXN and cube maps remain unresolved, so the rule is not a write contract, and keeping a donor's mip tail is not a safe fallback. Studio therefore writes Xbox 360 mipped rasters as a disclosed partial replacement and refuses to resize them.

**ARGB8 front-end rasters** (NHL 08 to 11, 6 926 of 6 926): DW2 = `(w−1) | (h−1) << 13`, pitch = align(w, 32), payload = pitch × align(h, 32) × 4.

## PS3 RenderWare texlib (`RW4ps3`)

Magic `\x89RW4ps3\0\r\n\x1a\n`; big-endian header words give the file count (0x20), the file-table offset (0x30) and the pixel-region start (0x44). File-table entries are 24 bytes (data slot `00 01 00 30`, descriptor `00 02 00 03`, name table `00 EC 00 10`). The 56-byte descriptor (14 words) holds the slot index, flags (0x400 ARGB8, 0x1000 cube), the format in words 8 and 9 (`0x83f3` DXT5 and `0x6007` ARGB8 decoded; `0x83f1` DXT1 and `0x83f2` DXT3 inferred from cross-platform agreement), **word 12 = `width << 16 | height`** and **word 13 = `mips << 24`**.

Pixels are **linear**: row-major, no swizzle, no byte swap, ARGB8 in A, R, G, B order, DXT mip chains concatenated base-first with a 16-byte floor per level. All 36 league logos decode pixel-identical to the Xbox 360 decode of the same art. The slot length equals the linear chain size on every non-cube raster (28 681 of 28 753 on Legacy). A PS3 resize rewrites words 12 and 13 and re-emits the chain; the 16-bit size field accepts far more than the hardware's documented limit, so a bounds check is representability only.

## PS4 GNF

Header: `GNF ` at 0, `contentsSize` 0xF8, version 2, the eight-word GNM texture descriptor at 0x10, pixels at 0x100. Fields: `data_format = (dword1 >> 20) & 0x3F`, `width−1 = dword2 & 0x3FFF`, `height−1 = (dword2 >> 14) & 0x3FFF`, `tiling_index = (dword3 >> 20) & 0x1F`, `last_level = (dword3 >> 16) & 0xF`, `type = (dword3 >> 28) & 0xF` (9 is 2D, 11 cube). Formats: `0x23` BC1, `0x24` BC2, `0x25` BC3, `0x26` BC4, `0x27` BC5, `0x28` BC6H, `0x29` BC7, `0x0a` 8_8_8_8; the SRGB number format dominates.

**Tiling** is index 13 on every file: 1D micro-tiling with 8×8-block tiles (32×32 texels) stored row-major, blocks inside a tile in the GCN non-displayable Z-order, and no byte swap. Row-major order inside a tile leaves a visible seam every 32 texels.

Header rules censused on the full population: `streamSize` equals the file size (single-stream files); the data-size word equals file size − 0x100; `base_level` 0; `last_level` = mips − 1; **pitch = roundup(width, 32) − 1** for every block format including BC6H; data size = Σ over levels of `ceil(ceil(w/4)/8) · ceil(ceil(h/4)/8) · 64 · block bytes` (8 for BC1 and BC4, 16 otherwise) with no further padding; cube maps hold 6× (single level) or 8× (mipped) the 2D prediction. The 16-byte tail after the descriptor is zeros, a `USER` tag or `04 00 00 00` (meaning open). **Multi-stream files** carry a second `GNF ` header at `streamSize`: 1 115 on NHL 15 and 5 342 on NHL 19, none elsewhere; only the first stream has been checked and decoded.

| Title | Files | Readable | Pass the size rule | BC6H | Largest shipped |
|---|---|---|---|---|---|
| NHL 15 | 49 309 | 48 309 | 48 287 of 48 287 | 0 | 4 096², 13 levels |
| NHL 16 | 68 849 | 68 083 | 68 060 of 68 060 | 0 | 4 096², 13 |
| NHL 17 | 83 572 | 82 667 | 82 641 of 82 641 | 0 | 4 096², 13 |
| NHL 18 | 105 551 | 104 369 | 104 342 of 104 342 | 0 | 8 192×4 096, 14 |
| NHL 19 | 124 292 | 122 900 | 121 585 of 121 585 | 1 212 | 8 192×4 096, 14 |
| NHL 20 | 140 142 | 138 740 | 135 774 of 135 774 | 2 821 | 8 192×4 096, 14 |
| NHL 21 | 149 613 | 148 066 | 145 136 of 145 136 | 2 722 | 8 192×4 096, 14 |

Decode status: BC1, BC2, BC3 and BC7 exact; 512-pixel BC5 normal maps drift down the image because the small-BC5 padding rule is open; BC6H refused. A resize recomputes width, height, `last_level`, pitch and the two size words, re-emits the micro-tiled chain and keeps every other word; cube maps, BC6H, depth textures and multi-stream files are refused. Same-size replacement regenerates the whole mip chain. Automated-tested; runtime acceptance open.

## XPR2 and DDS front-end textures

Xbox 360 NHL 12 to Legacy front-end art (league logos, portraits) is XPR2 inside RefPack: 64×64 and 256×256 8_8_8_8, 128×64 DXT5 with mips. PS3 NHL 12 to Legacy uses DDS inside RefPack: 64² and 256² 32-bit RGBA with no mips, 128×64 DXT5. The packages around them are described on [Logos and League Imagery](Logos-and-League-Imagery).

## Per-family chains on NHL Legacy (Xbox 360)

| Family | Chain | Studio status |
|---|---|---|
| Portraits | BIGF → RefPack → XPR2 | read and write on extracted files |
| Logos | texlib `RW4xb2`, DXT5 | lossless round trip plus edit re-decode |
| Uniforms | loose `RW4xb2` rasters (jersey, pant, sock) | as logos |
| Equipment | loose `RW4xb2` rasters per slot | identification partial (brand-keyed) |
| Heads | player mesh plus texlib | read verified; write rebuild only through the NHL 24 to Legacy round trip |

Every row is `game_verified = not_run`.

Last reviewed: 2026-10-20

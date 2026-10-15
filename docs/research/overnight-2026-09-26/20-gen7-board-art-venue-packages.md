# Handoff R127 (P23) — Presentation 5.2 boards on gen7: board-ad, power-ring, jumbotron and glass art live inside each arena's own venue package; the material → raster edge is not decoded, so no raster can yet be named "the board ad"

> **Update 2026-09-27 06:28 (§6):** the material → raster edge **is** decoded at the data level — the u32 at +256 of the `0xEF0001` chunk before each material is the raster descriptor's chunk index (slot 0; +288 = slot 1). The title and §2–§4 negatives describe the state before §6.

Research lane, 27 September 2026, published 05:27 MDT (system clock). Prompt
[`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) package 6. Claim `research-p23-gen7-board-art`.
PS4 boards are 2026-09-20 handoffs 11 / 12 (R18 / R19: decal arrays on shared vault records) and are not repeated. Read-only over the
Xbox 360 loose trees of NHL 10, 13, 15 and Legacy; Studio's `arena_assets.rs` read (it already joins `arenaartid` → `stadium_<id>_container_<n>`
on 100 % of rows, NHL 09 → Legacy).

Machine-readable: [`data/r127-gen7-stadium-board-rasters.json`](data/r127-gen7-stadium-board-rasters.json).

## Plain-language summary

On the older consoles the rink boards, the ring of lights around the bowl, the scoreboard and the glass are drawn from one large file per
arena (11–14 MB), not from a shared folder. Inside each file the material names show which parts are board advertising, and the file
carries 37–111 pictures. What could not be established tonight is which picture each material uses, so Studio cannot yet offer "replace
this arena's board ads". What it could offer now is a read-only gallery of an arena's pictures; a person looking at them can tell which
ones are the ads.

## 1. Measured (per tree; NHL 13 / 15 / Legacy nearly identical, NHL 10 an older layout)

| Fact | NHL 10 | NHL 13 | NHL 15 | Legacy |
|---|---|---|---|---|
| venue packages `rendering/stadium/stadium_<id>_container_<n>.rx2` | 44 | 61 | 64 | 64 |
| embedded rasters per package (`0x20003` descriptors) | 32–42 | 10–111 (mostly 38–43) | 10–111 | 10–111 |
| common raster sizes (X360 DXT1 / 3 / 5) | 256², 256×512, 512² … | 512² DXT1, 256² DXT1 / DXT5, 256×512 … | same | same |
| name dictionary (`0xEC0010`, count word = flag byte + 24-bit count) | names **materials** (`0xEF0005`) and geometry groups (`0xEB0000` / `0xEB0008`), **not rasters** | same | same | same |
| board-ad material names | `adbd_#` (71), `BoardAd_plast#` (44) | `BoardAd_plastic_mat#` (92), `adbd_#` / `adbd_<ABBR>` (81) | 96 / 86 | 96 / 86 |
| their shader files | `env_simple.FX`, `env_boards.FX` | `env_boards.FX`, `env_adbd_alpha.FX`, `env_adbd.FX` | same | same |
| other presentation materials | glass (`RinkGlassSides_…`), jumbotron, power ring | power ring `powerring_<city><n>` (`env_powerring.FX`), jumbotron (`jumbodigits.FX` for the stats digits), glass (`env_glass.FX`), bench ads | same | same |
| loose shared board file | `rendering/adboard/adboard_trans_rgb` (one file) | same | same | same |

Arena-specific names occur inside the packages (e.g. `adbd_WPG`, `jumbotron_WPG`, `powerring_atlanta1` in `stadium_1`, `powerring_boston3`
in `stadium_2`, `powerring_ottawa1` in `stadium_20`), so board / ring art is **per venue package**; other material names repeat across
packages (≈ 500 of ≈ 1 070 distinct names on 13 → Legacy) — a shared *name* is not shared *bytes* (payloads were not compared).
Shader names in the JSON sometimes carry one stray leading letter (a length byte read as text) — noise, not a different shader.

## 2. Negative scope (bounded; two approaches)

1. The name dictionary: it names materials and geometry groups only; rasters are unnamed on these packages (the kit-member dictionaries
   of R104 name rasters — a different package class).
2. Material chunks (`0xEF0005`): no word is a raster chunk index (the value 2 found in every material is a constant, not a reference);
   the binding presumably sits in the parameter chunks (`0xEF0000` / `0xEF0001`, 64 each) — **not decoded**. Stopped here under the
   investigation budget.

## 3. Developer-ready and not

| Operation | Status | Condition |
|---|---|---|
| list an arena's venue package and its rasters (index, w × h, format) with previews | **ready for a read-only listing** — the package is already joined per arena; X360 RW4xb2 descriptors decode as R107 documents; label rasters neutrally ("raster k — 512 × 512 DXT1") | no name claim; material names behind disclosure as "materials in this package" |
| replace a board-ad raster | **contract missing** — which raster the ad material uses is unknown; an in-place same-size write inside a multi-raster, multi-material RW4 package is not an existing route (the kit route writes single-raster texlibs) | material → raster decode or user identification, then a same-size in-place design with byte-exact restore |
| PS3 / NHL 09 / 11 / 12 / 14 venue packages | not measured here | — |

**Next discriminating step (either):** decode `0xEF0000` / `0xEF0001` on one package to recover material → raster; or a user card on the
read-only gallery once it exists (identify the board-ad rasters of one arena visually).

## 4. Addendum 05:42 — the two checkpoint hypotheses (both negative; B13 stops here)

On X360 Legacy `stadium_1_container_0.rx2`: (1) the RW4 manifest has **0 subreferences** and one external arena (its own id `0x07b07661`),
so no object-to-object pointer table exists to resolve material → raster; `0xEF0000` begins with five `(offset << 20 | index)`-like
words at a 0x140 stride (0x140 / 0x280 / 0x3C0 / 0x4D0 / 0x5F0 with indices 1–5 — the `0xEF0001` block size is 320 = 0x140), i.e. a
parameter-block table, not a raster list; (2) a per-material ordinal into the `0x20003` list is impossible (64 named materials, 39 rasters).
The binding needs the shader parameter layout (or the executable). Recorded as the negative scope of B13; the read-only raster gallery
(§3) remains the practical next step.

## 5. Addendum 06:25 — PS3 NHL 15 / Legacy venue packages ([`data/r127b-ps3-stadium-board-rasters.json`](data/r127b-ps3-stadium-board-rasters.json))

The PS3 loose trees ship the same 64 `stadium_<id>_container_<n>.rpsgl` packages with the same per-package raster counts, the same
material-name classes (board ad `BoardAd_plastic_mat#` 96, `adbd_#` 86; glass, jumbotron, power ring) and the same shader files as the X360
NHL 15 / Legacy packages; rasters are `RW4ps3` descriptors (signatures `83f1` / `83f2` / DXT5). The material → raster gap of §2 / §4 is
therefore the same on PS3.

## 6. Addendum 06:27 — the material → raster edge, decoded at the data level (R127c, [`data/r127c-material-texture-slots.json`](data/r127c-material-texture-slots.json)); supersedes the negative of §2 / §4 for this edge

A bounded test of the §4 lead compared every word of each material's chunk triple with raster descriptor / payload chunk indices and
offsets. The only systematic hits are in the **`0xEF0001` chunk immediately before each named `0xEF0005` material** (file-table order
`0xEF0000`, `0xEF0001`, `0xEF0005` per material):

- **u32 at +256 = the chunk index of a `0x20003` raster descriptor** (texture slot 0) on 4 628 of 4 728 materials across the 64 X360
  Legacy packages (NHL 13: 4 448 / 4 545 of 61; NHL 10: 2 165 / 2 169 of 44). The exceptions store `0x400000` (97 on Legacy — the
  power-ring / emissive materials; their texture comes another way) or another value (3).
- **u32 at +288 = a second raster** (slot 1) on about 40 materials per package (e.g. the DXT5 second map of `BoardAd_plastic_mat`); on
  single-texture shaders the word is another value.
- **Every board-ad material resolves** (`adbd_*`, `BoardAd_plastic_mat*`, 64 / 64 Legacy packages): slot 0 rasters are 512 × 512 (130),
  512 × 1024 (97), 1 024 × 256 (11), 256 × 512 (3), 512 × 128, 256 × 256 on Legacy; similar on NHL 13 / 10. Examples (`stadium_1`):
  `adbd_WPG` → raster chunk 44 = 512 × 1024 DXT1; `BoardAd_plastic_mat1` → chunk 38 = 512 × 512 DXT1 (+ slot 1 DXT5); `jumbotron_WPG` →
  512 × 256 DXT1; `jumbotronstats_0` → 2 048 × 128 DXT3 (digit strip).

Class: parsed data (a stable field / index relation on every package), not an observed runtime selection. Consequences: a developer can now
**name the board-ad rasters of an arena** (material name → slot 0 raster) and list / preview them; replacement still needs an in-place,
same-size raster write inside the multi-raster package (the same class as the kit route's texlib write, but inside an 11–14 MB venue
package — design and restore contract not established) and the usual in-game check. PS3 `.rpsgl` packages: not re-run for this edge.

## 7. Addendum 06:29 — PS3 check and the per-arena venue-art list (R127f, [`data/r127f-venue-art-slot-list.json`](data/r127f-venue-art-slot-list.json))

The §6 rule holds unchanged on the PS3 NHL 15 / Legacy `.rpsgl` packages (slot 0 = raster on 4 625 / 4 628 materials; `0x400000` on 99 / 97;
board-ad raster sizes identical to X360). The data file lists, per venue package of X360 NHL 10 / 13 / 15 / Legacy and PS3 NHL 15 / Legacy
(360 packages, 4 116 materials), every board-ad, bench-ad, jumbotron, glass and power-ring material with its slot-0 / slot-1 raster
(descriptor chunk index, ordinal, size, format) or the raw slot word — the input for a read-only per-arena venue-art gallery keyed by
`arenaartid` (Studio's existing arena → package join).

## 8. Addendum 06:31 — writer-contract inputs for a same-size venue raster (R127g, [`data/r127g-venue-raster-write-inputs.json`](data/r127g-venue-raster-write-inputs.json))

Measured on X360 NHL Legacy `stadium_1`, `stadium_2`, `stadium_20` (read-only):

| Input | Result |
|---|---|
| Header | `hdr_size + data_size` = file length on all three; `+0x1c` = the package's arena id (`0x07b07661`, `0x07b07662`, `0x0b0766b0` — per-package ids, not content checksums) |
| Raster → pixel bytes | every descriptor's **DW8** pixel-entry index (`u32 @ +32 >> 12`, Studio's `descriptor_data_entry_index`) addresses a `0x10031` entry large enough for its base level (39 / 39, 41 / 41, 41 / 41) |
| Other data entries | 302 / 312 / 324 of the `0x10031` entries are **not** rasters (geometry and other buffers) — so the texlib parser's "pair data and descriptor entries in order" rule **fails** here (it refuses with an orphan-data error); the pairing must use DW8 |
| Checksums | no CRC-32 / Adler-32 of the data region or of the whole file appears among the header words (**bounded negative, 28 Sep checkpoint 00 K4:** three X360 Legacy packages, header words 0 .. file table only; other algorithms, ranges and fields untested — not "no checksum"; preserve unknown words) |

Contract input (not a writer): a same-size, same-format replacement of one raster rewrites only the bytes of its DW8-addressed data entry
(pixel region = `hdr_size` + entry offset, tiled and 16-bit-pair byte-swapped exactly as Studio's loose texlib route writes them, mips as that
route regenerates them); no file-table, descriptor or size field changes; restore = byte-exact copy of the package. Open: which copy the
console loads (loose `rendering/stadium` vs any archive copy — precedence unobserved), and the in-game result (card).

## 9. Addendum 06:32 — archive copies of the venue packages (read-only sample)

The render archive `cacherender.big` holds every venue package too: X360 NHL Legacy 67 members (`chunklzx`), PS3 NHL Legacy 67 (`chunkzip`),
X360 NHL 13 64 (`chunklzx`) under `rendering/stadium/`. On a sample of six members per tree (the three ice packages and `stadium_0`, `_10`,
`_11`) the unwrapped member is **byte-identical** to the loose file. A same-size raster write therefore has two copies to consider (loose and
archive); which one the console loads is the existing runtime question (the archive route's `chunklzx` store acceptance is card R30-LZX-1;
PS3 `chunkzip` is the encoder Studio already writes). Not every member was compared.

## 10. Correction 2026-09-28 06:03 — slot words must lie inside their parameter block; 361 packages (developer observation 49)

Developer observation [49-R127f-slot1-past-block](../../receipts/overnight-development-2026-09-24/observations/49-R127f-slot1-past-block.md),
re-measured by research on X360 and PS3 Legacy (same distribution on both platforms, not PS3 only). The `0xEF0001` block before each
material is 320 bytes (2 983), **288 bytes (1 581)**, 352 (159), 240 (3) or 384 (2) long. The §6 / §7 slot rule reads u32 at +256
(slot 0) and +288 (slot 1).
- On 288-byte blocks, the "slot 1" word at +288 was read from the **next chunk**.
- On 240-byte blocks, slot 0 was too.

So a slot word exists only when the block reaches it (≥ 260 bytes for slot 0, ≥ 292 for slot 1). R127f's `raw` slot-1 values on short
blocks (e.g. `0x4638` on PS3 Legacy `adbd_ANA`) are not slot data, and the §7 slot-1 counts include them. None of them was a raster, so no
listed image changes. §7's "360 packages" is **361** (44 + 61 + 64 × 4), as the data file lists. Studio's venue reader (receipt 49)
already bounds-checks.

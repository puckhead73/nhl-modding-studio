# PS4 GNF texture container (`.gnf`)

The texture format for the PS4 "Kettle" asset stack (NHL 15 → 21) — player-head
maps, hair, logos, stadium art. The single largest asset class (≈49k files on
NHL 15). Each `.gnf` wraps one GNM (GCN/Liverpool) texture.

Status: **decode verified** for BC1/BC2/BC3 and **BC7** (`crates/tdb-rx2/src/gnf.rs`)
— a real Ovechkin head `cm` map and a real NHL 16 BC7 player portrait both
reconstruct exactly (seamless, no tile drift). BC4/BC5 (normal/roughness) decode is
a known follow-up (see [Open items](#open-items)). All values little-endian.

## Header (verified on real NHL 15 files)

```
0x00  "GNF "   magic (47 4E 46 20)
0x04  u32      contentsSize = 0xF8
0x08  u8       version = 2
0x09  u8       alignment
...
0x10  GNM T# (Tsharp) descriptor — 8 × u32 LE
0x100 tiled BCn pixel data  (= contentsSize 0xF8 + 8)
```

## T# (Tsharp) fields

Parsed per the shadPS4 `Image` bit layout (the exact runtime target):

| field | extraction | NHL 15 head values |
| --- | --- | --- |
| `data_format` | `(dword1 >> 20) & 0x3F` | BC1/BC3/BC5 (see below) |
| `width - 1` | `dword2 & 0x3FFF` | 1024 (cm), 512 (nm) |
| `height - 1` | `(dword2 >> 14) & 0x3FFF` | 1024 / 512 |
| `tiling_index` | `(dword3 >> 20) & 0x1F` | **13** (uniform) |
| `last_level` | `(dword3 >> 16) & 0xF` | full mip chain |
| `type` | `(dword3 >> 28) & 0xF` | 9 = 2D texture |

**GNM `data_format` → BCn** (GCN `IMG_DATA_FORMAT`): `0x23`=BC1, `0x24`=BC2,
`0x25`=BC3, `0x26`=BC4, `0x27`=BC5, `0x29`=BC7. NHL 15 head maps: `cm`/`am`/`sm`/
`coeff` = BC1, `hair_cm` = BC3, `nm` = BC5.

## Tiling — 1D micro (verified)

Despite the T# reporting a "2D" tile mode (`tiling_index = 13`), NHL 15's head
textures are laid out as simple **1D micro tiling**, confirmed by decoding a real
head to a clean image (no pipe/bank/macro swizzle):

- The base mip is split into **8×8-block micro tiles** (a 32×32-texel tile),
  padded to whole tiles.
- Micro tiles are stored in **row-major** order.
- Within a tile, the 64 blocks follow the GCN **non-displayable Z-order**:
  `x0,y0,x1,y1,x2,y2`.

De-tile (`detile_micro`): for output block `(bx, by)`,
```
tile   = (by / 8) * tiles_per_row + (bx / 8)
within = zorder(bx & 7, by & 7)          // x0,y0,x1,y1,x2,y2
src_block = tile * 64 + within
```
The result is a linear row-major BCn block layout — a standard mip ready for the
BCn decoder. **No 16-bit byte-swap** (unlike the Xbox 360 texlib path).

The Z-order within-tile is essential: a row-major within-tile order still yields a
recognizable face but leaves a ~2× brightness discontinuity at every 32-texel tile
boundary (measured by a per-boundary gradient metric) that is nearly invisible in
the flat texture but reads as **washboard ripples** on the lit, magnified 3D head.
Z-order makes the boundaries seamless (ratio ≈ 1.1 vs ≈ 2.6).

## Decode

- **BC1/BC2/BC3** round-trip through the existing DDS block decoder
  (`tdb-archive-codec`, via `studio-domain`'s `gnf_to_rgba` → `linear_to_rgba`).
- **BC7** (NHL 16+ player portraits) decodes through the pure-Rust
  `texture2ddecoder` crate (`gnf::decode_bc7`) — verified on a real 512×340 BC7
  portrait. The same micro-tile de-tile applies (128-bpp, like the 1024 hair BC3),
  and it comes out seamless — the 512-px drift below is **BC5-specific**, not a
  general small-texture problem.
- **BC4/BC5** have dedicated single-/dual-channel decoders in `gnf.rs`, but the
  index decode is not yet correct on real data — skipped for now.

## Player portraits (verified)

Portraits are **title-drifted** and resolved by `AssetRoots::portrait_path` (PS4
branch), probed by existence, then decoded by magic in `decode_portrait`:

| title    | path (under `nhl/ui`)                          | format         |
| -------- | ---------------------------------------------- | -------------- |
| NHL 15   | `assets/portraits/p{id}.png`                   | plain PNG      |
| NHL 16+  | `kettle_assets/playerportraits/p{id}.gnf`      | GNF **BC7**, 512×340 |

Keyed by the player-bio `lOnr` (portrait id), single size (no separate HUD set).

## Resolution & wiring

- Files: `nhl/data/kettle/rendering/characters/player/head_{slug}_{map}.gnf`
  and `hair_{slug}_{map}.gnf` (`map` ∈ `cm`/`nm`/`sm`/`am`/`coeff`), name-slug
  keyed. Resolved by `AssetRoots::texture_paths` (see `assets.rs`).
- Preview: `head_preview::decode_textures` dispatches `.gnf` → `gnf::parse` →
  `gnf_to_rgba`, producing an RGBA `PreviewTexture` named by the file stem, which
  the existing name-keyword binding (`head`/`hair` + `cm`) maps to the right
  submesh — so a PS4 head renders textured with no viewer change.
- Export: `export_player_assets` writes each `.gnf` map as a PNG.

## Open items

- **512-px BC5 normal maps (`nm`).** Skipped — decode is not yet correct. The
  BC4/BC5 *decoder* is verified (texpresso decoding the same de-tiled bytes as
  BC3 yields the identical result), so the fault is the **de-tile for the 512-px
  BC5 base mip**, and it is *size*-specific, not format-specific:
  - `cm`/`am`/`sm`/`coeff` (BC1, 1024) and `hair` (BC3, **128-bpp**, 1024) all
    de-tile cleanly with the row-major-tiles + Z-order-within scheme (seam ratio
    1.14–1.27), so 128-bpp and the scheme itself are fine at 1024.
  - For `nm` (BC5, 512) the row-major placement gets the **top** tile-row right
    (noise ~17) and **drifts** monotonically downward (noise ~100 mid-map) —
    the classic signature of an unaccounted **pitch/height pad** on the small
    base mip. But no integer tile-row stride (16–22), tile order (row / Morton /
    column), or base offset (start / end) removes the drift. The exact small-BC5
    padding rule is still unknown.
  - Impact is limited to a panel thumbnail: only the diffuse `cm` binds to the
    model, and it renders correctly.
- **Mip levels beyond base.** Only the base mip is de-tiled today.
- **Write-back.** Read-only. Re-tiling + GNF re-wrap for texture replacement is a
  separate effort, and depends on whether ShadPS4 honors loose `.gnf` overrides
  vs requiring repack into the `EB\0\3` `game*.big`.

## Related

- Decoder: `crates/tdb-rx2/src/gnf.rs`
- RGBA bridge: `crates/studio-domain/src/assets.rs` (`gnf_to_rgba`)
- Mesh companion (`.mcd`): see `docs/formats/ps4-rsf-mesh.md`

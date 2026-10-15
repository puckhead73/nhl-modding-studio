# Handoff P1.4-XP — league-logo packaging on every platform: actual members, codecs, replace/repack routes

Research lane, 18 September 2026 (R1 follow-on to [14](14-phase1-league-logo-assets.md)). Consumer: developer
Phase 1.4 (league logo preview / replace on all 27 targets), S5/C4 logo resolver. Machine-readable:
[`data/leaguelogo-packages-xplat.json`](data/leaguelogo-packages-xplat.json) (index 0 package of every
family on 10 Xbox 360 + 9 PS3 sources: member layout, offsets, RefPack, texture kind/dims/format) and
[`data/ps4-leaguelogo-members.json`](data/ps4-leaguelogo-members.json) (every `leaguelogo` member in every
`databigs/*.big` of the seven PS4 installs: archive, entry, offset, stored size, `chunkzip` chunk list,
GNF T# fields, decoded hash). Probes: `leaguelogo_pkg_xplat.py`, `ps4_leaguelogos.py`.

Evidence: **observed** on the X:/H: installs listed in the JSON (read-only; no game bytes stored). "Studio
codec" columns cite the crate function by name; whether the *game* accepts a rewritten member is a runtime
question for every row (nothing here is game-verified).

## Plain-language summary

Every platform stores a league logo as a texture keyed by the family-wide **league art id** (handoff 14):
`l{artid}` under a per-size family. The **container chain differs by generation and platform** — three
Xbox 360 chains, three PS3 chains, two PS4 chains — but within a chain all titles behave the same, and the
dimensions/pixel formats are stable (64×64 and 256×256 RGBA8, 128×64 DXT5 on gen7; 64/256/512 BC7 on PS4
16–21; PNG on PS4 NHL 15). Studio already has a decoder for every chain except the PS3 08–11 RenderWare
texlib (`RW4ps3`). Replacing a logo *with the same dimensions* is the cheap case on every platform: the
texture bytes have a fixed size, only the compression wrapper (RefPack / chunkzip) changes length, and the
archive layer has an in-place route when the new member is not longer. Changing dimensions additionally
touches a 288-byte Apt geometry record inside the gen7 packages (it carries the quad size as floats).

## 1. Chains (observed; index 0 = NHL, identical shape for every index)

| Chain | Titles | Archive → package | Package | Texture (leaguelogos / big / small / wide) | Studio codec |
|---|---|---|---|---|---|
| **X-A** | Xbox 07, 08, 09, 10 | `data0.big` (07/08) / `fe.big` (09/10), **BIG4** archive, entry = **RefPack'd** BIGF | BIGF, 4 members `0` Apt Data · `1` Apt const · `<n>` raw texlib · `<n+1>` 288 B record | `\x89RW4xb2` texlib, **uncompressed** inside: 64×64 `0x86` (ARGB8, 16 384 B) / 256×256 `0x86` (262 144 B) / 128×64 `0x54` DXT5 (16 384 B incl. mips) / 256×256 `0x86`. NHL 07: 100×104 `0x86`, `small`/`wide` texlibs use a layout `rx2_preview.parse` does not read | `tdb-archive-codec::refpack` + `tdb-rx2::texlib` (raster read/write) |
| **X-B** | Xbox 11 | `fe.big`, **EB v0x400** (16 B rows), entry = BIGF | BIGF, 6 members `0` · `<n+1>` (288 B) · `sg1` (0 B) · `<n>` **RefPack** · `sg2` (0 B) · `1`; members 64-byte aligned | RefPack → `\x89RW4xb2` (same dims/formats as X-A) | refpack + texlib |
| **X-C** | Xbox 12, 13, 14, 15, Legacy (+ Recompiled `game\`) | `cache.big`, EB v0x400, entry = BIGF (loose copies also present in the X: extraction trees, not on disc) | BIGF, 6 members as X-B, packed (no alignment) | RefPack → **XPR2/TX2D**: `leaguelogos` 64×64 `8_8_8_8` (fmt byte 0x86, data 16 384 B, dw10 `0x00000C14`) · `big`/`wide` 256×256 `8_8_8_8` (262 144 B) · `small` 128×64 **DXT5** (16 384 B, dw10 `0x00000D10`) | refpack + `tdb-rx2::xpr` (portrait chain) |
| **P-A** | PS3 08 | `data0.big` BIG4, entry = BIGF | BIGF 6 members, texture member **raw** (not RefPacked) | `\x89RW4ps3\0` texlib: 16 896 B (64×64 RGBA8 + 512 B) / 8 704 B (small) / 262 656 B (wide) | **none** (RW4 PS3 texlib not implemented) |
| **P-B** | PS3 09, 10, 11 | 09/10: `fe.big` BIG4; 11: **loose** `USRDIR/fe/ion/artassets/…` on disc | BIGF 6 members, texture member RefPack | RefPack → `RW4ps3` (same sizes as P-A; `big` 262 656 B) | refpack; texlib **none** |
| **P-C** | PS3 12, 13, 14, 15, Legacy | **loose** on disc `USRDIR/fe/ion/artassets/<family>/l{n}.big` **and** the same bytes as `cache.big` EB v0x400 members (Legacy: `leaguelogos/l0.big` entry 4068 @139 669 648, sha16 `595b884b…` = loose) | BIGF 6 members, texture member RefPack | RefPack → **DDS**: 64×64 / 256×256 32-bit RGBA (`pf_flags 0x41`, no fourcc, 0 mips, 16 512 / 262 272 B) · `small` 128×64 `DXT5` (8 320 B, 0 mips) | refpack + `tdb-archive-codec::dds` |
| **S-A** | PS4 15 | `databigs/inxp10.big` EB **v0x00100400** (16 B rows), entry = file; `patch_inxp00.big` overrides `l12` (both sizes) | — | **PNG**, stored raw: `nhl/ui/assets/leaguelogos/size{64,256}/l{artid}.png` (l0–l7, l10–l14, l100); also `playnow/leaguelogos/leaguelogo.png`, `gmconnected/fullcalendar/leaguelogo.png` | any PNG codec; EB 0x00100400 reader is the one the LOC-W1 record calls unsupported — check |
| **S-B** | PS4 16, 17, 18, 19, 20, 21 | `databigs/inxp_10.big` (19: `inxp_10-010.big`) EB **v0x00110400** (20 B rows); NHL 19 `patch_inxp_52.big` overrides `memcup(_fr)` | — | `chunkzip` (padded header, window **0x20000**, 1 chunk for 64/256, 3 chunks for 512 = 0x20000+0x20000+256 B) → **GNF** `BC7` (dfmt 0x29), **no mips** (`last_level 0`), `tiling_index 13`, `contents 0xF8`; sizes 64 (4 352 B), 256 (65 792 B), 512 (262 400 B, from NHL 17) | `tdb-archive-codec::decompress_chunkzip` + `tdb-rx2::gnf` (parse / `replace_base_mip` / `encode_gnf`, `bc7::encode`) |

PS4 name sets: NHL 16 `l0–l7, l10–l15, l100` (no `_fr`); NHL 17 adds `size512`, `_fr` French variants and
`memcup`; NHL 18 adds `l18–l20`, `eliteleaguelogo_large(_fr)` (252×268, under `competitiveseasons/`); NHL 19/20
add `l21, l23–l25`; NHL 21 drops `memcup`/`eliteleaguelogo` and adds `l99(_fr)`. Cross-title identity of the
decoded GNFs: 86 of 140 shared names are byte-identical across every title that has them; NHL 21 refreshed
most club-league logos (`l1, l2, l6, l7, l12, l13, l18, l19`), NHL 18 refreshed `l10`, NHL 17 `l2/l12/l13`
(list in the JSON). The Xbox X-C packages are byte-identical across 13/14/15/Legacy (`leaguelogos/l0.big`
sha16 `a7167e6d…`, `big` `15d3a31a…`); NHL 12 differs (Elitserien-era art).

## 2. The non-texture members (gen7 packages, X-B/X-C/P-B/P-C)

`0` = `Apt Data:1:5:4` (240–264 B; Apt/ION UI object), `1` = `Apt constant file` (32 B), `sg1`/`sg2` = empty
segment markers, and `<n+1>` = a **288-byte record** whose body is a vertex/quad description with the texture
size as IEEE floats: `leaguelogos` carries `0x42800000` (64.0) at the 4 corner slots, `leaguelogosbig`
`0x43800000` (256.0), `leaguelogossmall` `0x43000000`/`0x42800000` (128.0 × 64.0). Header words
`288, 1, 0, 16, 272, 1, 0, 16, 256, 1, 0, 0, 0xFFFFFFFF, <n>, …` (second-to-last word = the texture member's
numeric name). **Same-dimension replacement leaves this record untouched; a dimension change must rewrite
these floats** (untested at runtime). The member numbering `<n>`/`<n+1>` differs per family (`6/7`, `2/3`,
`8/9`) and is referenced from member `0`, so do not renumber.

## 3. Replace / repack candidates (developer), with the validation each needs

| Chain | Read | Same-dims replace (recommended first) | Size-changing replace | Validate |
|---|---|---|---|---|
| X-C (12–Legacy Xbox, Recompiled) | EB member → BIGF → `refpack::decompress` → `xpr::parse` | re-encode XPR2 with the original T# (fixed data size) → `refpack::compress` → **pad with zero bytes after the terminator to the original member length** (Studio's decoder halts at `0xFC–0xFF`; EA's runtime decoder is expected to as well — runtime check) → BIGF unchanged elsewhere → `Eb::replace_payload_in_place` | if the RefPack stream is longer than the original: rewrite BIGF index (member offsets/sizes, keep names and order) → `Eb::replace_payload` (reflow) | decode round-trip equals the intended RGBA; T# dims/format unchanged; BIGF member count/names/order unchanged; 288 B record unchanged; archive re-parses; loose-tree copy and archive copy kept consistent (or the user picks one — precedence unknown) |
| X-B (11 Xbox) | as X-C but texlib | `texlib` raster write (same dims/format, tiled + swap16) → RefPack → pad → in-place | as X-C; keep the 64-byte member alignment of NHL 11 packages | as X-C |
| X-A (07–10 Xbox) | BIG4 entry → RefPack → BIGF → raw texlib | texlib write → BIGF (member sizes fixed, raw) → **outer** RefPack → pad to the BIG4 entry size → in-place BIG4 entry write | BIG4 index rewrite (offsets are absolute u32 BE, entries tile the file) | + BIG4 `size_le`/index size consistency; loose `data0/` (NHL 08 PS3 ships one) is the only loose variant on Xbox |
| P-C (12–Legacy PS3) | loose file **or** `cache.big` member (identical bytes) → BIGF → RefPack → DDS | DDS with same header → RefPack → pad → write loose file (disc-faithful route) and/or `cache.big` in-place | loose: just write; archive: reflow | + both copies equal after write (or document which one the runtime loads — unknown) |
| P-A / P-B (08–11 PS3) | BIGF → (RefPack) → `RW4ps3` | **blocked**: no RW4 PS3 texlib codec in Studio; the PS3 texlib layout was not analysed here (16 896 B = 16 384 pixels + 512 B header/table; raster order/swizzle unknown) | — | — |
| S-B (16–21 PS4) | EB 0x00110400 member → `decompress_chunkzip` → `gnf::parse` | `gnf::replace_base_mip`/`encode_gnf` (BC7, no mips, tiling 13) → `compress_chunkzip` (window 0x20000, deflate) → in-place if ≤ original, else `replace_payload` (reflow of a 28 k-entry archive; dedup blocks split) | same | decoded GNF T# equal (dfmt 0x29, dims, `last_level 0`, tiling 13); chunk count = ceil(size/0x20000); patch archives (`patch_inxp_5x`) do not shadow the edited name — or edit the patch copy instead (precedence unknown, naming suggests patch wins) |
| S-A (15 PS4) | EB 0x00100400 member = PNG | PNG encode → in-place if ≤ original else reflow; `patch_inxp00.big` `l12` override | same | archive version 0x00100400 must be supported by the EB reader/writer (LOC-W1 record says it is not yet) |

Stored-chunk alternative for S-B: `chunkzip` accepts flag `4` (stored) chunks, so a replacement can be made
**size-deterministic** (0x28 header + per-chunk 8 B + raw bytes) at the cost of being larger than the deflate
original — always a reflow, never in-place.

## 4. Archive-backed vs loose-only, and precedence (left as runtime questions)

- Xbox 07–10: archive only on disc; the X: trees carry extracted loose copies (`fe/ion/artassets/…`) too.
- Xbox 11–Legacy: archive only on disc (`fe.big` / `cache.big`); loose copies exist only in the extraction
  trees. Recompiled `game\` = archive route; a `_compiled` loose tree is the user's convention (handoff 01 §5).
- PS3 08–10: archive only (`data0.big` / `fe.big`). PS3 11: loose only for these packages (no `fe.big`).
  PS3 12–Legacy: **both**, byte-identical (Legacy verified for all four families).
- PS4: archive only (`inxp*.big` + `patch_*` overrides).
- Which copy the engine loads when both exist (Xbox loose vs archive under Xenia; PS3 loose vs `cache.big`;
  PS4 base vs `patch_`) is **not established** by static inspection.

## 5. Reproduce

```
python probes/leaguelogo_pkg_xplat.py out.json 0        # any index; 10 Xbox + 9 PS3 sources
python probes/ps4_leaguelogos.py out.json leaguelogo    # seven PS4 installs under H:\Emulators\ShadPS4\Games
python probes/eb_list.py "<cache.big>" --extract fe/ion/artassets/leaguelogos/l0.big pkg.big
python probes/xpr_preview.py pkg.big out.png            # X-C only
```

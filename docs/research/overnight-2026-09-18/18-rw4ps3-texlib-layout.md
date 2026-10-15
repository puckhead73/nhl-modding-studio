# Handoff C-PS3 — `RW4ps3` texlib layout (PS3 NHL 08–Legacy RenderWare 4 textures): bounded decoder + same-dimension replacement

Research lane, 18 September 2026 (follow-up dispatch item 1). Consumer: developer league-logo workflow chains
P-A/P-B (handoff 16) and, later, the PS3 rink/ice/jersey texlibs (`rendering/**/*.rpsgl`). Machine-readable:
[`data/rw4ps3-leaguelogo-survey.json`](data/rw4ps3-leaguelogo-survey.json) (36 PS3 league-logo textures on
NHL 08/09/10/11 × four families × indices 0/1/5: wrapper offsets, descriptor words, pixel hypothesis and the
mean absolute difference against the Xbox 360 decode of the same title/family/index). Probes:
`rw4ps3_probe.py` (parse + candidate decodes), `rw4ps3_survey.py`. Nothing under Studio was changed.

Evidence classes below: **observed** = read from the files; **verified** = decoded pixels equal an
independent decode (the Xbox `RW4xb2` texlib of the same art via `tdb-rx2`'s untile/swap16 port);
**hypothesis** = a reading of constant words that was not needed to decode anything.

## Plain-language summary

The early PS3 titles store textures in the same RenderWare 4 "texlib" container as the Xbox 360 titles, with
one difference that makes the PS3 side *easier*: the pixels are stored plain — row-major, no tiling, no
byte-swapping. A 64×64 league logo is 16 384 bytes of A,R,G,B in order; the small 128×64 variant is a plain
DXT5 block stream; the rink logos are plain DXT5 with their mip levels simply concatenated. Decoding the PS3
files this way reproduces the Xbox art pixel-for-pixel on all 36 league logos checked (and on two of three
rink-logo rasters exactly, the third within noise). The container header, file table and descriptor sit at
fixed offsets for the logo packages, so a bounded reader/writer for "same size, same format" replacement is a
small piece of work; the constant descriptor words are copied through untouched.

## 1. Container (observed on all 36 logo textures + 2 rink texlibs; same scheme as `RW4xb2`)

```
0x00  "\x89RW4ps3\0\r\n\x1a\n"      magic (Xbox: "\x89RW4xb2")
0x20  file_count      u32 BE   (logos: 2 — one data slot + one descriptor; rink texlib: 7 = 3 slots + 3 descriptors + name table)
0x30  file_table_off  u32 BE   (logos: 0x1B4; rink texlib: 0x26C)
0x44  hdr_size        u32 BE   (logos: 0x200; rink texlib: 0x380) — pixel region starts here
0x4C  (logos) total pixel bytes (0x4000 / 0x40000 / 0x2000)  — hypothesis: same "data size" word as Xbox @0x54 shifted; not used
file table: 24-byte entries = 5 × u32 BE + 4-byte tag
  00 01 00 30  texture data slot:  ints[0] = offset relative to hdr_size, ints[2] = bytes, ints[3] = 0x80 (align), ints[4] = 1
               (Xbox uses tag 00 01 00 31 for the same slot)
  00 02 00 03  texture descriptor: ints[0] = absolute offset (logos: 0x17C), ints[2] = 0x38, ints[3] = 4, ints[4] = 0xA (logos) / 7
  00 EC 00 10  name table (`*.Raster` names, NUL-joined) — present on rink texlibs, absent on the logo packages
descriptor (0x38 bytes):
  +0..3   raster ordinal within the texlib (0, 2, 4 on the 3-raster rink texlib; 0 on logos) — observed
  +24..27 u32: 0x0000_0408 (ARGB8 logos) / 0x0000_0008 (DXT5 no mips) / 0x0000_0808 (DXT5 with mips) — hypothesis: bit 0x800 = mipmapped, 0x400 = uncompressed
  +28..31 u32: 0x0000_0DE1 on every texture
  +32..39 u32 ×2: 0x0000_6007 ×2 (ARGB8) / 0x0000_83F3 ×2 (DXT5) — pixel-format signature (constant per format)
  +40..43 u32: 0x0000_8035 (ARGB8) / 0x0000_FFFF (DXT5)
  +48     u16 BE width, +50 u16 BE height      — observed (64/64, 256/256, 128/64, 256/128, 128/128)
  +52     u8  mip level count (1 on logos; 7/8/9 on rink rasters) — observed
  +53..55 zero
```

Wrapper lengths: RW4 file = `hdr_size` + Σ slot bytes; logos are exactly `0x200 + pixels` (16 896 / 262 656 /
8 704 B). The RW4 sits RefPack-compressed inside the six-member BIGF package on 09–11 and **raw** on NHL 08
(handoff 16 P-A/P-B).

## 1b. Exact sample coverage (what the 36 entries are — not a full 4 × 4 × 3 grid)

The survey requested indices 0, 1, 5 of every family on every title; a row exists only where the package
exists in that title's source. Measured (✓) / absent in the source (—):

| Title (PS3 source) | `leaguelogos` | `leaguelogosbig` | `leaguelogossmall` | `leaguelogoswide` | rows |
|---|---|---|---|---|---|
| NHL 08 (`data0.big`) | l0 l1 l5 | — (family absent on 08) | l0 (l1/l5 absent) | l0 l1 (l5 absent) | 6 |
| NHL 09 (`fe.big`) | l0 l1 l5 | l0 l1 (l5 absent) | l0 (l1/l5 absent) | l0 l1 (l5 absent) | 8 |
| NHL 10 (`fe.big`) | l0 l1 l5 | l0 l1 l5 | l0 (l1/l5 absent) | l0 l1 l5 | 10 |
| NHL 11 (loose) | l0 l1 l5 | l0 l1 l5 | l0 l1 l5 | l0 l1 l5 | 12 |

So: `leaguelogossmall` is verified on one index only for 08–10; `leaguelogosbig` on 09 on two indices; all
other league indices (2–4, 6+) and every non-logo package are **unmeasured**. The two rink texlibs (§2 last
row) are single files on NHL 11 and Legacy PS3, not a survey. The "linear" finding is therefore established
for the league-logo families of these four titles and for two rink texlibs — it is not a statement about
every PS3 texture (other families may be swizzled or use other formats; the descriptor signature + slot
length check in §3 is what a reader must apply per file).

## 2. Pixel layout (verified against the Xbox decode; `mad_vs_xbox == 0.0` on 36/36 logos)

| Family | Dims | Slot bytes | Layout (verified) | Xbox equivalent |
|---|---|---|---|---|
| `leaguelogos` | 64×64 | 16 384 | **linear ARGB8** (byte order A,R,G,B; row-major; no swizzle; no mips) | `0x86` tiled ARGB |
| `leaguelogosbig`, `leaguelogoswide` | 256×256 | 262 144 | linear ARGB8 | `0x86` |
| `leaguelogossmall` | 128×64 | 8 192 | **linear DXT5**, no mips (Xbox ships 16 384 B = with mips) | `0x54` tiled + swap16 |
| `rendering/logo/texlib_*.rpsgl` (NHL 11, Legacy PS3) | 256×128 / 128×64 / 128×128 | 43 712–43 728 / 10 944–10 960 / 21 872 | linear DXT5, **mip chain concatenated base-first, each level ≥ 16 bytes** (sum of levels == slot bytes on all six rasters) | `chunklzx` → `RW4xb2` DXT5 (handoff 40) |

Candidates rejected by the scoring (`rw4ps3_probe.py`): Morton/Z-order swizzle (MAD 60–95), RGBA/BGRA byte
orders (MAD 24–40); ABGR scores 0.5–0.8 only because most logo pixels are grey/transparent. No swizzled
texture was found in these sets; PS3 titles *can* store swizzled textures elsewhere — apply the format
signature + length check per file rather than assuming linear for every `.rpsgl`.

## 3. Bounded decoder and same-dimension replacement (what to implement; no Studio change made here)

Reader: parse header (0x20/0x30/0x44), walk `file_count` entries, pair each `00 01 00 30` slot with the
descriptor of the same ordinal (`desc[+0..3]`), read `w`, `h`, `mips`, choose the pixel format by the
descriptor signature (`0x6007/0x8035` → ARGB8; `0x83F3/0xFFFF` → DXT5) and confirm it against the slot length
(`w*h*4` or the DXT5 mip-chain sum); reject otherwise (unknown format — DXT1/DXT3 signatures were not observed
and must not be guessed). Convert ARGB8 → RGBA and DXT5 → RGBA with the existing `texpresso` decode; mips are
base-first.

Writer (same dims, same format): encode the new base level (ARGB8 = plain bytes; DXT5 = `compress_bc`), regenerate
the mip chain when `mips > 1` (box filter, 16-byte floor per level), splice into the slot range, leave header,
table and descriptor bytes untouched. Package step per chain (handoff 16 §3): NHL 08 = raw member in BIGF in
`data0.big` (BIG4) — slot bytes are the same length, so the package and archive entry keep their sizes (pure
in-place); NHL 09/10 = RefPack member in a BIGF inside `fe.big` (BIG4) — re-RefPack, pad after the terminator
to the original member length when shorter, else rewrite the BIGF index and re-plan the BIG4 entry; NHL 11 =
RefPack member in a loose BIGF on disc — write the loose file (length free).

Round-trip expectations for a copied specimen (`fe/ion/artassets/leaguelogos/l0.big` of each title):
decode → re-encode the *unchanged* pixels → the RW4 bytes are identical (ARGB8 is lossless; DXT5 is re-used
as-is when no edit is made) → the RefPack stream from Studio's encoder will generally **differ in length**
from EA's stream, so byte-identity holds at the RW4 level, not at the package level; the package must
re-parse (`BIGF` count/names/offsets consistent, RefPack decodes to the same RW4). Validate: RW4 SHA-256
unchanged for a no-op edit; `w/h/mips/format` unchanged; slot length unchanged; Xbox-vs-PS3 pixel MAD stays 0
for the untouched rasters.

## 4. Unknowns / not covered

Meaning of the constant descriptor words (kept opaque); DXT1/DXT3 or non-power-of-two PS3 textures (none in
these sets); the RW4ps3 ice/jersey/puck texlibs (NHL 11–Legacy `rendering/**`, chunkzip-wrapped in the EB
archives — same container family expected, not measured); whether the PS3 runtime tolerates a RefPack stream
with trailing padding (runtime question, same as Xbox); NHL 07 (no PS3 build).

## 5. Reproduce

```
python probes/rw4ps3_survey.py out.json 0 1 5                      # 36 logos, MAD vs Xbox
python probes/rw4ps3_probe.py <file.rw4> <xbox-reference.png> out  # per-file candidates + PNGs
```

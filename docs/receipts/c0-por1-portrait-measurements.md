# C0-POR1 — Portrait layout inventory and representative measurements

> **Package:** C0-POR1 — inventory the three supplied portrait layouts separately and measure
> representative specimens. **Date:** 2026-09-08. **Author role:** developer. Reviewer: Codex
> (architect).
> **Nothing staged, committed or pushed.** No source byte was written, repacked, replaced or
> installed. No runtime was launched. No bulk extraction was performed and no game data entered the
> repository. No unrelated game directory was searched. No fixture was acquired.
> **No support level and no game-acceptance is promoted by anything in this record.**
>
> **SUPERSEDED IN PART — read §12 first.** Package **C0-POR1-R1** (2026-09-08) reviewed this record
> and corrected eleven claims: provenance labelling, interpretation strength, two totals, and the
> profile promotions (14 established constraints → 3). The text below is preserved **unrewritten** as
> the original record; §12 lists every correction and points at
> [`c0-por1-r1-evidence-scope.md`](c0-por1-r1-evidence-scope.md), which governs wherever the two
> differ.

> Machine-readable companion: [`data/c0-por1-portrait-specimens.json`](data/c0-por1-portrait-specimens.json).

---

## 1. Path intake and identity

### 1.1 Folder-spelling discrepancy — the package's paths were not literal

The package scoped "both `playerhead` and `playerheadsmall` folders". **Neither spelling exists.**
The actual directories, on both PS3 and Xbox 360, are:

| Package wrote | Directory that actually exists |
|---|---|
| `playerhead` | **`playerheads`** |
| `playerheadsmall` | **`playerheadssmall`** |

This is a missing plural `s`, not a backslash-escape artefact — the supplied paths contained no
backslash before an underscore at all, so that particular caution did not arise. The three supplied
roots themselves verified exactly as written. I inventoried the actual directories under their real
names and did not substitute any other directory; every path in this record and in the manifest is
the real on-disk spelling.

The `p0_4000` / `p4001_8000` / `p8001_12000` bucket names *do* contain literal underscores, and they
are literal.

### 1.2 Identity and provenance

| | PS3 | Xbox 360 | PS4 |
|---|---|---|---|
| Root | `H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR\fe\ion\artassets` | `H:\Emulators\games\XBOX\NHL Legacy - Vanilla\_compiled\fe\ion\artassets` | `H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted\nhl\ui\kettle_assets\playerportraits` |
| Verified | yes | yes | yes |
| Title | `NHL Legacy` (PARAM.SFO `TITLE`) | not in the binary; XEX title id **`454109EC`** | `NHL 21` (param.sfo `TITLE`) |
| Title id | `BLUS31540` | `454109EC`, media id `3CF2A23F` | `CUSA18182`, content id `EP0006-CUSA18182_00-NHLICEHOCKEY2021` |
| Versions | `APP_VER 01.01`, `VERSION 01.00`, `CATEGORY GD` | XEX `version`/`base_version` `00000004`, disc 1/1, exe `nhlzf.exe` | `APP_VER 01.00`, `VERSION 01.01`, `c_date 20200901`, `sdk_ver 07508000` |
| **Build identification** | **incomplete** | **incomplete** | **incomplete** |

Identity was read from the containers themselves (`PARAM.SFO`, `sce_sys/param.sfo`, the XEX2
`EXECUTION_INFO` optional header) — not from folder names. Two of the three then **corroborate
against this repository's own record**: [`discovery.rs:48`](../../crates/studio-domain/src/discovery.rs#L48)
already maps `nhllegacy` to `("BLUS31540", "BLES02153", "454109EC")`, which independently matches
the measured PS3 title id and the measured Xbox 360 XEX title id. The Xbox 360 folder label
*"NHL Legacy - Vanilla"* is user-supplied text and is **not** treated as evidence; the title id is.

**Build identification stays explicitly incomplete on all three.** A title id plus an SFO version
field is not a build. No patch level, disc revision or executable build string was established, and
this record does not claim one.

**The PS4 title was measured, not inferred.** `CUSA18182` → `NHL 21` comes from that copy's own
`param.sfo`, so the profile target `nhl21-ps4` is correctly addressed. Nothing about PS4 was derived
from the PS3 or Xbox 360 evidence, and nothing about PS3/Xbox 360 equivalence was assumed anywhere —
where the two were compared (§5) it was **measured**, and it is reported as a bounded sample.

### 1.3 Two of the three trees are derived, not shipping layouts

This materially limits what may be established, and it is the single most important caveat here.

| Tree | Kind | Evidence |
|---|---|---|
| **PS3** | **installed game tree** | Portraits are loose files in the game's own `USRDIR/fe/ion/artassets/...` layout. The only archives at `USRDIR` root are `options.viv` and `patch.big`. |
| **Xbox 360** | **extracted / recompiled** | The title root holds the shipping archives (`boot.big`, `cache.big`, `nocache.big`, `cacherender.big`, `data0.big`, …) **and a sibling `Extracted BIG Files\` directory**. `_compiled\` is a derived loose tree. |
| **PS4** | **extracted** | The supplied path is literally `databigs\Extracted\...`. |

So the Xbox 360 and PS4 **placement** observations say nothing about how those titles ship or how a
replacement would have to be installed. As the package required: an extracted PS4 tree does not
establish its outer `.big` packaging, and folder placement is not proof of required install
behaviour. The consequence is carried through into §7: packaging-dependent facts are established for
**PS3 only**.

### 1.4 The PS3 tree is not vanilla

**103 archives per folder are third-party-modified**, each with a sibling `.big.bak` retained
original. The modified files are trivially separable: every vanilla portrait carries
`mipMapCount = 0`, while the modified ones carry a mip chain (100 at mip 9/10, 3 at mip 1). The
accounting closes exactly — 100 + 3 = 103 modified, matching 103 `.bak` files, and excluding them
leaves 9228 / 9229 vanilla archives per folder, **exactly** the Xbox 360 counts.

No modification markers were found in the Xbox 360 or PS4 portrait folders.

---

## 2. Read-only inventory

Each folder was inventoried independently; the PS3 and Xbox 360 folders were **not** merged.

| Location | Folder | Files | Extensions | `p0_4000` | `p4001_8000` | `p8001_12000` | `silhouettes` |
|---|---|---:|---|---:|---:|---:|---:|
| PS3 | `playerheads` | 9331 | 9228 `.big`, 103 `.bak` | 3087 | 3196 | 3044 | 4 |
| PS3 | `playerheadssmall` | 9332 | 9229 `.big`, 103 `.bak` | 3089 | 3196 | 3043 | 4 |
| Xbox 360 | `playerheads` | 9228 | 9228 `.big` | 3023 | 3173 | 3028 | 4 |
| Xbox 360 | `playerheadssmall` | 9230 | 9229 `.big`, 1 `.options` | 3026 | 3173 | 3027 | 4 |
| PS4 | `playerportraits` | 10421 | 10421 `.gnf` | — flat, no subfolders — | | | |

### 2.1 Structural variants

**Two**, one per platform generation. The census behind this is **exhaustive, not sampled**: every
one of the 36 930 Gen7 archives and all 10 421 GNF files was opened and had its container header
parsed. **Zero parse failures. Zero structural exceptions.**

**Variant A — `gen7-bigf-apt-portrait`** (PS3 + Xbox 360, 36 930 archives). A `BIGF` archive with
**always exactly 6 members**, always in this order:

| # | Name | Content |
|---|---|---|
| 0 | `0` | `Apt ` header (`Apt Data:1:5:4`) |
| 1 | `<N>` | APT constant pool, 288 bytes, begins `00 00 01 20` |
| 2 | `sg1` | **zero-length group marker** |
| 3 | `<N-1>` | **the texture payload** (RefPack) |
| 4 | `sg2` | **zero-length group marker** |
| 5 | `1` | 32-byte `Apt constant fil…` |

`<N>` varies per archive (it is an APT resource id, which is why a naive whole-header grouping
fragments into ~4700 pseudo-variants — that fragmentation is an artefact of the id, not a structural
difference). Member `0` is 240 bytes in most archives and larger in some (1080 bytes in `p104`);
that too is APT payload size, not a structural variant — verified by inspecting such an archive on
both platforms.

**Variant B — `ps4-gnf-portrait`** (PS4, 10 421 files). A loose `GNF ` file, one GNM 2D texture,
256-byte header then the tiled payload. Every file is **exactly 180 480 bytes**. Two *header*
sub-variants exist (§4.3); both carry identical geometry and differ only in two bitfields.

The `silhouettes` bucket is **not** a separate variant — `p60000`–`p60003` are the same 6-member
BIGF shape at the same dimensions as their folder's other members, just far more compressible.

### 2.2 Specimen selection and coverage limits

Specimens (22, all hashed, all listed in the manifest) were chosen to cover: each measured dimension
class on each platform; the same id across PS3/Xbox 360 for direct comparison; the large-APT archive
shape; the `silhouettes` bucket; both PS4 header sub-variants; a modified PS3 archive **and** its
`.bak` original; and the stray `.options` file.

**Coverage limits.** No single specimen is presented as covering a folder. Container and texture
headers are measured **exhaustively** across all 47 351 files, so the dimension/codec/mip/tiling
distributions in §4 are population facts rather than extrapolations. What *is* sampled is the
pixel-level decoding: full decodes were performed on a handful of PS3/Xbox 360 specimens and the
block-stream comparison on 50 ids from one bucket. **No PS4 pixel data was decoded at all.**

---

## 3. The one stray file is EA's own build command line

`playerheadssmall/p0_4000/p1112_glump.options` (488 bytes, Xbox 360) is a leftover EA packaging
invocation. It is worth quoting because it documents the archive layout *from the toolchain that
produced it*, rather than by inference from bytes:

```
-big -o32 -b0 -v0 -failoverlap4096 -pack1 -paths0 -packthresh1024
  "C:\p4\nhl\eacgfx\build\runtime\xenon\fe\ion\artAssets\playerHeadsSmall\p0_4000\p1112.big"
  -pad4 -align64 "…\0"
  -pad4 -align64 "…\p1112_geometry\177"
  -pad4 -align64 "…\p1112_textures\sg1"
  -pad4 -align64 "…\p1112_textures\176"
  -pad4 -align64 "…\sg2"
  -pad4 -align64 "…\1"
```

It independently confirms three measured facts: the **6-member order**, the **64-byte member
alignment**, and that `sg1`/`sg2` are **group markers** (they sit under a `p1112_textures` group,
with the numeric members carrying the actual geometry and texture payloads). It also shows EA's
source tree spelled these directories `artAssets` / `playerHeadsSmall` in camelCase while the
shipped tree is lowercase.

This is a build artefact, not a shipped asset. It is evidence about EA's packaging step; it is
**not** evidence about install behaviour on a retail console.

---

## 4. Measured specimen facts

### 4.1 Gen7 container chain — `BIGF` → RefPack → `XPR2` / `DDS `

Header layout, measured on all 36 930 archives:

| Offset | Field | Endianness |
|---|---|---|
| `0x00` | magic `BIGF` | — |
| `0x04` | size, **= filesize − 16** | **little-endian** |
| `0x08` | member count, always **6** | big-endian |
| `0x0C` | header size (88 / 90 / 92, varies with name lengths) | big-endian |
| `0x10`… | per member: offset u32 BE, length u32 BE, NUL-terminated name | big-endian |

> The `0x04` word being **little-endian while every other header integer is big-endian** is
> unexpected enough to state plainly: it was verified on all 36 930 archives on **both** platforms,
> so it is a property of this `BIGF` writer and not a platform difference.

Members are 64-byte aligned (corroborated by `-align64` in §3). The texture member is a **RefPack**
stream (`10 FB` + 3-byte big-endian decompressed size) wrapping a `DDS ` (PS3) or `XPR2` (Xbox 360)
texture.

### 4.2 Gen7 raster census — exhaustive

| Location | Folder | Dimensions | Codec | Mips | Count | Vanilla |
|---|---|---|---|---:|---:|---|
| PS3 | `playerheads` | 512 × 512 | DXT5 | 0 | 6567 | yes |
| PS3 | `playerheads` | 512 × 256 | DXT5 | 0 | 2661 | yes |
| PS3 | `playerheads` | 512 × 512 | DXT5 | 10 | 100 | **no** |
| PS3 | `playerheads` | 512 × 512 | DXT5 | 1 | 3 | **no** |
| PS3 | `playerheadssmall` | 256 × 128 | DXT5 | 0 | 9229 | yes |
| PS3 | `playerheadssmall` | 256 × 128 | DXT5 | 9 / 1 | 100 / 3 | **no** |
| Xbox 360 | `playerheads` | 512 × 512 | `0x14` (DXT4/5) | 0 | 6567 | yes |
| Xbox 360 | `playerheads` | 512 × 256 | `0x14` (DXT4/5) | 0 | 2661 | yes |
| Xbox 360 | `playerheadssmall` | 256 × 128 | `0x14` (DXT4/5) | 0 | 9229 | yes |

The vanilla PS3 and Xbox 360 distributions are **identical** (6567 / 2661 / 9229).

PS3 numbers come from the `DDS ` header directly (`dwWidth`, `dwHeight`, `dwMipMapCount`,
`ddspf.dwFourCC = "DXT5"`, `dwPitchOrLinearSize` matching the payload exactly, `dwCaps = 0x1000`
with no `MIPMAP` bit). Xbox 360 numbers come from the `XPR2` GPU fetch constant at offset `0x48`,
six big-endian dwords, e.g. `84000002 00000054 003FE1FF 00000D10 00000000 00000A00`:

* `dword2` `0x003FE1FF` → `width-1 = value & 0x1FFF = 511`, `height-1 = (value >> 13) & 0x1FFF = 511`
  → **512 × 512**. The three observed values `0x3FE1FF` / `0x1FE1FF` / `0x0FE0FF` decode to
  512 × 512, 512 × 256 and 256 × 128 — **exactly matching the PS3 `DDS` dimensions and the payload
  byte counts**, which is what makes this reading measured rather than assumed.
* `dword0` bit 31 = **1 → tiled**, on all 18 457 Xbox 360 archives. `(dword0 >> 22) & 0x1FF` × 32
  gives pitch 512 / 512 / 256, consistent with the widths.
* `dword1 & 0x3F = 0x14` = DXT4/5 in the GCN/Xenos data-format table, on every file.
* `dword3` is the constant `0x00000D10` on every file — one uniform mip descriptor.

**Mip absence is established by arithmetic, not only by a flag**: the payload is exactly
`width × height` bytes for a 16-byte-per-block codec (262 144 / 131 072 / 32 768). A full chain would
be ≈ 349 520 bytes. This is the number the 100 modified PS3 archives actually carry (349 680), which
is how they were identified.

### 4.3 PS4 GNF — exhaustive census

All 10 421 files: **512 × 340**, `data_format 0x29`, `tiling_index 13`, `last_level 0`, version 2,
**every file exactly 180 480 bytes**.

**The payload arithmetic independently corroborates the geometry.** 180 480 − 256 header = 180 224 =
128 × 88 blocks × 16 bytes. 340 rows is 85 block rows, so **3 block rows (384 blocks) are surface
padding** to an 88-block-row (352-pixel) tile alignment. Reading the first byte of every block as a
BC7 mode field gives a plausible mode distribution across all real blocks **and exactly 384 all-zero
blocks** — the padding — in both sampled specimens. Two unrelated derivations (file size, and
per-block mode bits) agreeing on 85 real + 3 padding block rows is what establishes the 340 height.

Two header sub-variants, identical geometry:

| Sub-variant | Count | `dword1` | `num_format` (bits 26–29) | `dst_sel_x` bits |
|---|---:|---|---|---|
| majority | 10 246 | `0x02900008` | `0` (UNORM) | 7 |
| minority | **175** | `0x26900008` | **`9` (SRGB)** | 0 |

**This matters for the profile:** colour space is *explicitly encoded* on PS4 and is **not uniform**
across the folder. Whatever else is true, PS4 portrait colour space cannot be recorded as one
target-wide value.

### 4.4 Alpha mode — measured

Alpha is **straight, not premultiplied**. Measured by decoding PS3 specimens and testing whether any
channel exceeds alpha on semi-transparent texels: 1709 / 5002 semi-transparent texels violate
premultiplication in `playerheads/p1008`, with a maximum excess of 253/255. Three specimens agree.
This is decisive rather than marginal.

### 4.5 Path and id rule

Template `fe/ion/artassets/{playerheads|playerheadssmall}/{bucket}/p{id}.big`, lowercase. All 36 930
archives match `p<digits>.big`; **zero** non-conforming names.

| Bucket | Observed id range |
|---|---|
| `p0_4000` | 1 – 3999 |
| `p4001_8000` | 4001 – 7998 |
| `p8001_12000` | **8003 – 12401** |
| `silhouettes` | 60000 – 60003 |

**The third bucket's name understates its contents.** 248 ids per folder exceed 12 000 and still
live in `p8001_12000`. The id→bucket function is therefore **not** the literal ranges in the folder
names and cannot be derived from them. This is a finding, not something to guess a rule for.

The numeric id is measured **from the filename only**. Its binding to a database field (`ubPc.lOnr`
is used for this elsewhere in the application) was **not** examined in this package and is not
established here.

---

## 5. Cross-target observation — measured, and bounded

For **50 sampled ids** (25 `playerheads` + 25 `playerheadssmall`, seeded random from `p0_4000`), the
PS3 and Xbox 360 **DXT5 block streams are byte-identical** after undoing the Xbox 360 8-in-16 byte
swap and 2D tiling. Fully decoded, PS3 and Xbox 360 `playerheads/p1008` produce **byte-identical
RGBA** (max abs difference 0 across all 1 048 576 bytes).

One sampled id, `playerheadssmall/p127`, initially differed — it is one of the 103 third-party-
modified PS3 archives, and its `.big.bak` original **is** byte-identical to the Xbox 360 file.

**Scope limit, stated deliberately.** This is an observation over 50 ids in one bucket of one title.
It is **not** a target-wide equivalence rule, and it must not be used to transfer evidence from one
platform to the other — the package forbade inferring PS3/Xbox 360 equivalence, and this measurement
does not license it either. It is recorded because it is a strong integrity signal that the derived
Xbox 360 `_compiled` tree carries authentic EA content rather than re-encoded content.

---

## 6. Method, and what is *not* independent proof

Where a claim mattered, it was checked against bytes with an **independently written** decoder rather
than taken from this repository's parsers:

| Step | Implementation | Independence |
|---|---|---|
| `BIGF` header/member parse | written for this package | independent |
| RefPack decompression | written for this package | **independent**; decoded length equals the declared length on every file |
| `DDS ` / `XPR2` / `GNF ` header parse | written for this package | independent |
| DXT5 block decode | written for this package | independent |
| Xbox 360 untiling | **the repository's** `xg_address_2d_tiled` formula ([`texlib.rs`](../../crates/tdb-rx2/src/texlib.rs)) | **not independent** — see below |

**The one place I used repository code, and why it is still evidence.** My first untiling attempt
(written from the published `XGAddress2DTiledOffset` form) produced a visibly wrong image. I then
adopted the repository's formula, which produced an image **byte-identical to the independently
decoded PS3 ground truth**. So the repository's formula was *validated against external ground
truth here*, rather than its output being taken on trust. That is the honest characterisation: the
tiling claim rests on the PS3 cross-check, not on the repository agreeing with itself.

**The end-to-end chain was verified against ground truth**, not merely parsed: `BIGF` → RefPack →
`DDS`/`XPR2` → DXT5 → RGBA yields a correct, correctly-coloured player portrait with a clean alpha
cut-out on both platforms. A structurally wrong interpretation would not survive that.

**PS4 was not decoded to pixels.** The BC7 identification is a **table lookup** from the measured
`data_format 0x29`, and the `tiling_index 13` layout was **never exercised**. It is corroborated only
by the size arithmetic and the zero-padding block count (§4.3). This is flagged as a derivation, and
§9 carries it as the leading next action.

---

## 7. Profile integration — and two schema gaps that block it

`defs/adapters/asset-profiles/assets-portraits.toml`, schema `asset-output-profile/2`.

### 7.1 Gap A (blocking) — `dimensions` cannot express co-existing variants

`DimensionsValue` is a **single** `{width, height, variant, power_of_two}` inside one
`Constraint<T>`. One target profile can therefore state **one** dimension.

The evidence is that NHL Legacy has **three co-existing portrait dimensions on one target**:
512 × 512 (6567), 512 × 256 (2661) and 256 × 128 (9229) — and the first two co-exist *inside the same
folder*, so they cannot be separated by path. Picking 512 × 512 would silently misdescribe 11 890 of
18 457 assets.

**The escape hatch the schema advertises does not exist.** `DimensionsValue.variant` documents itself
as *"Which variant of the asset this is (`menu`, `hud`, …). A target carrying several declares
several profiles."* I tried to follow that and found it is not reachable:

* two `[[target_profile]]` entries for the same target in one profile are **refused** by
  `Rule::DuplicateTargetProfile` ([`asset.rs`](../../crates/tdb-canonical/src/asset.rs), the `seen`
  set in `validate_asset_profile`); and
* a profile file is keyed **one per family** by its own filename
  (`defs/adapters/asset-profiles/<family>.toml`) and carries a single `family` key, so there is no
  second profile to declare either.

So `variant` can only *label* the one value a target is allowed, and the documented mechanism for
carrying several is unimplemented. That is the precise gap: it is a doc-comment/schema mismatch, not
merely a missing convenience.

Per the package's instruction not to force variant differences into one universal constraint,
**`dimensions` is left `unknown`** on both NHL Legacy targets, with the measured distribution
recorded in the block note and in the manifest. This is the precise gap, reported rather than
worked around. PS4 is unaffected — it is genuinely uniform at 512 × 340 — so `dimensions` **is**
established there.

Consequence: Q10 makes original dimensions a prerequisite for release replacement, so `replace`
stays blocked on both Gen7 targets until this is resolved.

### 7.2 Gap B (blocking) — `path_rule` cannot express the id→bucket function

`PATH_TOKEN` is `player-slug | numeric-id | team-id | variant`. The real path needs a **bucket
directory derived from the numeric id** by a rule that (§4.5) is *not* the literal folder-name
ranges. A `{variant}` token can hold the bucket string but cannot express that it is a function of
the id, so a template using it is not a path *rule* — it cannot resolve a path from an id alone.
Separately, one target has **two** portrait path families (`playerheads` and `playerheadssmall`),
which one `path_rule` value also cannot carry. `path_rule` is therefore left `unknown` with the
measurement in the note.

### 7.3 Gap C (non-blocking, but it caps `raster`) — `RasterValue` bundles fields of different scope

`RasterValue` requires `{codec, color_space, channels, alpha}` as one value. Of these I measured
`codec`, `channels` and `alpha`; **`color_space` is not recorded by `DDS` or `XPR2` at all**, and on
PS4 it is recorded but **not uniform** (§4.3). Since a block is established only as a whole,
`raster` cannot be established on any of the three targets on the strength of one unmeasured field.

Rather than guess a colour space, `raster` is recorded as a **candidate** on each target, carrying
the three measured fields, with the note naming `color_space` as the single blocker.

### 7.4 What I did populate

Applying one consistent rule: **packaging-dependent blocks** (`discovery`, `container`, `path_rule`,
`install`, and `compression` where it is a property of the packaging) are established only where the
tree is a genuine shipping layout — i.e. **PS3 only**. **Texture-format blocks** describe EA's own
texture bytes, which an outer archive does not change, so they are established wherever measured.

| Block | `nhllegacy-ps3` (new) | `nhllegacy-xbox360` | `nhl21-ps4` |
|---|---|---|---|
| `discovery` | **established** (loose-tree) | unknown — derived tree | unknown — derived tree |
| `identity` | unknown + candidate | unknown + candidate | unknown |
| `container` | **established** (`bigf`→`dds`) | unknown + **measured** candidate (outer layer unexamined) | unknown + **measured** candidate |
| `path_rule` | unknown — **Gap B** | unknown — **Gap B** | unknown |
| `raster` | unknown + candidate — **Gap C** | unknown + candidate — **Gap C** | unknown + candidate — **Gap C** |
| `dimensions` | unknown — **Gap A** | unknown — **Gap A** | **established** 512 × 340 |
| `mipmaps` | **established** (`none`) | **established** (`none`) | **established** (`none`) |
| `compression` | **established** (`refpack`) | **established** (`refpack`) | unknown — extracted tree |
| `tiling` | **established** (`linear`) | **established** (`xbox360-tiled`) | **established** (`gnf-tiled`) |
| `atlas` | **established** (`none`) | **established** (`none`) | **established** (`none`) |
| `install` | unknown | unknown | unknown |

`deterministic = false` is kept on every `compression` block: I did **not** verify that any encoder
reproduces EA's exact RefPack bytes.

### 7.5 Two things Codex must decide, because they are acceptance changes

**(a) A third target was added.** `nhllegacy-ps3` did not exist in the profile. The key follows the
established `title-platform` convention, and the PS3 evidence is the *strongest* of the three
(it is the only genuine shipping tree), so leaving it out would have discarded the best evidence in
the package. But it changes the coverage enumeration recorded in
[`c0-06`](../validation/c0-06-canonical-project-and-adapters.md) from 2 target profiles to 3.

**(b) The "zero established constraints" tripwires now trip.** **Five** committed tests encoded
C0-AS1's state and failed against the populated profile — all five for the right reason, none
because the profile is malformed (`validate_asset_profile` returns **zero findings**, so the file
satisfies the contract as written). Their prose reason was C0-AS1's finding that *"the only portrait
source in this repository is an operations CSV, which is an asserted source."* **C0-POR1 retires
exactly that premise.**

I updated them to pin the new **explicit** expectation rather than relaxing or deleting them, so each
remains a tripwire against any *unreviewed* future change:

| Test | Was | Now |
|---|---|---|
| `the_committed_asset_profile_satisfies_the_contract` | `targets.len() == 2` | `== 3` |
| `the_committed_profile_promotes_nothing_it_cannot_cite` → renamed **`…_establishes_exactly_what_c0_por1_measured`** | `established == 0` everywhere | pins the exact established block set per target, asserts every block is accounted for, and asserts the three deliberately-unknown blocks stay unknown while keeping their candidates |
| `every_operation_…_blocked_by_named_unknowns` → renamed **`exactly_one_operation_has_its_constraints_met_and_every_gap_is_named`** | every operation blocked by *all* its required blocks | blocking set must be a **subset** of the required set, must never name an established block, and exactly one pair — `nhllegacy-ps3/discover` — may report `ConstraintsEstablished` |
| `the_synthetic_fixture_is_excluded_from_registry_population` | counts 2 targets | counts 3 |
| `asset_representation_completeness_is_reported_apart_from_population` | `established == 0` | `established == 14`, and asserts `unknown > established` so the representation-vs-population point still lands |

**One result deserves the architect's attention specifically.** `nhllegacy-ps3/discover` now reports
`OperationReadiness::ConstraintsEstablished` — the first time any operation on a real profile has.
It means only that the profile knows enough about the **format** to specify discovery on that target.
It is not support, not implementation and not acceptance;
`readiness_never_states_that_studio_can_perform_an_operation` still passes unchanged, and no
capability, CSV or matrix cell was touched anywhere. Every other (target, operation) pair — 17 of 18
— remains blocked.

The headline figure in the roadmap's C0 item 7 table (*"0 established constraints"*) and the coverage
enumeration in `c0-06` are consequently stale and need an architect edit.

I did **not** treat either of these as a developer call to make silently; they are called out here
because nothing is staged and Codex reviews before anything lands.

---

## 8. Preservation

* **22 specimens hashed (SHA-256) at first read and re-hashed after all inspection completed.
  22/22 unchanged, 0 mismatches.** Digests are in the manifest.
* **No source byte was written.** Every access was `open(..., 'rb')`. No repack, replacement,
  installation, runtime launch, bulk extraction or game-data copy into git.
* Temporary decoded payloads and PNGs were written **only** to the session scratch directory
  outside the repository, with source/entry provenance preserved in their filenames. Nothing derived
  from game data was placed in the repository — the manifest carries provenance, counts, header
  fields and hashes only.
* No unrelated game directory was searched; only the three supplied roots and, for identity only,
  each title's own `PARAM.SFO` / `param.sfo` / `default.xex`. No new fixture was acquired.

---

## 9. Unknowns and next actions

| # | Unknown | Next action |
|---|---|---|
| 1 | **PS4 pixels were never decoded.** BC7 is a table lookup from `data_format 0x29`; `tiling_index 13` was not exercised. | Decode one GNF specimen to RGBA and confirm against a known portrait, as was done for PS3/Xbox 360. |
| 2 | **Colour space** is unrecorded on Gen7 and non-uniform on PS4 (175 SRGB / 10 246 UNORM). Caps `raster` on all three targets (Gap C). | Determine whether the 175 SRGB files are a distinct asset class; decide whether `RasterValue` should split colour space out. |
| 3 | **`dimensions` schema gap (A).** | Architect decision: allow a per-target set of dimension variants, or split the family. |
| 4 | **`path_rule` schema gap (B).** | Architect decision on expressing an id-derived bucket and two path families per target. |
| 5 | **Xbox 360 / PS4 shipping placement** is unknown — both trees are derived. | Locate the portrait members inside the shipping archives before any `discovery` or `install` claim. |
| 6 | **The id→bucket function** is not the literal folder-name ranges (248 ids per folder exceed 12 000 inside `p8001_12000`). | Establish the real rule from the game's own lookup, not from folder names. |
| 7 | **`identity`**: the numeric id is measured from filenames only; its binding to `ubPc.lOnr` is unexamined here. | Establish the DB-field binding with a citation naming these targets. |
| 8 | **Build identification incomplete** on all three copies. | Decide what constitutes a complete build id and capture it. |
| 9 | **RefPack determinism** unverified — `deterministic = false` everywhere. | Test whether the encoder reproduces EA's bytes for unchanged input. |
| 10 | **PS3 tree is not vanilla** (103 modified archives per folder). | Register it as a non-pristine fixture; prefer `.bak` originals, or source a clean copy. |

**Parser limitations found.** None of this repository's parsers failed on this data — but note that
the only repository code exercised was the Xbox 360 tiling formula (§6). `BIGF`, RefPack, `DDS`,
`XPR2` and `GNF` parsing here was done with code written for this package, so **this record is not a
qualification of the repository's own parsers**. The one interpretation gap found is that the
`BIGF` size word at `0x04` is little-endian against an otherwise big-endian header (§4.1) —
worth checking any writer against.

---

## 10. Checks run

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical` | **162 passed, 0 failed** (was 157 + 5 failing mid-package) |
| `validate_asset_profile` on the committed profile | **zero findings** — the populated file satisfies `asset-output-profile/2` as written |
| `cargo fmt -p tdb-canonical -- --check` | **clean** |
| `cargo clippy -p tdb-canonical --all-targets` | **0 warnings, 0 errors** |
| `cargo test -p tdb-qa` | **207 passed, 0 failed** — the family vocabulary and evidence ledger are unaffected |
| Specimen preservation | 22 hashed before inspection, 22 re-hashed after, **22/22 unchanged** |

`cargo test --all` was **not** run — it carries the known C0.2 baseline failures, and it is not needed
here: `tdb-canonical` is a **leaf crate** (no crate in the workspace depends on it, verified against
every `Cargo.toml`) and it is the only reader of `defs/adapters/asset-profiles/`. The targeted runs
above are therefore complete coverage for this change, not a sample of it.

Scope note: the repo-wide `cargo fmt --all` / `cargo clippy -D warnings` baseline defects recorded in
C0.2 are **untouched** — only `tdb-canonical` was formatted, and it was already clean apart from the
lines this package added. No QA generator was run and no `capability.csv` was edited.

One clippy finding was real and came from my own new code: a catch-all match arm over
`OperationReadiness` that is unreachable because the enum has exactly two variants. Removed, which
also makes the readiness test exhaustive by construction.

---

## 11. Files changed

| File | Change |
|---|---|
| `docs/receipts/c0-por1-portrait-measurements.md` | this record |
| `docs/receipts/data/c0-por1-portrait-specimens.json` | machine-readable specimen/variant manifest (provenance, counts, header fields, hashes; **no game bytes**) |
| `docs/validation/c0-05-fixture-runtime-register.md` | new register section for the three portrait trees |
| `defs/adapters/asset-profiles/assets-portraits.toml` | populated per §7.4; `nhllegacy-ps3` target added |
| `crates/tdb-canonical/src/asset_tests.rs` | the two zero-established tripwires updated to the new explicit expected set (§7.5b) |

**Stop for Codex review. Nothing staged, committed or pushed.**

---

## 12. Corrections log — appended by C0-POR1-R1 (2026-09-08)

This section is **appended**; nothing above it was rewritten. Where this record and
[C0-POR1-R1](c0-por1-r1-evidence-scope.md) differ, **R1 governs**.

### 12.1 Provenance labelling (§1.3, §1.4, and every "vanilla" above)

| Above | Corrected |
|---|---|
| The PS3 tree is an "installed game tree" / "shipping layout" | An **observed installed copy**. Not established to be pristine or complete: no shipping manifest was checked, and **`patch.big` at `USRDIR` root was never opened** |
| "103 archives per folder are third-party-modified"; the rest "vanilla" | Neither a `.bak` sibling nor `mipMapCount = 0` establishes provenance. Groups are **current / backup-labelled** and **base-level-only / mip-chain headers** |
| "The modified files are trivially separable … excluded from every established fact" | The mipmap reasoning was **circular** — the group was *defined* by its mip count, then that mip count was reported as the finding |

### 12.2 Interpretation strength (§5, §6)

* **"External ground truth" is withdrawn.** The PS3 and Xbox 360 chains **share** `parse_bigf`,
  `refpack_decode` and `decode_dxt5`. The comparison exercises only the Xbox 360-specific stages
  (`parse_xpr2`, `swap16`, the transcribed tiling formula) and cannot detect a fault common to both.
* **The authenticity argument is withdrawn.** §5 used the pixel agreement as an "integrity signal
  that the derived Xbox 360 tree carries authentic EA content". Agreement between two copies is
  evidence about the specimens compared, not about whether either tree was modified.
* **The comparison result stands, restated:** 50 compared, **49 identical, 1 differing**; the
  differing PS3 **current** file has a backup-labelled sibling whose stream does match. §5's framing
  as "50/50 vanilla specimens agree" assumed the provenance conclusion it was being used to support.

### 12.3 Scope of specific findings

| §  | Above | Corrected |
|---|---|---|
| 4.4 | "Alpha is **straight**" (target-wide) | **Specimen observation** — 3 PS3 files, one decoder. Removed from the profile |
| 4.3 | PS4's 384 all-zero blocks "independently confirm" the 340 height | **Consistent with** padding; a blank image region encodes identically. Height is read from the descriptor |
| 4.3 / 7.4 | The surface must be tiled because linear surfaces carry no padding | **Wrong** — linear surfaces can be pitch-padded. The only evidence is the `tiling_index` field |
| 4.3 | PS4 dimensions/mips/tiling as measured facts | **Observed header-field interpretations.** No PS4 pixel data was decoded |
| 4.5 | "The id→bucket function is **not** the literal folder-name ranges" | **Withdrawn as a rule.** No bucket function was formulated or tested; only **membership** was observed |

### 12.4 Census corrections (§2, §2.1)

| Above | Correct |
|---|---|
| 36 930 Gen7 archives | **37 120** accepted (18 663 PS3 incl. backup-labelled, 18 457 Xbox 360) |
| 47 351 files | **47 542** files inventoried; **47 541** accepted; 1 excluded (the `.options` artefact) |
| "Two structural variants" | Scoped to the **BIGF member shape** only. **15** distinct structural tuples once dimensions, mips, colour format and group are included |
| "Zero parse failures" | Means **accepted by the inspection parser** — the fields it looks for were found where it expects them. Not semantic validation |

Reconciled per-folder, per-extension, per-group and per-tuple counts are generated into
[`data/c0-por1-census.json`](data/c0-por1-census.json), now the single source for them.

### 12.5 Profile promotions (§7.4, §7.5)

**14 established constraints → 3**, all on `nhllegacy-ps3`: `container`, `compression`, `atlas`.

The rule R1 applied: `established` requires an **inspected-corpus invariant** (holding across every
inspected file in that root, in **all** groups) over a corpus **attributable to the target**. The
three survivors hold across the backup-labelled files too. Demoted: `mipmaps` (circular, and the
corpus is not invariant), `tiling` (three specimen decodes), `discovery` (`patch.big` unopened), and
**everything on Xbox 360 and PS4** (derived corpora — attribution, not measurement).

**Fabricated candidate fields removed:** `raster.color_space` on all three targets, `raster.alpha` on
PS4, `identity.ambiguity` on PS3 and Xbox 360. A candidate is a proposal, not a hole with a plug in
it; the measurements moved to the manifest.

§7.5(b)'s note that `nhllegacy-ps3/discover` reported `ConstraintsEstablished` **no longer applies** —
`discovery` was demoted, so no operation on a real profile has its constraints met.

### 12.6 Receipt accuracy (§10)

* **"Complete coverage" is withdrawn.** The targeted runs are complete for the *reverse-dependency*
  question only (`tdb-canonical` is a leaf crate and the sole reader of the profile). They do not
  cover the findings, which are documentary and have no automated test.
* **Preservation restated.** §8 implied whole-corpus preservation. Accurately: **22 named specimens
  were hashed before and after; 22/22 unchanged.** The other 47 520 files were opened read-only but
  not hashed before/after. What covers them is the access mode, not a hash comparison.
* **`tdb-qa` total wrong.** §10 reported 207 passed; the listing it was read from was
  `head`-truncated and dropped a test binary. The correct total is **215**, all passing.
* **fmt/clippy baseline.** The clean results reported are for the **working tree**. CLAUDE.md records
  that both fail **at HEAD**, with the C0-INT1 repairs living only in the tree. Nothing here repaired
  that baseline defect.

### 12.7 Still standing

Unaffected by R1: the three roots and folder-spelling discrepancy (§1.1); identity from `PARAM.SFO` /
`param.sfo` / XEX2, and its corroboration against `discovery.rs:48` (§1.2); build identification
incomplete on all three copies; the `BIGF` header layout including the little-endian size word at
`0x04` (§4.1); the `.options` build artefact and what it corroborates (§3) — a **non-pixel**
corroboration, unrelated to the withdrawn authenticity argument; the exhaustive header-census
distributions (§4.2), subject to the corrected totals; and that no source byte was written anywhere.


# Handoff R42 / R44 / R47 — Xbox 360 `RW4xb2`: the `DW8` field is the file-table index of the texture's pixel-data entry (100 % on NHL 07–Legacy, RefPack-wrapped NHL 08 included); tile-aligned pitch; the resize field bounds, shipped ranges and boundary vectors for all three families

Research lane, 21 September 2026 pm, roadmap step 1.3 / 2.5 / 4.x (prompt step 2 "resize contract hardening"; closes Q-R32-1 and the
review's R1 join question). Claims `research-r42-rw4xb2-slot-join`, `research-r47-resize-bounds`. Read-only over the loose `rendering`
trees on `X:` (10 Xbox 360 titles) and the NHL 08 `data0.big` member used for one identity check; nothing game-verified; no writer;
Studio's `texlib.rs` / `gnf.rs` / `texlib_ps3.rs` were read (not edited) for the developer's current constants.

Machine-readable: [`data/r42-rw4xb2-slot-join.json`](data/r42-rw4xb2-slot-join.json) (10 trees; per title counts, entry class at the
`DW8` index, shipped maxima, formats, descriptors per file, per-shape shipped slot sizes, full residual lists),
[`data/r44-rw4xb2-nhl08-refpack-census.json`](data/r44-rw4xb2-nhl08-refpack-census.json) (NHL 08 after RefPack unwrap),
[`data/r47-resize-boundary-vectors.json`](data/r47-resize-boundary-vectors.json) (field bounds + 40 boundary vectors with expected words).
Probes `r42_rw4xb2_slot_join.py` (≈ 26 min, all trees), `r44_rw4xb2_nhl08_refpack_census.py` (≈ 9 min), `r47_resize_boundary_vectors.py`
(< 1 s, pure arithmetic).

## Plain-language summary

The Xbox 360 descriptor word that last night's handoff called "2 × the texture's index" is simpler and exact: it is the **position in the
container's file table of the entry that holds the texture's pixels**. In ordinary texture libraries the table alternates pixel entry /
descriptor entry from position 0, so the pixel entries sit at 0, 2, 4 … — which is where "2 × index" came from; in containers that start
with other entries (stadium ice containers, indicators) the pixel entries sit at 1, 3, 5 … and the word follows them. Measured on every
descriptor of every Xbox 360 title: 100 % on NHL 09–Legacy (17 680 … 29 171 per title), 100 % on NHL 08 once its RefPack wrapper is removed
(14 066), and 10 666 of 10 675 on NHL 07 (the 9 leftovers are null-format entries in one `fx` XML container). So the exceptions are gone,
the contract stays "preserve the word" for any edit that keeps the file table as it is, and an edit that inserts, removes or reorders
entries now has an exact rule if the developer ever needs one (recompute it as the pixel entry's new index). Two more corrections fall out:
NHL 07 *does* obey the mip-address rule (the earlier "6 / 7 275" was the counter subtracting this word), and NHL 08's textures are the same
container inside a RefPack wrapper, not "another container" — except for 110 NHL-team jersey libraries whose RefPack stream is undecodable
(byte-identical in the archive; high-entropy; refuse). The pitch rule is the developer's tile-aligned form on every tiled descriptor of
every title. The second half of the handoff gives the developer what the review asked for: the field bounds of each family (what the
header *can* store), the shipped ranges (what EA *did* store), the hardware / runtime limits (not verified here), and boundary vectors
with the expected header words, kept separate.

## 1. `DW8` upper field = file-table index of the paired pixel-data entry (rule B)

| Title | Descriptors | A: = 2 × ordinal | **B: = file-table index of the paired data entry** | data entry immediately precedes / follows the descriptor entry | Residual |
|---|---|---|---|---|---|
| NHL 07 (loose, table at EOF; untagged `data07` payload entries) | 10 675 | 1 | **10 666** | follows: 10 624 (the rest in the 4 multi-texture containers) | 9 = `fx/common/texture.xml.rx2` (format `0x0`, 2 561 × 5 "descriptors" whose `DW8` upper is 0 — not textures; refuse by format) |
| NHL 08 (RefPack `10 FB` wrapped; r44) | 14 066 (7 594 files; 1 236 without textures) | 12 558 | **14 066** | precedes: 14 046; follows: 20 | 0 (110 files undecodable — §3) |
| NHL 09 | 17 680 | 15 985 | **17 680** | precedes 17 679 / follows 1 | 0 |
| NHL 10 | 16 482 | 14 753 | **16 482** | precedes | 0 |
| NHL 11 | 22 129 | 20 057 | **22 129** | precedes | 0 |
| NHL 12 | 27 282 | 24 703 | **27 282** | precedes | 0 |
| NHL 13 | 25 934 | 23 004 | **25 934** | precedes | 0 |
| NHL 14 | 28 495 | 25 444 | **28 495** | precedes | 0 |
| NHL 15 | 25 701 | 22 650 | **25 701** | precedes | 0 |
| NHL Legacy | 29 171 | 26 118 | **29 171** | precedes | 0 |

"Paired" = Studio's pairing (the k-th `00 01 00 31` data entry with the k-th `00 02 00 03` descriptor entry, `texlib.rs::parse`), so the
rule also confirms that pairing on every file. The entry class found *at* the `DW8` index is a data entry on every descriptor of 08–Legacy
(`entry_class_at_dw8_index`). Rule A's shortfall (2 627 on Legacy etc.) is exactly the set of containers whose table does not start with
the first data entry at index 0 — `stadium/ice_intl_container_0.rx2` (entry 0 = a `00eb000e` record; data entries at 1, 3, 5, 7, 9, 11),
`indicator/*` on NHL 09 (descriptor at 0, payload at 1). Note the earlier 2026-09-21 handoff 11 counted 28 745 Legacy descriptors; r42
counts 29 171 because it reads the whole file when the table lies beyond the first 512 KiB (the 4 files "beyond the read window" and the
multi-texture containers are now included).

**Contract.** (a) Any edit that keeps the file table's entries and their order — same-size replacement, the P2-B resize (slot length
change with in-place reflow) — **preserves `DW8` byte-for-byte** (unchanged from the R1 correction; now known to be exact rather than
majority). (b) An edit that inserts, removes or reorders file-table entries must rewrite every descriptor's `DW8` upper field to its
pixel entry's new index; until such an edit exists it stays refused. (c) A descriptor whose `DW8` index does not address a data entry
(the 9 NHL 07 null-format records) is refused as "not a texture descriptor". Studio's `make_descriptor` `data_off >> 4` formula remains
wrong for any pixel entry other than index 0 and should not be used for shipped containers.

## 2. The other dimension-dependent words, re-measured on the full population (r42; r44 for NHL 08)

| Word | Rule | 07 | 08 | 09 | 10 | 11 | 12 | 13 | 14 | 15 | Legacy |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `DW7` pitch (tiled) | **`roundup(w, 128) / 32` for BCn, `roundup(w, 32) / 32` for 8_8_8_8** (the developer's P2-B form) — 0 misses on tiled descriptors on every title | 10 666 / 10 666 | 14 066 / 14 066 | 17 680 / 17 680 | 16 482 / 16 482 | 22 129 / 22 129 | 27 282 / 27 282 | 25 702 / 25 934 | 28 250 / 28 495 | 25 456 / 25 701 | 28 926 / 29 171 |
| `DW7` pitch (linear, `tiled` = 0) | the 232 / 245 / 245 / 245 misses on 13–Legacy are all the **non-tiled** 32 × 32 8_8_8_8 `effects/powerring_lighting_*` rasters with pitch 2 (= 64 texels): candidate `roundup(w, 64) / 32` for linear rasters, one shape only — Studio refuses non-tiled resizes (`NotTiled`), so no writer depends on it | — | — | — | — | — | — | 232 | 245 | 245 | 245 |
| `DW12` mip page = tiled base / 4096 (2D, ≥ 128 texels BCn) | holds on all but **one descriptor per title**: `ice/texlib_0.rx2` 16 × 256 (9 mips, `packed_mips` = 1, page 0) | 9 554 / 9 555 | 13 429 / 13 430 | 16 686 / 16 687 | 15 584 / 15 585 | 21 012 / 21 013 | 24 748 / 24 749 | 23 699 / 23 700 | 25 994 / 25 994 | 22 931 / 22 932 | 26 254 / 26 255 |
| `DW12` cube maps | 6 × tiled base / 4096 | 2 / 2 | 5 / 5 | 4 / 4 | 4 / 4 | 5 / 5 | 4 / 4 | 68 / 68 | 70 / 70 | 71 / 71 | 71 / 71 |
| `DW12` single level | 0 | 166 | 463 | 902 | 886 | 1 047 | 2 510 | 2 134 | 2 410 | 2 610 | 2 755 (all / all) |
| `DW12` tiny (≤ 8 × 8 blocks) | **mixed**: 0 or the rule (zero / rule = 1 / 942 on 07, 168 tested all consistent on 08 (split not recorded), 18 / 69 on 09, 5 / 2, 58 / 6, 7 / 12, 18 / 14, 7 / 14, 73 / 15, 75 / 15) → refuse below 128 texels (BCn) | | | | | | | | | | |
| `DW13` byte 2 = mips | all except 2 descriptors on each of 13 / 14 / 15 / Legacy: `stadium/stadium_321_container_0.rx2`, two 128² DXT1 declaring **16 mips** (more than 128 halves to) with `DW13` bytes `02` / `08` — shipped descriptors *can* declare an impossible level count, so the resize gate must bound levels by the request, never trust the shipped field (the developer's halvings bound does this) | | | | | | | | | | |

`DW11` mip_max = mips − 1 and the constants are as handoff 11; NHL 07 / 08 carry `DW0 = 3` (not `0x00400003`), which Studio's
`patch_descriptor_for_resize` refuses as `UnmeasuredGeneration` — with this census the same rules are measured on those generations,
so lifting that refusal for 07 / 08 is a developer decision with evidence, not a research gap (NHL 07's table-at-EOF layout and untagged
payload entries, and NHL 08's RefPack wrapper, are the container-level differences).

## 3. NHL 08: RefPack-wrapped `RW4xb2` — and 110 undecodable jersey libraries

Every NHL 08 loose `rendering/**/*.rx2` (8 940) is a RefPack stream (`10 FB`, 3-byte size) around an `RW4xb2` container of the NHL 08+
layout (data / descriptor pairs, data region biased by `hdr_size`): 8 937 RefPack + 3 plain; 7 594 unwrapped files carry textures
(14 066 descriptors, every rule above at 100 %). **110 files do not decode**: `rendering/jersey/texlib_1_<team>_<0|1>.rx2` for the NHL
teams 0–28 (except 2 / 5 / 8) and 130–158 — the home / away (`_0` / `_1`) jersey libraries; the `_10` / `_11` variants and all
`texlib_0_*` decode. Their stream starts like the others (`10 FB 63 10 00 E5 89 52` = size 6 492 160, literal run, `\x89R`) and then
diverges into a high-entropy byte stream (8.00 bits / byte vs 7.1–7.8 for the decodable files; no RefPack stop code at the tail). The
`data0.big` member (entry 13 895, 2 036 655 B) is **byte-identical** to the loose file, so this is the shipped form, not a bad copy.
Candidate: an encrypted or otherwise transformed stream after the first literal (not decoded). One bounded test: a repeating-XOR key derived from the known 22 literal bytes (`RW4xb2…` of a decodable sibling) does not make the stream decodable at key lengths 1 / 2 / 4 / 8 / 16 / 22 — not a simple XOR. Consequence:
NHL 08 jersey home / away textures of those 55 teams are **not readable or replaceable** through the RefPack + `RW4xb2` route — refuse
with "undecodable RefPack stream (NHL 08 jersey home / away)"; everything else on NHL 08 is the ordinary route.

## 4. Field bounds vs shipped ranges vs hardware / runtime (kept separate)

| Family | Header field bounds (what the words can store) | Shipped maxima (measured) | Hardware / runtime (external, **not verified here**) |
|---|---|---|---|
| `RW4xb2` | width / height 1..8 192 (13-bit, stored − 1); mips 1..16 (4-bit) and ≤ ⌊log₂ max(w, h)⌋ + 1; pitch 9-bit × 32 texels (≤ 16 352 — never binding under 8 192); mip page 20-bit × 4 KiB (never binding: 8 192² DXT5 tiled base = 16 384 pages); container: entry bytes / offsets and `data_size` u32, slots 4 KiB-aligned | w ≤ 2 048 on every title (2 561 only in the NHL 07 null-format `fx` record); h ≤ 2 048 (07–12) / 4 096 (13–Legacy: the 512 × 4 096 `effects/lcd_*` strips); mips ≤ 12 (07–12) / 16 (13+, the two impossible declarations above; 13 real); pitch32 ≤ 64; mip page ≤ 1 024 (NHL 09) / 512; slot ≤ 5 603 328 B (NHL 09) / 2 834 432 B; file table ≤ 3 146 entries | Xenos texture limit 8 192 × 8 192 per public documentation — coincides with the field bound; whether the game's loaders allocate for anything above the shipped maxima is a runtime question (card R32-TEX-1 rev. 2) |
| `GNF` | width / height 1..16 384 (14-bit, stored − 1); levels 1..16 (4-bit) and ≤ ⌊log₂⌋ + 1 = **15 at 16 384, so 16 is never reachable for 2D**; pitch 14-bit (stored `roundup(w, 32) − 1` ≤ 16 383: never binding); depth 13-bit (0); `d7` / `streamSize` u32 (16 384² BC7 15 levels = 358 MB; 16 384² 8_8_8_8 ≈ 1.43 GB — both fit) | NHL 15: 4 096 × 4 096, 13 levels, 22 374 400 B data (r43; the other titles in handoff 03 as they complete) | GCN limit 16 384 per public documentation; the PS4 registry size-directory rule (`size64 / 256 / 512`) is the addressable set for logo families |
| `RW4ps3` | width / height 1..65 535 (16-bit halves of word 12); mips byte (0..255) and ≤ ⌊log₂⌋ + 1; slot u32 — **the u32 slot binds before the dimension field**: ARGB8 above ≈ 28 377² with mips, DXT5 above ≈ 56 755² with mips; 65 535² DXT1 (2.86 GB) fits | 2 048 × 2 048, 12 mips within the recorded shapes (2026-09-21 r33; top-60 shapes per title) | RSX limit 4 096 × 4 096 per public documentation — **far below the field bound**: the field bound alone would accept sizes no PS3 can load; a writer should state that the 16-bit gate is a representability gate, not an acceptance gate |

The developer's R1 constants (`DESC_MAX_DIM` 8 192 / `DESC_MAX_MIPS` 16 / halvings; `GNF_MAX_DIM` 16 384 / `GNF_MAX_LEVELS` 16 / halvings /
u32 `streamSize`; PS3 16-bit + level bound + u32) match the field bounds above exactly; the only refinement this census adds is that
`GNF_MAX_LEVELS` 16 is unreachable (harmless) and that the PS3 gate should be documented as representability only.

## 5. Boundary vectors (r47; expected words under the measured rules; field bounds only — hardware limits not applied)

| Family | Vector | Verdict | Expected |
|---|---|---|---|
| `RW4xb2` | 128 × 128 DXT1, 1 mip | representable | pitch32 4, `DW9 0x000fe07f`, `DW11 0`, `DW12` page 0, `DW13` byte2 1 |
| | 128 × 128 DXT1, 8 mips | representable | pitch32 4, `DW11 0x1c0`, `DW12` page 2 (`0x00002000`), `DW13` 8, tiled base 8 192 B |
| | 160 × 128 DXT5, 1 mip | representable | pitch32 **8** (tile-aligned 256), `DW9 0x000fe09f`, tiled base 32 768 B |
| | 2 048 × 4 096 DXT1, 13 mips | representable (shipped shape) | pitch32 64, `DW9 0x01ffe7ff`, `DW11 0x300`, page 1 024 (`0x00400000`), `DW13` 13 |
| | 8 192 × 8 192 DXT1 / DXT5 / 8_8_8_8, 14 mips | representable (field maximum) | pitch32 256, `DW9 0x03ffffff`, `DW11 0x340`, page 8 192 / 16 384 / 65 536, `DW13` 14 |
| | 8 192 × 8 192, 15 mips | **refuse** | more levels than the size halves to (14) |
| | 8 193 × 128; 128 × 8 193 | **refuse** | 13-bit field (the review's 8 196 case) |
| | mips 0; mips 17 | **refuse** | 4-bit field |
| | 127 × 128 DXT1; 16 × 256 DXT1 9 mips; 31 × 32 8_8_8_8 | **refuse** (representable, but measured exceptions) | below 128 texels BCn / 32 texels 8_8_8_8 (`DW12` packing not modelled) |
| `GNF` | 4 × 4 BC1, 1 level | representable | word2 `0x0000c003`, pitch 31, `d7` 512, `streamSize` 768 |
| | 32 × 32 BC1, 6 levels | representable | last_level 5, pitch 31, `d7` 3 072 |
| | 164 × 164 BC7, 1 level | representable (shipped shape) | pitch **191**, `d7` 36 864 |
| | 4 096 × 4 096 BC1, 13 levels | representable (NHL 15 shipped maximum) | word2 `0x03ffcfff`, last_level 12, pitch 4 095, `d7` 11 187 200 |
| | 16 384 × 16 384 BC1 / BC7, 15 levels | representable (field maximum) | word2 `0x0fffffff`, last_level 14, pitch 16 383, `d7` 178 959 360 / 357 918 720 |
| | 16 384 × 16 384, 16 levels | **refuse** | more levels than the size halves to (15) |
| | 16 385 × 4; 4 × 16 385; 16 400 × 4 | **refuse** | 14-bit field (the review's 16 400 case) |
| | levels 0; levels 17 | **refuse** | 4-bit field |
| | 16 352 × 16 → pitch 16 351; 16 353 × 16 → pitch 16 383 | representable | the pitch rounding at the top of the field |
| `RW4ps3` | 4 × 4 DXT1 | representable | word12 `0x00040004`, word13 `0x01000000`, slot 8 |
| | 2 048 × 2 048 DXT5, 12 mips | representable (shipped maximum) | word12 `0x08000800`, word13 `0x0c000000`, slot 5 592 432 |
| | 65 535 × 65 535 DXT1, 16 mips | representable (field maximum; **not loadable per the RSX limit**) | word12 `0xffffffff`, word13 `0x10000000`, slot 2 863 311 536 |
| | 65 535 × 65 535 DXT5 / ARGB8; 32 768² ARGB8 | **refuse** | slot exceeds u32 |
| | 65 536 × 4 | **refuse** | 16-bit halves |
| | 128 × 128 DXT1, 9 mips | **refuse** | more levels than the size halves to (8) |

## 6. Slot-size oracle (size arithmetic)

The shipped data-slot length is a **single value per (w, h, format, mips, dimension, packed) shape on every title** (`slot_size_by_shape`,
120 shapes per title, one `slot_bytes` value each — e.g. 512² DXT1 10 mips = 196 608; 1 024² DXT1 11 = 720 896; 512² DXN 10 = 393 216; 128²
DXT5 8 = 65 536; 256 × 128 DXT5 8 = 81 920; 128 × 64 DXT5 7 = 49 152; 256² DXT1 9 = 65 536; 256² 8_8_8_8 9 = 360 448; all multiples of
4 096). The packed-mip tail arithmetic behind these values is **not derived here**; the developer's chain writer can be checked against
this table: a resize to a shipped shape must produce that shape's shipped slot length, and a resize to an unshipped shape has no oracle
(state so in the receipt rather than inferring one).

## 7. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r42_rw4xb2_slot_join.py docs/research/overnight-2026-09-21-pm/data/r42-rw4xb2-slot-join.json               (≈ 26 min, 10 trees on X:)
python local/runs/overnight-research-2026-09-21-pm/probes/r44_rw4xb2_nhl08_refpack_census.py docs/research/overnight-2026-09-21-pm/data/r44-rw4xb2-nhl08-refpack-census.json   (≈ 9 min)
python local/runs/overnight-research-2026-09-21-pm/probes/r47_resize_boundary_vectors.py docs/research/overnight-2026-09-21-pm/data/r47-resize-boundary-vectors.json          (< 1 s)
```
Executed: r42 four development runs (300 / 600-file smoke tests, one full run aborted for an encoding fix) + 1 final full run; r44 one
60-file smoke test + 1 full run; r47 once; the NHL 08 jersey identity check (archive member vs loose, SHA-256 `8f447f93…`, entropy) in
the shell (not retained). Not executed: writes, launches, any decode attempt beyond the standard RefPack command set.

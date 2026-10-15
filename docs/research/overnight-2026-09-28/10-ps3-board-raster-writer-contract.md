# Handoff 10 (28 Sep) — PS3 board-ad raster writer: a narrowed same-size, same-format contract with a full-chain test

Research lane, 28 September 2026, published 13:27 MDT (system clock). After-receipt-5 prompt §2. Claim `research-q8-ps3-board-raster-writer`.
It replaces handoff [03](03-board-art-replacement-boundaries.md) §5 **for PS3**, as corrected in [09](09-after-receipt5-corrections.md) K10 / K11. X360 stays under 03 §5 and its gates (§6 below).
This is **for coordinator review.** It does not authorise enabling a writer, and it does not design Studio's transaction layer.

Machine-readable: [`data/r142-ps3-board-raster-writer.json`](data/r142-ps3-board-raster-writer.json) (SHA-256 `0866a36e…`).
Probe: `local/runs/overnight-research-2026-09-28/probes/r142_ps3_board_raster_writer.py` (SHA-256 `d7ee2d48…`), one run of about 3 minutes.
- **Read-only** on the installs: the NHL 14 / 15 / Legacy loose trees and the `stadium.big` / `cacherender.big` members, unwrapped in memory.
- **Mutations** only on four package-owned disposable copies in `scratch-q8`. Each copy was deleted after restore. The folder was then removed after checking it was empty, sat at the expected resolved path, and was not a reparse point.
- Nothing launched. No archive was copied or written.

## Plain-language summary

On PS3, a board-ad picture can be replaced with one of the same size and format by rewriting only that picture's own bytes, including
every smaller copy of it (its mip levels):
- Tested on disposable copies for both formats the PS3 boards use (DXT1 and DXT3), on a loose-plus-archive game (Legacy) and an
  archive-only game (NHL 13).
- Every level read back correctly through an independent decoder, nothing else in the file changed, and restoring the original
  gave the exact installed file.
- The one real container consequence: the archive copy's **compressed length changes**. Writing it therefore needs Studio's real
  archive stager, never an in-place patch.
- No PS3 board uses DXT5, so DXT5 is **not admitted**.
- The old research helper could read past a short data block. The new validator refuses that, and 14 synthetic malformed packages
  prove each refusal.

Unchanged: nobody knows which copy the console loads or how the game shows the result.

## 1. Validated edge (replaces `r137b::board_edges`) — *for writer admission superseded by §8.1 (15:13): the overlap test below was neighbour-only, DXT5 was admitted and zero values normalised*

For each `0xEF0005` material whose dictionary name matches `^(adbd|boardad)` (case-insensitive), the edge exists only if **all** of
these hold. Otherwise the material is refused, with the reason:

1. a predecessor chunk exists, has type `0xEF0001`, and lies inside 0 .. `hdr_size`;
2. that block is **≥ 260 bytes**, so the slot word at +256 lies inside it;
3. the slot word is `< chunk count`, and names a `0x20003` descriptor of **≥ 0x38 bytes** inside 0 .. `hdr_size`;
4. the descriptor's format words u16 @34 and @38 are equal and are `83f1` (DXT1) or `83f2` (DXT3); see §3 for `83f3`;
5. the descriptor u32 @0 is `< chunk count` and names a `0x10030` data entry;
6. the entry span `hdr_size + offset .. + size` lies inside the file, and its size equals the plain chain exactly (§2);
7. the entry overlaps no other data entry, and **no other descriptor owns it**.

**Sharing, kept separate:**
- Several materials naming **one descriptor** is normal. A replacement changes all of them, and the plan lists every one.
- One data entry owned by **several descriptors** is refused. None was measured, but the check stays.

**Synthetic check (in memory).** A valid baseline plus 13 malformed variants: short 240-byte block, wrong predecessor type, block past
the header region, slot word out of range, slot naming a material, descriptor shorter than 0x38, format `6007`, entry word out of range,
entry naming a non-data chunk, entry past the end of the file, entry = 6 × chain (cube), entry owned by two descriptors, and
overlapping entries.
- The validator gives the expected result on all **14 / 14**.
- The old `r137b.board_edges` on the short-block variant returns `adbd_TEST → 3`: a word read from **past the block**. This is the
  defect the coordinator named. The earlier valid-sample results stand, because every measured board-ad block was long enough (§4).

## 2. Byte range (permitted mask)

The data entry, and only the data entry:
- base level followed by every mip level, plain and concatenated;
- level *l* is `max(w >> l, 1)` × `max(h >> l, 1)`, with a floor of one 4 × 4 block per axis;
- 8 bytes per block for DXT1, 16 for DXT3;
- level count = descriptor byte @52.

The mask length equals the entry size (check 6). Nothing else may change:
- header, file table, descriptors, dictionaries, materials, size words;
- every other entry.

Levels are regenerated from the replacement image. PS3's plain layout has no packed tail.

## 3. Admitted formats (narrowed by measurement)

Census of every PS3 venue package ([`r142`](data/r142-ps3-board-raster-writer.json) `census`):

| Tree | Topology | Packages | Board materials → edges | Board rasters | Formats | Materials per raster | Levels |
|---|---|---|---|---|---|---|---|
| PS3 NHL 14 / 15 / Legacy (each) | loose + `cacherender.big` pair | 67 | 243 → 243 | 139 | DXT1 138, DXT3 1 | 1: 51 · 2: 73 · 3: 14 · 4: 1 | 9: 1 · 10: 69 · 11: 69 |
| PS3 NHL 13 | archive-only `cacherender.big` | 61 | 232 → 232 | 132 | DXT1 131, DXT3 1 | 1: 48 · 2: 69 · 3: 14 · 4: 1 | 9: 1 · 10: 66 · 11: 65 |
| PS3 NHL 12 | archive-only `cacherender.big` | 57 | 226 → 226 | 127 | DXT1 126, DXT3 1 | 1: 44 · 2: 68 · 3: 14 · 4: 1 | 10: 62 · 11: 65 |
| PS3 NHL 11 | archive-only `stadium.big` | 51 | 137 → 136 | 112 | DXT1 112 | 1: 91 · 2: 18 · 3: 3 | 3: 1 · 8: 2 · 10: 55 · 11: 54 |

- **Admitted:** DXT1, and DXT3. DXT3 has one raster per title (`adbd_staples`, 512 × 128, 10 levels, where present), exercised in §4.
- **Not admitted:** DXT5. No PS3 board-ad raster uses it, so there is no sample to test. Refuse `83f3` for boards until a sample exists.
- **NHL 11 refusal:** `stadium_602_container_0` `adbd_41` has slot word `0x400000`, which is not a chunk index. It is refused by
  check 3, and its material has no raster edge.
- **NHL 11 short chains** (3 and 8 levels; `adbd_4` 128 × 256 has 3) follow the same rule, because the size check uses the
  descriptor's level count.
- **No board material had a short block** on any PS3 tree. The short-block refusal is exercised synthetically only.
- **Cube maps** (the 11 NHL 11 entries of 6 × chain, handoff 03 §9) are not board edges. Check 6 refuses them.
- **Non-board materials** (power rings, glass, jumbotron) are not offered by this contract.

**Encoder route.** The existing Studio encoder is `tdb-archive-codec::compress_bc` (`texpresso` Bc1 / Bc2 = DXT1 / DXT3). Research
did **not** run it.
- The probe wrote format-valid **solid** blocks, a different colour and alpha per level.
- They were decoded by Pillow's BCn decoder, which is independent of the writer.
- This tests the byte ranges and level placement, not encoder quality. The developer's receipt should decode its `compress_bc`
  output the same way, level by level.

## 4. Full-chain round trips on disposable copies

One sample per admitted format and topology, chosen as the smallest multi-level raster, preferring one shared by several materials:

| Tree / topology | Package (source SHA-256) | Raster | Materials sharing it | Result |
|---|---|---|---|---|
| PS3 Legacy, pair | `stadium_321_container_0.rpsgl` loose `25ee59f4…`; member `cacherender.big` stored `59b3f9eb…` | desc 124 → entry 123, DXT1 256 × 512, 10 levels | `adbd_sign_mult_Mat1_Clone_178`, `adbd_placeholder1_Clone_393`, `adbd_placeholder` | all checks pass |
| PS3 Legacy, pair | `stadium_13_container_0.rpsgl` loose `f9df11e6…`; member stored `7750dd06…` | desc 30 → entry 29, DXT3 512 × 128, 10 levels | `adbd_staples` | all checks pass |
| PS3 NHL 13, archive-only | `stadium_321_container_0.rpsgl` member stored `262fc778…` | desc 124 → entry 123, DXT1 256 × 512, 10 levels | the same three | all checks pass |
| PS3 NHL 13, archive-only | `stadium_13_container_0.rpsgl` member stored `d6e4c8ec…` | desc 30 → entry 29, DXT3 512 × 128, 10 levels | `adbd_staples` | all checks pass |

"All checks pass" means every one of the following held:
- the stock entry decodes level by level;
- the whole chain was replaced;
- the file length is unchanged;
- bytes outside the entry are unchanged, and 0 .. `hdr_size` is unchanged;
- every checked edge resolves as before, including all sharing materials;
- **every level** (256 × 512 down to 1 × 1; 512 × 128 down to 1 × 1) decodes to its own level colour, and for DXT3 its alpha;
- restore by byte copy gives the source SHA-256;
- the copy was deleted.

**Container probe (research-only format round trip, not a stager):**
- For both pairs, the unwrapped archive member equals the loose file byte for byte.
- A Python zlib-9 native chunkzip framing, mirroring `compress_chunkzip_native`, reproduces **all four stock members byte for byte**.
- The mutated package re-wraps and decodes back exactly.
- The **stored member length changes**: −44 848 / −5 024 bytes (Legacy), −45 248 / −5 024 bytes (NHL 13).
- So the archive copy can never be patched in place. It goes through Studio's archive stager, which re-plans the archive.

## 5. Source identities and failure invariants (developer transaction; research states them, does not implement them)

**Identities bound at plan time and re-checked at apply:**

| Topology | Identities |
|---|---|
| **Pair** (PS3 NHL 14 / 15 / Legacy) | loose package path + SHA-256; containing archive path + SHA-256 (or the stager's archive identity); member path; member wrapper (chunkzip) + stored SHA-256; unwrapped SHA-256 = loose SHA-256 (required; else refuse as a diverged pair); selected material(s) → descriptor → entry; format / w / h / levels; replacement image hash + generated chain hash; original entry bytes (or the whole original files) required for restoration |
| **Archive-only** (PS3 NHL 11 / 12 / 13) | archive + member + wrapper + stored / unwrapped SHA-256; the same raster / replacement / original identities. **No loose counterpart is invented**, and a loose file of that name, if one appears, is a different topology: refuse |
| **Unmeasured** (X360 NHL 10 BIG4) | refuse ([09](09-after-receipt5-corrections.md) K10) |

**Failure invariants:**
- **Pair:** both reviewed outputs persist, or both originals remain recoverable. A partial state, with one copy new and one old,
  is never presented as success.
- No unrelated archive member changes.
- A stale, missing or replaced source (loose or member), or a diverged pair, refuses before any write.
- A compressed-size change goes through a real archive stager. No in-place member patch.
- **Unknown copy precedence is not evidence that one copy suffices.** Updating only the loose file, or only the member, is not
  offered.
- Recovery material is kept on any rollback failure (compare receipt-60 F1, receipt 61).
- Research's copies prove format round trips only. They say nothing about the safety of Studio's transaction or the game's behaviour.

## 6. Not covered, with gates kept explicit

- **X360:** the tiled base plus `LargeLevels` gate, the kept packed tail, and chunklzx re-wrap (card R30-LZX-1) are unchanged from
  03 §5. None of this PS3 evidence carries over. A partial-mip X360 replacement is not "complete".
- **X360 NHL 10:** its RefPack / BIG4 topology is unmeasured.
- **DXT5 boards** (none exist), non-board materials, and resizing.
- Copy precedence, runtime display, and any game launch.

## 7. Developer-consumable item (queue C37)

A PS3-only board-ad replacement behind this contract:
- DXT1 / DXT3, same size, full chain;
- the §1 validator;
- pair and archive-only topologies through the existing archive stager;
- refusals as §1 / §3 / §5.

Tests to write:
- the 14 synthetic cases;
- one pair and one archive-only disposable copy per format, with a level-by-level independent decode;
- the restore hash;
- a diverged-pair refusal;
- a stale-member refusal.

**Coordinator review comes first.**

## 8. Revision 15:13 — admission validator corrected after the receipt-6 review (R142b; supersedes §1 for writer admission)

The coordinator's [receipt-6 review](../../validation/research-20260928-receipt6-review.md) replayed the 14 R142 vectors (all pass) and
found three admission faults in `board_edges_checked`:
1. **Overlap.** It compared only neighbouring sorted spans. An enclosing span could hide an overlap with a non-adjacent one.
2. **DXT5.** It admitted DXT5, which §3 excludes.
3. **Zero values.** It normalised zero width and zero level count to one.

All three are confirmed and fixed in a separate probe. §1 stays as the R142 **discovery** description; writer admission is now §8.1.
Claim `research-q10-c37-validator-correction`.

**Probe:** `local/runs/overnight-research-2026-09-28/probes/r142b_c37_admission_validator.py` (SHA-256 `9709b0f7…`). It imports R142
(`d7ee2d48…`, unchanged) for the parser and the synthetic builder.

**Data:** [`data/r142b-c37-admission-validator.json`](data/r142b-c37-admission-validator.json) (`4c2343fa…`).

**Command** (one run of about 2 minutes, read-only):
```
python -B local/runs/overnight-research-2026-09-28/probes/r142b_c37_admission_validator.py docs/research/overnight-2026-09-28/data/r142b-c37-admission-validator.json --census
```

### 8.1 Writer admission (`admit`), in order

1. **R142 checks 1–3, unchanged:**
   - predecessor `0xEF0001` inside the header region, at least 260 bytes long;
   - the slot word names an in-range `0x20003` descriptor of at least 0x38 bytes, inside the header region.
2. **Format words:** u16 @34 must equal u16 @38.
3. **Format:** `83f1` (DXT1) or `83f2` (DXT3). `83f3` (DXT5) is identified by `discover` but **refused** for writing, and so is any
   other format.
4. **Positive dimensions:** width > 0 and height > 0. Width and height ≤ 8192. The 8192 cap is a defensive bound, not a measured
   limit; the measured maximum is 1024.
5. **Level count:**
   - more than 0;
   - at most `1 + floor(log2(max(w, h)))`;
   - the chain is computed **exactly as declared**. A partial chain is kept, and a full chain down to 1 × 1 is not required.
6. **Entry word:** names an in-range `0x10030` data entry that lies inside the file.
7. **Entry size:** equals the declared plain chain, with one-block floor per level. Cube entries (6 × chain) are refused.
8. **Overlap, pairwise:** the entry's span `[s, t)` is refused if it intersects **any** other data entry of non-zero size
   (`a < t and s < z`). This covers:
   - enclosing spans, non-adjacent spans, equal starts and duplicates are refused;
   - spans that only touch at a boundary are allowed;
   - empty spans are ignored.
9. **Ownership:** exactly one descriptor owns the entry.

`discover` stays separate. It reports every board material's format (DXT1 / DXT3 / DXT5 / other) for census and preview, and
admits nothing.

### 8.2 Vectors: 29 / 29 pass

- **The 14 R142 vectors keep their outcomes.** The short-block correction and every refusal are preserved. The format-6007 reason
  now reads "not admitted for writing".
- **15 new vectors:**

| Case | Expected | R142b | R142 (old) |
|---|---|---|---|
| review's enclosing construction (`synth(a_off=0x80, b_off=0)`, chunk 5 → data `(0x40, 0x10)`, chunk 6 size `0x180`) | refuse | refuse (intersects entry 6) | **edge** (fault 1) |
| equal start, different size | refuse | refuse | refuse |
| duplicate span | refuse | refuse | refuse |
| partial overlap on the right | refuse | refuse | refuse |
| boundary-touching disjoint span | edge | edge | edge |
| empty data span inside the selected span | edge | edge | edge |
| DXT3 8 × 8, 2 levels (80 B) | edge | edge; discovered DXT3 | edge |
| DXT5 8 × 8, 2 levels (80 B) | refuse | refuse; **discovered DXT5** | **edge** (fault 2) |
| zero width (review's `a_size 24`) | refuse | refuse | **edge** (fault 3) |
| zero height | refuse | refuse | **edge** |
| zero level count (review's `a_size 32`) | refuse | refuse | **edge** (fault 3) |
| level count 5 for 8 × 8 (4 possible) | refuse | refuse (level bound) | refuse (size only) |
| format words differ (83f1 / 83f2) | refuse | refuse | refuse |
| declared partial chain, 2 of 4 levels | edge | edge | edge |
| declared single level (32 B) | edge | edge | edge |

### 8.3 Census replay (read-only)

This shows that the new checks keep the measured legitimate inputs.

| Tree | Edges R142b / R142 | Edge-set changes | Refusals | Levels per edge |
|---|---|---|---|---|
| PS3 NHL 14 / 15 / Legacy | 243 / 243 each | none | none | 9: 1 · 10: 134 · 11: 108 |
| PS3 NHL 13 (archive-only) | 232 / 232 | none | none | 9: 1 · 10: 128 · 11: 103 |
| PS3 NHL 12 (archive-only) | 226 / 226 | none | none | 10: 118 · 11: 108 |
| PS3 NHL 11 (archive-only) | 136 / 136 | none | `adbd_41` slot word (unchanged) | **3: 1** · 8: 2 · 10: 56 · 11: 77 |

- Discovery found no DXT5 board material on any tree.
- The **NHL 11 three-level chain** (`adbd_4`, 128 × 256) is admitted as declared.
- No installed board edge relied on the faulty checks, so this is a defensive correction.

### 8.4 Evidence classes, unchanged

- The four full-chain round trips (§4) are **research evidence** on disposable copies with solid blocks. They are not Studio encoder
  tests, not transaction tests and not recovery tests.
- The developer still owns:
  - an independent level-by-level decode of `compress_bc` output;
  - paired-output transaction and rollback tests;
  - archive-stager tests.
- **C37 remains for coordinator review.** Research does not authorise or implement the writer. X360 gates, X360 NHL 10 topology and
  archive re-keying stay withheld.

# Handoff R2 — member-size changes across the measured container families: what a safe repack must patch, preserve and refuse

**Rev. 2 (2026-09-19, correction checkpoint after [overnight-research-2026-09-19-review](../../validation/overnight-research-2026-09-19-review.md) F1/F2/F3).**
Measured data is unchanged; conclusions marked **[superseded rev. 2]** below are withdrawn and replaced by the audit in
[`data/r2-fit-audit.json`](data/r2-fit-audit.json) (probe `r2_fit_audit.py`) and [`data/r2-anomaly-hole-scan.json`](data/r2-anomaly-hole-scan.json)
(probe `r2_hole_scan.py`). Summary of the corrections: (a) the claim that *no* RefPack replacement fits its slot is wrong — joined to
the measured slots, **6 of 38** unchanged-content Studio re-encodes fit (3 X-A outer packages, 3 inner `leaguelogossmall` textures)
and 32 do not; placement is decided per actual encoded replacement against the measured slot and its verified-zero gap bytes, never
from an encoder ratio; (b) the declared-size anomaly population is **four** archives (NHL 15 `game20.big`, `inxp20.big`,
`patch_inxp00.big`, NHL 16 `game_90.big`), not ten — every hole in those four archives was now read in full and is entirely zero
(68 756 holes, 11.6 MB), which is evidence about those four files only; (c) gap bytes are never free capacity unless read and verified
zero for that slot, and unexplained non-zero bytes are preserved or the operation refused/relocated; (d) the package population is
129 rows / 102 distinct byte contents, not 116; (e) cards R2-BIG4-1 and R2-LZX-1 are rewritten with bounded readings. See §11 for the
exact change list.

Research lane, 19 September 2026 (queue entry R2 of the [overnight research brief](../../plans/overnight-research-2026-09-19.md)).
Claim `research-r2-packaging`. Consumer: developer **P1-F** (logo archive placement when the member length changes),
P1-E (logo replace), P1-BS-4 / D1 (text member re-encode), later every asset step that rewrites a compressed member.
Reuses handoffs [16](../overnight-2026-09-18/16-league-logo-packaging-cross-platform.md) (chains X-A/B/C, P-A/B/C, S-A/B),
[18](../overnight-2026-09-18/18-rw4ps3-texlib-layout.md), [20](../overnight-2026-09-18/20-dictionary-growth-invariants.md) §6,
[40](../overnight-2026-09-18/40-chunklzx-framing.md) and [01](01-ps4-settings-csv-defaults-parity-and-packaging.md) §5 (D1/D2).
Nothing here is game-verified; no install, archive, image or anchor was written; no game launched.

Machine-readable (under [`data/`](data/)):
- [`r2-packaging-survey.json`](data/r2-packaging-survey.json) — **authority for §1–§4**: 9 BIG4 archives with their Viv4
  `.bh` sidecars, 118 EB archives (Xbox 360 11–Legacy, PS3 12/15/Legacy, every `databigs/*.big` of the seven PS4
  installs), 129 inner league-logo BIGF package rows = 102 distinct byte contents (indices 0/1/5 per chain; rev. 2 — rev. 1 said 116), 250 sampled stock RefPack members, 240 sampled
  chunkzip members with a native zlib-9 re-encode.
- [`r2-encoder-vectors.json`](data/r2-encoder-vectors.json) — **authority for §5**: 38 RefPack + 30 chunkzip stock members
  re-encoded with Studio's own `tdb-archive-codec` (`refpack_compress`, `compress_chunkzip`) — identities, lengths,
  hashes, byte identity and round-trip results.

Probes (`local/runs/overnight-research-2026-09-19/probes/`): `viv4_index.py` (new: `.bh` reader + cross-check),
`pkg_size_change_survey.py` (new; imports the 2026-09-18 `eb_list`/`big4_list`/`refpack` readers). The encoder comparison
used a throwaway Rust binary linking the working-tree `tdb-archive-codec` crate (built in the session scratchpad,
deleted; the extracted vector members were deleted after hashing).

**Evidence vocabulary:** **observed** = read from the named files; **inferred** = a reading whose producing mechanism is
not traced; **unknown** = not established. Every "the game accepts …" statement is unknown throughout; this handoff
makes packaging *structurally* deterministic and names the runtime cards that remain.

## Plain-language summary

When a replaced texture or text file compresses to a different length, the archive around it has to change, and each
archive generation has its own set of words that must be patched and its own rules about where a member may sit. This
handoff measured those rules on every installed family. The most consequential findings: (1) the early Xbox 360 / PS3
titles (NHL 07–10) keep a **second index** beside each archive — the `.bh` file — that repeats every member's offset,
stored size, decompressed size and a path hash, so a size change must patch both files (and on NHL 07 also the copies
of the `.bh` files packed inside `boot.big`); (2) the small logo packages inside the archives have a fixed member
layout with 64-byte alignment, a 16-byte tail that is *not* counted in the package's own size word, and an opaque
8-byte tag that Studio's current package builder does not reproduce — so a "rebuild the package" route would change
bytes it should not touch, while a "shift the members after the texture" route is fully determined; (3) Studio's
RefPack encoder produces streams **0.5–4 % longer than EA's on every one of 38 stock members**; joined to the measured slots
(rev. 2, `r2-fit-audit.json`) 6 of those 38 unchanged-content re-encodes still fit their slot and 32 do not — so a same-content
re-encode *usually* but not always changes placement, and an edited texture compresses to an unrelated length anyway, so placement
is decided from the actual encoded replacement at plan time **[rev. 2: replaces "never fits back in place"]**; (4) EA's chunkzip members on PS3 and PS4 (240/240) are reproduced
byte-for-byte by standard zlib level 9 in 128 KiB chunks, including the rule that a chunk is stored uncompressed when
deflate does not shrink it — the same encoder the settings handoff asked for (D1) serves PS3 and PS4 alike; (5) on
PS4 16–21 almost every league-logo member **shares its bytes** with other paths (its French twin, other league ids,
even region logos), and Studio's stager currently refuses those; the safe rule is "append a private copy and re-point
only the chosen path"; (6) four of EA's own PS4 archives contain appended entries beyond a stale declared size (three
NHL 15 archives *and* one NHL 16 archive), with holes where old payloads were (rev. 2: every hole in those four archives read in full
and found zero) — the shape Studio's append-and-retarget produces — which shows the shipped layout *contains* that shape on those four
files; it is not evidence that the loader reads such entries, nor that every archive of the version tolerates it. Disc images
stay same-length only.

## 0. Sources (observed)

Identity of each archive is recorded in the survey JSON as path + file length + header words + entry count (whole-file
SHA-256 of multi-gigabyte archives was not computed; every vector member carries its own stored and plain SHA-256).

| Family | Archives measured | Version / magic | Count |
|---|---|---|---|
| Xbox 360 BIG4 (X-A) | NHL 07 `data0.big`, NHL 08 `data0.big`, NHL 09 `fe.big`+`boot.big`, NHL 10 `fe.big`+`boot.big` | `BIG4` + sibling `Viv4` `.bh` | 6 |
| PS3 BIG4 (P-A/P-B) | NHL 08 `data0.big`, NHL 09 `fe.big`, NHL 10 `fe.big` | `BIG4` + `.bh` | 3 |
| Xbox 360 EB (X-B/X-C) | NHL 11 `fe.big`; NHL 12/13/14/15/Legacy `cache.big`; Legacy `cacheboot.big` | EB `0x00000400` (16 B rows) | 7 |
| PS3 EB (P-C) | NHL 12 `cache.big`, NHL 15 `cache.big`, Legacy `cache.big`+`boot.big` | EB `0x00000400` | 4 |
| PS4 NHL 15 (S-A) | all 17 `databigs/*.big` of `CUSA00561` | EB `0x00100400` (16 B rows) | 17 |
| PS4 NHL 16–21 (S-B) | all `databigs/*.big` of CUSA01909 / 04326 / 07569 / 11126 / 15439 / 18182 | EB `0x00110400` (20 B rows) | 11 / 12 / 13 / 25 / 13 / 16 |
| Inner packages | `fe/ion/artassets/leaguelogos{,big,small,wide}/l{0,1,5}.big` on 13 sources (Xbox 07–15/Legacy, PS3 08/10/11/Legacy loose + `cache.big`) | BIGF | 129 rows / 102 distinct (rev. 2; rev. 1 said 116) |

## 1. Outer archives — facts a size change must respect (observed unless marked)

### 1.1 BIG4 + Viv4 `.bh` (Xbox 360 NHL 07–10, PS3 NHL 08–10)

| Fact | Xbox 07 / 08 / 09 / 10 | PS3 08 / 09 / 10 |
|---|---|---|
| Header `size_le` (u32 LE @4) == file length | 9/9 archives | ✓ |
| Entry offsets 64-byte aligned | 100 % (3 490 / 14 162 / 9 053 / 11 004 / 144 / 167 entries) | 100 % (13 921 / 9 128 / 11 004) |
| Inter-entry gap | 0–63 B; zero on every *checked* gap (389–2 300 sampled gaps per archive, not all; one 1-byte nonzero gap in NHL 09 `boot.big` — unexplained, preserved). Rev. 2: a gap is reusable capacity only after its bytes are read and verified zero for that slot | 0–63; sampled gaps zero, same rule |
| Index order == offset order; overlaps; duplicate names | ✓ ; 0 ; 0 | ✓ ; 0 ; 0 |
| Index trailer | the 8 bytes after the last name: `L277` `15 05 00 00` (07/08) or `L286` `15 05 00 00` (09/10); `index_size` (@12) = names table + 8; NHL 07/08 then carry a text string in the gap before the first entry (not zero) — **opaque, copy verbatim** | same tags |
| First entry | at the first 64-byte line ≥ `16 + index_size` | same |
| Sidecar `.bh` | `Viv4`, u32 LE size (= its own length), u32 BE count, u32 BE index size (= its own length); then count × 16-byte BE records `[offset][stored size][decoded size or 0][djb2(path)]`; then a 16-byte trailer `[u32 BE = .big file length][12 × 0]` | same |
| `.bh` ↔ `.big` agreement | count equal, offset/size equal on every record (9/9 archives); word 8 = RefPack declared decoded length on every sampled RefPack member (1 490 + 1 462 + 1 492 + 1 494 + 131 + 151 Xbox samples) and 0 on every raw member; word 12 = djb2 of the path (same algorithm as the EB `+12` word; 100 % of 8 311 sampled records) | same (PS3 members are mostly raw: 1 097 / 1 497 / 1 494 raw in 1 500) |
| Embedded `.bh` copies | **NHL 07 only**: `boot.big` carries RefPack'd `data0.bh … data5.bh`, `gamedata/nis.bh`, `gamedata/aidata.bh` whose decoded bytes equal the loose files (8/8). NHL 08/09/10 `boot.big` carry none (magic scan + RefPack decode of every member < 2 MB) | not scanned on PS3 (no `boot.big` copies found by name) |

Handoff 01 (2026-09-18) §container families said "`boot.big` also embeds the `dataN.bh` indices" for the whole BIG4
era; measured, that holds for **NHL 07 only** — correction recorded in the atlas.

### 1.2 EB `0x00000400`, 16-byte rows (Xbox 360 NHL 11–Legacy, PS3 NHL 12–Legacy)

`archive_size == file length` on 11/11; payloads 16-byte aligned, table order == offset order, gaps 0–15 zero-filled,
no shared spans, `word_4 == 0` everywhere, `+12` word = djb2(path) on 100 % of 117 k rows; the file ends 0–15 bytes
after the last payload (i.e. `archive_size = align16(last end)`). Everything `stage.rs` §"Layout facts" states holds
on these 11 archives (it was measured on three).

### 1.3 EB `0x00100400`, 16-byte rows (PS4 NHL 15, EB-V1)

17 archives: no shared spans; **table order ≠ offset order on every archive** (sorted by the hash word, as on
`0x00110400`); payloads 16-aligned; gaps 0–15 zero-filled except the holes below. **Declared-size anomaly on 3/17**
(`game20.big` −19 757 056 B, `inxp20.big` −8 445 696 B, `patch_inxp00.big` −178 432 B): in each, the entries beyond
the declared size (124 / 39 / 1, all `*.gnf` under `rendering/stadiums/common/`) **start exactly at the declared
`archive_size`** and are packed 16-aligned up to the exact end of the file; each of these archives also contains
large holes inside the declared region (up to 2.25 MB). Rev. 2: **every** hole of the three archives was read in full
(`r2-anomaly-hole-scan.json`: 24 853 / 1 308 / 526 holes, 5.26 / 3.06 / 0.08 MB, 0 non-zero bytes); the original "heads of all
17 holes are zero" was a partial read. The other 14 have `archive_size == length`.

### 1.4 EB `0x00110400`, 20-byte rows (PS4 NHL 16–21)

90 archives: `archive_size == length` on **89**; **NHL 16 `game_90.big` is the fourth and only `0x00110400` anomaly**
("tenth" in rev. 1 was wrong — the distinct declared-size mismatches across all 118 EB archives are exactly four, §1.3 + this one):
declared 7 414 739 808, length 7 431 532 384; the three entries beyond (`jersey_nineties_japan_away_{cm,nm,coeff}.gnf`) start
exactly at the declared size, followed by a 2 798 848-byte hole of exactly the first appended member's size (rev. 2: read in
full, all zero; all 42 069 holes of this archive are zero).
`stage.rs` refuses this archive today (`ArchiveSizeMismatch` is raised for any version other than `0x00100400`) — the
"required to equal" comment in `BoundSource::bind` is falsified by one shipped archive; the rule that *is* consistent
with all **four** anomalous archives (NHL 15 ×3 + NHL 16 ×1) is "entries beyond the declared word start at the declared word and
end at the file length". Whether the loader reads that word is unknown (the shipped titles contain it and the entries
are referenced by path, which is consistent with, not proof of, the loader ignoring it). Rev. 2: one shipped specimen per
version does not make every mismatched archive of that version acceptable; a bounded preserve-length operation on such an
archive needs the full span/overlap check of *that* archive first (§4.1).

Other `0x00110400` facts (observed on all 90): shared spans are the norm (dedup rows: NHL 16 9 724 … NHL 20 27 872
across the install); table order ≠ offset order everywhere; `word_4 == 0`; `word_16` = djb2(path) 100 %; large zero
holes with `archive_size == length` exist too (NHL 19 `game_90-008.big` 3.07 MB, `patch_game_91/93.big` 645 KB, NHL 21
`game_90.big` 832 KB) — dead space is a normal feature of shipped archives.

**League-logo members on S-B are shared spans** (measured on each title's `inxp_10*.big`): NHL 16 12/30, NHL 17 96/104,
NHL 18 114/124, NHL 19 130/140, NHL 20 132/140, NHL 21 134/140 members share their bytes with at least one other row —
the `_fr` twin (`l10.gnf` = `l10_fr.gnf`), other league ids (`l100`/`l100_fr`/`l5`/`l5_fr`) and **non-logo paths**
(`l11` = `regionslogosbig/region-0.gnf`, `l12` = `region-1.gnf`; 10 of NHL 21's 64 groups contain a non-logo path).
Slot slack of every logo member: 0–15 B.

## 2. Inner packages (BIGF, the `l{n}.big` payload) — exact layout (observed on all 116)

| Property | X-A (Xbox 07–09) | X-A NHL 10 | X-B / P-B (Xbox 11, PS3 10/11) | X-C / P-C (Xbox 12–Legacy, PS3 12–Legacy) | P-A (PS3 08) |
|---|---|---|---|---|---|
| Members | 4: `0`, `1`, `<n>`, `<n+1>` | 4 | 6: `0`, `<n+1>`, `sg1`, `<n>`, `sg2`, `1` | 6 | 6 |
| `data_offset` (@12) | 64 (= 16 + table 40 + trailer 8) | 64 | 88 / 90 (= table end + 8) | 88 / 90 | 88 |
| Index trailer (8 B after the last name) | `L277 15 05 00 00` | `L286 15 05 00 00` | `L287 15 05 00 01` (Xbox 11) / `L286 …` | `L286 15 05 00 01` | `L277 15 05 00 00` |
| Gap `data_offset → first member` | zero | zero | zero | zero | **non-zero bytes** (opaque) |
| Member placement | `0` @64; `1` @align64(end); `<n>` @**4096**; `<n+1>` immediately after `<n>` | same | `0` @128; `<n+1>` @align64; `sg1` (0 B) @align64; `<n>` @ **sg1 + 64**; `sg2` @align64(tex end); `1` @ **sg2 + 64** | `0` @128; `<n+1>` @align64; `sg1` @align64; `<n>` @ **sg1** (same offset); `sg2` @align64(tex end); `1` @ **sg2** | as X-C |
| `size_le` (@4, LE) | == package length | == length − 16 | == length − 16 | == length − 16 | == length |
| Tail after the last member | none | 16 zero bytes (not counted in `size_le`) | 16 zero bytes | 16 zero bytes | none |
| Texture member | raw texlib (fixed size per dims) | raw | RefPack, flags `0x10` | RefPack `0x10` | raw `RW4ps3` |
| Outer wrapper of the whole package | RefPack `0x10` (whole package) | RefPack `0x10` | none (EB member / loose) | none | none |

So the two zero-length `sg` markers occupy a 64-byte line of their own on X-B/P-B and none on X-C/P-C/P-A — the
**stride is derivable from the source package** (`offset(<n>) − offset(sg1)` ∈ {64, 0}) and must be reused, not
assumed per era.

**Studio's `ArchiveBuilder::build` (`tdb-archive-big/src/lib.rs`) does not reproduce this layout** (observed in source):
it packs members with no alignment, sets `data_offset = 16 + table` (no 8-byte trailer), and writes
`archive_size = payload_end + eof_padding` where EA writes `payload_end` alone on NHL 10+. Rebuilding a stock package
through it changes header words, member offsets and the trailer even when no member changes. The route for P1-F is a
**span-preserving inner re-plan** (§4.2), not the builder.

## 3. Compression wrappers (observed)

### 3.1 RefPack

- Every sampled stock RefPack member (250 archive members on Xbox 07/08/09/10 and PS3 10, plus every inner texture
  member and every X-A outer wrapper of the 116 packages) uses flags **`0x10`** (3-byte big-endian decoded size, no
  compressed-size field), produces exactly the declared length, and **ends exactly at the terminator: 0 trailing bytes
  after the stream in every case** (rev. 2: that is 250 archive members + 94 inner texture streams + 29 outer package wrappers = 373; rev. 1 wrote "250 + 116"). "Pad with zeros after the terminator to the original length" (handoff 16 §3) is
  therefore a construction no stock member exhibits; whether a decoder tolerates it is unknown. It is not needed at the
  EB level (fit-in-slot zero-fills the slot and patches the `size` word) and should not be used inside BIGF packages
  (the inner `size` word would then disagree with the stream, which stock never does).
- 3-byte sizes cap the decoded length at 16 777 215 B; a larger payload would need flags `0x90` (4-byte sizes) — not
  observed anywhere; refuse rather than emit it.

### 3.2 chunkzip (PS3 12–Legacy `cache.big`/`boot.big`; PS4 15–21)

240 sampled members (40 each on PS3 NHL 12 / 15 / Legacy ×2, PS4 NHL 15 `inxp00.big`, NHL 21 `inxp_10.big`): header
window **0x20000** on all, chunk flag 1 on all 442 deflate chunks, **0 trailing bytes** after the last chunk on all,
and a native-framed **zlib level 9 (raw deflate, window −15) re-encode is byte-identical on 240/240**. The two stored
chunks Studio's codec docs mention (NHL 21 `overlays_21.big` `guestinthebooth*.gfx`, flag 4) follow the rule
**store the chunk when `len(deflate9(chunk)) ≥ len(chunk)`** (6 844 ≥ 6 839, 35 135 ≥ 35 120); with that rule both
members re-encode byte-identical as well. This extends handoff 01's D1 finding (53/53 NHL 16–21 CSV members) to PS3
and NHL 15 PS4: **one native encoder serves every chunkzip family**, with the same status as in handoff 01 rev. 2 —
a reproduction target with recorded backend (CPython 3.12.10 / zlib 1.3.1), not a loader rule.

### 3.3 chunklzx (Xbox 360 NHL 11–Legacy)

Read side pinned by handoff 40. Write side: no LZX encoder exists in Studio or in this lane; the only offline route is
the `chunkzip` substitution `compress_chunkzip` already offers. Whether the Xbox loader accepts a `chunkzip` member
where it shipped `chunklzx` is **unknown** and cannot be settled statically → card R2-LZX-1 (§8). Until then every
chunklzx member is browse-only for size-changing edits.

## 4. What a size-changing replacement must patch — per chain (the preservation mask)

Notation: `L` = new stored member length, `S` = old stored length, `slack` = bytes to the next entry's start.

### 4.1 EB `0x00000400` / `0x00110400` member (X-B, X-C, P-C, S-B): already implemented by `stage.rs`

| Case | Patched | Preserved verbatim | Status |
|---|---|---|---|
| `L ≤ S + slack` (fit-in-slot) | payload bytes, zero fill to the old end, row `size` | every other byte, incl. the row's `word_4`/`+12`/`word_16`, header, name/dir tables | impl (`Placement::FitInSlot`) |
| `L > S + slack` (append) | payload at `align16(EOF)`, row `block` + `size`, header `archive_size` (u64 @0x18) | old bytes left in place (dead), all other rows | impl (`Placement::AppendAndRetarget`) |
| Shared span (S-B logos, §1.4) | **must be append-and-retarget for the chosen row only**; fit-in-slot would rewrite the siblings' bytes | siblings' rows and bytes | **missing** — `stage::plan` returns `StageError::SharedSpan`; needs the "private copy" rule plus a review line naming every sibling path |
| Whole group replace (all paths of a dedup group) | fit-in-slot allowed if `L ≤ S + slack`, all rows of the group keep the shared block; else append once and re-point every row of the group | — | missing (optional; the user must choose it explicitly) |
| `archive_size ≠ length` on `0x00110400` (NHL 16 `game_90.big`) | as `0x00100400`: preserve the declared word, never change the length (append refused); rev. 2: only after a full span/overlap/hole check of the specific archive (`r2-anomaly-hole-scan.json` supplies it for this one file), not as a blanket acceptance of the version | — | **missing** — refused as `ArchiveSizeMismatch` today |
| `archive_size ≠ length` on `0x00100400` (3 NHL 15 archives) | fit-in-slot only; length never changes | declared word | impl (refuses length change with `DeclaredSizeBlocksGrowth`) |

### 4.2 Inner BIGF package (X-B, X-C, P-B, P-C; P-A same-dims only): **span-preserving re-plan** (missing)

Input: the source package bytes, the texture member name `<n>`, the new texture member bytes `T'` (RefPack stream or raw).

1. Keep bytes `[0, data_offset)` verbatim except the affected table words: the `<n>` entry's size, and the
   offset/size words of every member whose offset is **after** `<n>`'s (i.e. `sg2` and `1`); `size_le`.
2. Members before `<n>` (`0`, `<n+1>`, `sg1`) and the gap bytes between them: verbatim (P-A's non-zero gap included).
3. `<n>` at its original offset, length `L = len(T')`.
4. `stride = offset(<n>) − offset(sg1)` from the source (64 or 0; refuse any other value). Then
   `sg2 = align64(offset(<n>) + L)`, `1 = sg2 + stride`, member `1` bytes verbatim. Rev. 2 (F2): gap bytes are carried from
   the source where the source gap is preserved and verified zero where a new gap is created; if any source gap that the
   re-plan would overwrite or shorten holds non-zero bytes (P-A packages do, §2), refuse rather than erase them. When
   `L ≤ S + gap` **and** the gap bytes are verified zero, the texture may be replaced in place with no package growth
   (`r2-fit-audit.json` shows three such unchanged-content cases).
5. `size_le = end(1)` when the source had a 16-byte tail (`size_le == len − 16`), else `end(1)` with no tail (source
   `size_le == len`); append the tail exactly as the source had it (16 zero bytes or none).
6. Output length is a multiple of 16 whenever the source's was (all gen7 packages: `1` ends on a 32-byte boundary and
   the tail is 16).

The 288-byte `<n+1>` record is untouched for same-dimension replacement (handoff 16 §2); its four size floats are the
only thing a dimension change adds. X-A packages (4 members, texture at 4096, record immediately after) need no inner
re-plan for same dimensions (raw texlib, fixed size) — only the outer RefPack changes length.

### 4.3 BIG4 + `.bh` (X-A, P-A, P-B): outer placement (missing)

Entry `i` with `S`, `slack ∈ [0, 63]`, new stored length `L`:

| Case | `.big` patched | `.bh` patched | Preserved |
|---|---|---|---|
| `L ≤ S + slack` (fit-in-slot; rev. 2: `slack` counts only bytes read and verified zero for this slot — a non-zero gap byte is preserved, so the usable slack ends before it) | payload, zero fill to the old slot end, directory entry `size` (u32 BE) | record `i` word 4 (size) and word 8 (decoded length; 0 for raw) | `size_le`, `index_size`, trailer, every other entry/record, `.bh` trailer, `.bh` `size_le` (its own length is unchanged) |
| `L > S + slack` — **append-and-retarget** (structurally valid; **not observed** in stock, where index order == offset order on 9/9) | payload at `align64(EOF)`, entry `offset`+`size`, header `size_le` = new length | record `i` words 0/4/8, trailer word 0 = new `.big` length | old bytes left in place; all other records |
| `L > S + slack` — **ordered reflow** (stock-shaped: every later entry shifts by `align64(L) − align64(S)`) | every later entry's `offset`, header `size_le`, the whole tail rewritten (hundreds of MB on `fe.big`) | every later record's word 0, trailer | gap bytes (copy each entry with its own trailing padding) |
| NHL 07 `data0.big`–`data5.big` | as above **plus** re-encode the changed `.bh` with RefPack and place it in `boot.big` (a second, cascading replacement — the `.bh` bytes change so the RefPack stream length changes) | — | refuse in the first implementation unless the cascade is planned as one transaction |
| Decoded length > 16 777 215 or `L`/offset > u32 | refuse | | |

Which of append vs reflow the loader tolerates is unknown (card R2-BIG4-1). Both keep `.bh`/`.big` consistent; the
reflow is the only shape stock exhibits.

### 4.4 Disc images and loose files

Disc images (`iso_patch`): same-length only, unchanged. PS3 12–Legacy loose `USRDIR/fe/ion/artassets/**` files: the
package is the file — write the re-planned package; the `cache.big` copy of the same bytes must be replaced through 4.1
in the same transaction or the discrepancy disclosed (precedence unknown, handoff 16 §4). NHL 11 PS3 is loose-only.

## 5. Encoder provenance — Studio's encoders against stock bytes (observed; `r2-encoder-vectors.json`)

| Encoder | Vectors | Byte-identical | Length vs stock | Round trip |
|---|---|---|---|---|
| `refpack_compress` (Studio) | 38 (12 archive members NHL 09/10 Xbox; 18 inner texture members X-B/X-C/P-B/P-C; 8 X-A outer packages NHL 07–10) | **0/38** | **longer on 38/38**, +15 … +1 023 B (+0.5 … +4.2 %) | 38/38 |
| `compress_chunkzip` (Studio, single chunk) | 30 (PS3 NHL 12/15/Legacy, PS4 NHL 15/21) | 0/30 (header words differ; single chunk) | longer 15, shorter 11, equal 4 | 30/30 |
| zlib 1.3.1 level 9, native framing + store-if-not-smaller (this lane, Python) | 240 + 2 stored-chunk members + 53 (handoff 01) | **295/295** | equal | — |

**[superseded rev. 2]** ~~Consequences: with today's RefPack encoder no same-dimension logo replacement fits its slot on any
RefPack chain — every replacement is a size-changing one until the encoder matches EA's ratio.~~ Rev. 2, from
`r2-fit-audit.json` (each vector joined to its measured slot, gap bytes read and verified zero on all 38 slots): the
unchanged-content Studio re-encode **fits on 6/38** — X-A outer packages NHL 10 / 09 / 08 `leaguelogossmall/l0` (+25 / +15 / +15 B
against 38 / 39 / 30 B of zero slack) and inner `leaguelogossmall` textures on X-B NHL 11 (+19 B vs 60), P-B NHL 10 (+18 vs 61)
and P-C Legacy (+30 vs 42) — and does not fit on 32/38 (15 inner textures whose 64–1 024 B package growth exceeds the 0–15 B
outer slot, 12 BIG4 members with 14–55 B slack, 5 X-A outer packages). Compression of *stock* pixels does not bound *edited*
pixels in either direction, so the planner must offer fit-in-slot whenever the actual encoded replacement fits the verified
slack, and append/relocate otherwise — no encoder-ratio precondition. A better match finder (EA's stream is consistently ~1–4 % shorter, which is what
a longer hash chain / lazy matching typically buys) is a bounded developer experiment; byte identity is not required
for structural validity, only decoded-content preservation (as for chunkzip, addendum 1).

## 6. Supported / refused operations (proposed for P1-F; policy, not evidence)

Supported once §4.1's two missing rules and §4.2/§4.3 exist: same-dimension logo replace on X-B, X-C, P-C, S-B, S-A
(PNG, fit-in-slot or append), plus P-B via handoff 18's texlib writer; append-and-retarget for shared-span S-B members
with every sibling named in the review; BIG4 fit-in-slot (rare) and one of append/reflow on X-A/P-B with `.bh` kept
consistent.

Refused with the reason: chunklzx members (no encoder); NHL 07 `dataN.big` growth unless the `boot.big` cascade is
planned; any BIG4 archive whose `.bh` disagrees with its directory (count/offset/size) or is missing; a package whose
`sg` stride is neither 0 nor 64, whose trailer is not `L2xx`, or whose members are out of the measured order; RefPack
decoded length > 16 MiB; a `0x00110400` archive with `archive_size ≠ length` when the plan would change the length;
a replacement whose independent decode does not equal the intended plain bytes (always, on every chain).

## 7. Developer vectors (offline)

All from the two JSON files; each names archive, entry index, offset, stored size, stored SHA-256, plain SHA-256.

- **V-BH-1..9**: for each of the 9 BIG4 archives, `sidecar_bh` (count, mismatches 0/0, word-8 rule, djb2 hits, trailer)
  — the `.bh` reader/patch test: parse, cross-check against the directory, patch record *i* and trailer for a synthetic
  `L`, re-verify.
- **V-BH-07**: NHL 07 `boot.big` members `data0.bh … data5.bh`, `gamedata/{nis,aidata}.bh` decode-equal to the loose
  files (8/8) — the cascade detection test.
- **V-PKG-\***: 116 packages with full member tables — the §4.2 re-plan test: re-plan with `T' = T` (same bytes) must
  reproduce the package byte-for-byte on all 116 (the identity check that the builder fails); re-plan with a `T'` of
  `len(T) ± 1, ± 63, ± 64` must move only `sg2`/`1`/`size_le`/tail as §4.2 and keep everything else equal.
- **V-RP-\***: 38 RefPack members — encoder acceptance: round trip 38/38; report length delta; target ≤ stock.
  Rev. 2: `r2-fit-audit.json` adds the per-vector slot (`slot_slack`/`inner_slack`, zero-verified) and the expected
  placement — the planner test is "fit-in-slot on the 6 fitting vectors, append/re-plan on the 32 others".
- **V-CZ-\***: 30 + 242 chunkzip members — native encoder acceptance (framing, store rule, decode equality; byte
  identity as the reproduction target).
- **V-EB-ANOM**: NHL 16 `game_90.big` (declared 7 414 739 808 / length 7 431 532 384 / 3 entries beyond starting at
  the declared word) — a candidate for `bind` acceptance under the `0x00100400` rule **after** the per-archive span/hole check
  (`r2-anomaly-hole-scan.json`: 42 069 holes all zero, last end == length, first beyond == declared); NHL 15
  `game20`/`inxp20`/`patch_inxp00` as today. Rev. 2: four anomalies in total; no blanket acceptance of the version.
- **V-EB-SHARED**: NHL 21 `inxp_10.big` group at offset 1 079 203 232 = `size256/l11.gnf`, `size256/l11_fr.gnf`,
  `regionslogosbig/region-0.gnf` — replace `l11.gnf` alone → append-and-retarget for that row only; the other two rows'
  `block`/`size` and the shared bytes unchanged; replace-the-group → one payload, three rows re-pointed.
- Boundary cases: `L == S`, `L == S + slack`, `L == S + slack + 1`; a member at the last table position (slack to
  EOF); an `L` that is not a multiple of 16 (EB) / 64 (BIG4); the `size_le`/tail variant with and without the
  16-byte tail; `.bh` word 8 for a raw member (0) vs RefPack; djb2 recomputation must equal the stored word before any
  patch (else refuse).

## 8. User cards (not authorised here; user execution only)

- **R2-BIG4-1 (NHL 10 Xbox 360, `fe.big`, one league logo) — rev. 2 wording.** Treatment A: append-and-retarget with
  `.bh` patched (index order broken); Treatment B: ordered reflow (stock shape); Treatment B′: B with the `.bh` left
  unpatched; Control C0: stock. Readings are bounded to this title and archive: A shows stock art while B shows the edit →
  the appended entry was not consumed by this screen (placement, caching, `cache:` shadowing or rejection are *not*
  separated by this pair); both show the edit → the edit path worked under both placements for this screen (says nothing
  about other consumers or later loads); both show stock art → the edit was not consumed and **no cause is identified** —
  placement, caching, shadowing (2026-09-18 atlas Q1) and rejection all remain open, so the next discriminating control is a
  fit-in-slot variant of the same member (one of the six fitting vectors of `r2-fit-audit.json`) on the same title. B′
  working shows only that *this* load of *this* archive did not require the patched sidecar; it does not show the sidecar is
  never consulted (other loads, other archives, or a `.bh` that is read only when the sizes disagree remain possible).
- **R2-LZX-1 (NHL Legacy Xbox 360, `cacherender.big`, one rink-logo texlib) — rev. 2 wording.** Step 1 (source-selection
  control): replace the `chunklzx` member with a `chunkzip` re-encode of a *visibly edited* plaintext (same dimensions),
  fit-in-slot if shorter else append; the same edit must first be shown consumed through a `chunklzx`-free path if one exists,
  otherwise pair it with step 2. Step 2: the unchanged plaintext as `chunkzip`. Readings: the edited art renders → the
  `chunkzip` wrapper was decoded by this consumer for this member (this title/archive/class only); unchanged art renders in
  step 2 → **inconclusive on its own** — without the edited control it cannot distinguish "wrapper accepted" from "another
  copy or cache was drawn"; black/missing texture or a crash → the replacement was not usable, which may be wrapper
  rejection, placement, or a bad member, so run the same edit as `chunklzx`-free / fit-in-slot control before concluding
  on the wrapper.
- **R2-EB-1 (NHL 21 PS4, `inxp_10.big`, `l11.gnf` alone).** Private-copy append; then the group variant. A visible
  change on the league logo but not on the region logo → per-row retargeting honoured.

## 9. Unknowns (explicit)

1. Loader tolerance of BIG4 append (non-monotonic offsets) vs reflow; whether `.bh` is consulted at all (per title).
2. Loader tolerance of chunkzip-for-chunklzx substitution on Xbox 360 11–Legacy.
3. Whether the PS4 loader honours `archive_size` (the four shipped anomalies are consistent with "ignored", not proof).
4. EA's RefPack match finder (Studio's output is longer on every vector; identity not required).
5. The one non-zero 1-byte gap in NHL 09 Xbox `boot.big`; the NHL 08 PS3 package pre-member bytes; the `L2xx` trailer
   words; `word_12` on `0x00110400` — all preserved verbatim, none interpreted.

## 10. Reproduce, checks run, cleanup

```
python local/runs/overnight-research-2026-09-19/probes/pkg_size_change_survey.py <out.json> [--only big4|eb|bigf|chunkzip|refpack]
python local/runs/overnight-research-2026-09-19/probes/viv4_index.py "<archive.big>" [--limit N]
```

Executed this session (read-only): the survey in five parts (outputs merged into `r2-packaging-survey.json`; the part
files under `local/runs/overnight-research-2026-09-19/r2-packaging/` are regenerable copies); ad-hoc python for the
`.bh` trailer, the BIG4 index trailer, `boot.big` Viv4/BIG scans (magic + RefPack decode), the NHL 15/16 PS4
declared-size anomalies (hole heads, appended-region accounting), the NHL 21 stored-chunk rule and re-encode identity,
the S-B shared-span groups; a throwaway Rust binary against the working-tree `tdb-archive-codec` for the 68 encoder
vectors (extracted members and build tree deleted after hashing). **Not executed:** any Studio binary or `cargo test`;
any `stage`/`plan` run; any write to an install; any launch.

## 11. Rev. 2 change list (2026-09-19, review F1/F2/F3)

Wording-only; measured data unchanged; two data files added. Applied by `probes/rev2_handoff02_edit.py` (exact record).

1. Header: rev. 2 note and summary of corrections.
2. Plain-language summary (3): "never fits back in place" withdrawn → 6/38 fit, placement per actual encoded replacement.
3. Plain-language summary (6): "zero-filled holes" → holes read in full on the four archives; evidence bounded to those files.
4. §1.1 gap row: sampled-gap wording; gap reusable only after verified zero.
5. §1.3: whole-hole scan replaces "heads of all 17 holes are zero".
6. §1.4: "tenth anomaly" / "all 10 anomalous archives" → four; no blanket acceptance of the version.
7. §4.1 table: NHL 16 `game_90.big` row conditioned on the per-archive check.
8. §4.2 step 4: source-gap preservation, in-place replace when the verified-zero gap suffices.
9. §4.3 fit-in-slot row: slack = verified-zero bytes only.
10. §5 consequences paragraph struck and replaced by the audit result.
11. §7 V-RP / V-EB-ANOM vectors: point at `r2-fit-audit.json` / `r2-anomaly-hole-scan.json`.
12. §8 cards R2-BIG4-1 and R2-LZX-1 rewritten (review F3): bounded readings, no causal exclusion, next discriminating same-title control named.
13. §9 item 3: ten → four.
14. Package population: the "116 measured packages" figure in the header list and §0 table is not reproducible from the array (129 rows, 102 distinct byte contents, 94 with an inner RefPack texture, 29 with an outer RefPack wrapper, 6 raw P-A) — see `r2-fit-audit.json` `package_populations`; the "250 + 116 members" RefPack count in §3.1 likewise reads as 250 archive members + 94 inner + 29 outer = 373 wrapped streams (0 trailing bytes on every one measured). The 116 figure is left in place above as the rev. 1 text and is superseded by this item.

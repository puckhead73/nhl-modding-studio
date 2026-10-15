# Handoff R98 — `.ast` stream framing on the archive-backed gen7 titles (PS3 NHL 09 / 10 / 12 / 13, X360 NHL 09) from their own archive members: 9 987 referenced streams + 28 loop bodies on 51 `.ast` files, 0 refusals, every file byte-accounted, the R91 grammar unchanged on the four PS3 titles; **X360 NHL 09 is a packet-header variant** — sub-blocks tile and bytes account, but the packet header word is not a frame count there (constant `0x08000000` / `0x20000000`), sub-blocks hold up to two packets, and the sample rule is recovered only by walking the frames inside each packet (R98b: first-block delta 384 on 270 / 270, middle blocks exact on 19 138 / 19 225); representative decode NOT attempted (separate probe, not implied)

Research lane, 25 September 2026, 14:00 → 14:35 MDT (real clock). Direction: the 25 September audit's "extend ABKC / AST framing to the
archive-backed titles omitted from the loose-tree census, using their own member hashes and existing source inventories; separate
structural enumeration from representative decode tests; retain unsupported / residual variants". Claim `research-r98-ast-framing-archived`.
Read-only over the five trees on `X:` (archives opened read-only; members decoded in memory; nothing extracted to disk). Nothing
game-verified; nothing listened to; no decode; no writer.

Machine-readable: [`data/r98-ast-framing-archived.json`](data/r98-ast-framing-archived.json) (probe `r98_ast_framing_archived.py`, 21 s;
1 240 inputs hashed — every `.abk` / `.ast` member: stored bytes SHA-256, decoded bytes SHA-256, wrapper), per-stream rows in
`local/runs/overnight-research-2026-09-25/blockmaps/r98-streams-archived.jsonl`; [`data/r98b-x360-09-packet-header.json`](data/r98b-x360-09-packet-header.json)
(probe `r98b_x360_09_packet_header.py`, X360 NHL 09 vs X360 NHL 10 control). R91's grammar, per-stream tests S1–S8, accounting and
controls are **imported unchanged** (`r91_ast_framing.py`), so "framed" here means the same thing as in AST_FRAMING.md.

## Plain-language summary

The stream files of the five older titles that keep their audio inside archives were read straight from those archives, each member
hashed as stored and as unpacked. On the four PlayStation 3 titles the layout is exactly the one measured on the loose titles: every
stream tiles into EA frames, the padding sits only at the end, the per-block sample counts are exact, the loops read as intro plus body,
and the same fifteen corruption controls behave the same way. Xbox 360 NHL 09 is different in one specific place: the four-byte word at
the head of each XMA packet, which on NHL 10 and later tells a decoder how many frames the packet holds, is a fixed value on NHL 09 and
does not count anything, and a packet slot can hold two packets instead of one. Reading through the header naively made the sample
counts look wrong; walking the frames inside each packet instead recovers the same start, middle and end rules as the later titles on
nearly every block. That is a real difference an encoder or decoder would have to know about, so it is recorded as its own class with
its residuals rather than folded into the "same as NHL 10" result. No stream was decoded in this package.

> **Note 2026-09-25 22:07 (R102, [14-correction-checkpoint-r97-r101-review.md](14-correction-checkpoint-r97-r101-review.md) §F3):** the 15:30 note's explanation ("the ffmpeg decoder walks frames by length and ignores the
> header word") is narrowed to the three tested vectors and to the frame-count / metadata bits (R100b). The 87 residual middle blocks,
> the two header values and the 4 loop bodies stay open; decode success on selected streams does not extend structural acceptance.

## 1. Sources (exact members; nothing substituted)

| Tree | `.abk` members (archives) | `.ast` members used (archive → member; wrapper) | copies |
|---|---|---|---|
| PS3 NHL 09 | 186 (`audioboot.big` RefPack, `data0.big` stored) | 7, all `data0.big`, **stored**: colors1 / colors2 / createplayer / crowds / playercalls / practicerink / sfx | one copy each |
| PS3 NHL 10 | 199 (`audio.big`, `audioboot.big`, `audioboot2.big`, `stadium.big`, RefPack) | 3 in `audio.big` (createplayer, practicerink, sfx — stored) + **4 loose only** (`audio/sfx/colors1 / colors2 / crowds / playercalls.ast` — the title ships them loose; no archive copy exists, R82 §3 "exactly one archive or loose") | one copy each |
| PS3 NHL 12 | 219 (`nocache.big`, `audioboot.big`, `audioboot2.big`, chunkzip) | 15, all `nocache.big`, stored (ambientskatesandpunches, 5 × bap_*, colors1 / 2, createplayer, crowds, crowds_winterclassic, playercalls, playercalls2, practicerink, sfx) | one copy each |
| PS3 NHL 13 | 213 (same + `cacheaudio.big`) | 15: 8 in `nocache.big` (ambient, bap_*, createplayer, practicerink) + 7 in `cacheaudio.big` (colors1 / 2, crowds, crowds_winterclassic, playercalls, playercalls2, sfx), stored | one copy each; NHL 13's `crowds.ast` (`3a3a4982…`) ≠ NHL 12's (`b69f5f2d…`); `createplayer.ast` byte-identical on 10 / 12 / 13 (`bc03e437…`) |
| X360 NHL 09 | 372 (`audioboot.big`, `data0.big`, stored) + loose copies | 7 in `data0.big` (stored) **and** 7 loose `audio/sfx/streams/*.ast` — **byte-identical pairs** (7 / 7 same SHA-256); the archive member was used | two identical copies each |

Every `.ast` member is **stored** (no RefPack / chunkzip wrapper on any of the 51); ~~the `.abk` members are wrapped as R82 recorded.~~
**(Corrected 2026-09-26 03:42, developer observation in claim `dev-p48-archive-sound-banks` and R110b:)** the `.abk` members are stored too
(`ABKC` at offset 0 on all 959 bank members; this handoff's own JSON already recorded `stored`); the RefPack / chunkzip labels in the table
above describe the containing archive's music members, not the banks.
Every used `.ast` has `S10S` and an id word equal to its bank's 0x2c word (51 / 51; the copy test is in the JSON).

## 2. Structural result (R91 tests, per tree)

| | PS3 NHL 09 | PS3 NHL 10 | PS3 NHL 12 | PS3 NHL 13 | X360 NHL 09 |
|---|---|---|---|---|---|
| `.abk` members with stream records / `.ast` files | 7 / 7 | 7 / 7 | 15 / 15 | 15 / 15 | 14 (7 archive + 7 loose) / 7 |
| bank tree (R89 `parse_tree`) refusals | 0 | 0 | 0 | 0 | 0 |
| referenced streams framed `ok` (refused / failed) | 506 (0 / 0) | 1 108 (0 / 0) | 3 719 (0 / 0) | 3 708 (0 / 0) | 946 (0 / 0) — 473 distinct (two identical copies) |
| loop records: stub one block, (a, b) = stub words, body unreferenced, stub + body = num_samples, a = stub samples | 2 / 2 | 4 / 4 | 9 / 9 | 9 / 9 | 4 / 4 |
| loop body "fresh start" (block-0 delta = class delta) | 2 / 2 | 4 / 4 | 9 / 9 | 9 / 9 | **0 / 4** (see §3) |
| files byte-accounted (header + blocks + terminal padding + zero gaps = size; unexplained nonzero bytes) | 7 / 7 (0) | 7 / 7 (0) | 15 / 15 (0) | 15 / 15 (0) | 7 / 7 (0) |
| stream ends 0x40-aligned | 508 / 508 | 1 112 / 1 112 | 3 728 / 3 728 | 3 717 / 3 717 | 950 / 950 |
| PS3: EA-frame chains tile every block, zero padding only terminal; MPEG-1 granule index alternates per sub-stream | 506 / 506 | 1 108 / 1 108 | 3 719 / 3 719 | 3 708 / 3 708 | — |
| first-block delta (decodable − declared) | 1152 on 504 (+ the 2 stubs: 2880 / 3456 = whole warm-up) | 1152 on 1 104 (+ 4 stubs 3456) | 1152 on 3 710 (+ 9 stubs) | 1152 on 3 699 (+ 9 stubs) | **−3712 on 942 / 946 under the XMA2 header reading** (§3) |
| middle blocks exact (per sub-stream) | 506 / 506 | 1 108 / 1 108 | 3 719 / 3 719 | 3 708 / 3 708 | **0 / 942 under the header reading** (§3) |
| terminal trim (granules / packets) | 2–4 granules | 2–4 | 2–4 | 2–4 | header reading: −8..+1 packets (meaningless) |
| PCM block frame offset field | 0 on all | 0 | 0 | 0 | — |
| controls (15 named, in memory on a copy of the decoded member) | 14 detected + main-data flip undetected | same | same | same | **13 detected + 2 undetected** (`packet_frame_count_0_metadata_1` not refused — the header is not read that way on NHL 09; packet-body flip undetected as everywhere) |
| channel configs seen | cc 0 / 1 / 3 | cc 0 / 1 / 3 | cc 0 / 1 / 3 | cc 0 / 1 / 3 / 5 (1) | cc 0 / 1 / 3 |

The four PS3 titles: **the R91 grammar holds unchanged** (same first-block delta, exact middle blocks, terminal padding only, loops,
controls). Coverage class per tree = complete enumeration of every stream entry of every bank member with a matching `.ast`.

## 3. X360 NHL 09 — a packet-header variant, established by R98b (not forced into the XMA2 reading)

R91 reads each X360 sub-block as `u32 w0 | one XMA2 packet` and takes the packet's frame count from the header (top 6 bits), which
R84 / R91 verified on NHL 10–Legacy (decoded count = header count × 512; metadata bits = 1 on every packet). On NHL 09 that reading
gives −3712 on the first block and inexact middle blocks. R98b tests three readings on the same bytes (X360 NHL 09 `crowds`,
`playercalls`, `sfx`, `colors1`: 270 non-loop streams, 35 213 packets; control X360 NHL 10 `crowds`, `playercalls`, `sfx`: 94 streams,
24 558 packets):

| Reading | X360 NHL 09 | X360 NHL 10 (control) |
|---|---|---|
| header as XMA2 (`FrameCount 6 | FrameOffsetInBits 15 | Metadata 3 | Skip 8`) | word = `0x08000000` on 35 014 packets, `0x20000000` on 199 (frame count "2" / "8", metadata 0, skip 0, offset 0) — **not a frame count** | frame counts 1–15, metadata 1 on 24 558 / 24 558, offsets vary |
| header as XMA1 (`Sequence 4 | Unused 2 | FrameOffsetInBits 15 | Skip 11`) | sequence never increments (0 / 35 212 consecutive pairs), "unused" bits non-zero on 35 014 — **not XMA1 either** | (not applicable) |
| **frame walk** from `FrameOffsetInBits` (15-bit frame lengths, counting frames that start in each 0x800 packet; per sub-block, per XMA stream) | first-block delta **384 on 270 / 270**; middle blocks exact on **19 138 / 19 225** (crowds 2 519 / 2 529, playercalls 150 / 150, sfx 14 829 / 14 906, colors1 1 640 / 1 640); terminal trim 0–416 samples | walk count = header count on **24 558 / 24 558** packets; first-block 384 on 94 / 94; middle exact 23 631 / 23 631 — the walk is validated on the control |
| packets per sub-block | **1 or 2** (crowds 755 / 1 816, sfx 1 360 / 13 612, playercalls 251 / 1, colors1 1 951 / 19); `w0 >> 2` still = sub-block length | 1 on 24 558 / 24 558 |
| sub-streams (cc 3) agree with each other | 0 disagreements | 0 |

**Class statement for X360 NHL 09 (bounded):** block grammar, sub-block lengths, 0x40 alignment, zero gaps and byte accounting hold
(R98); the packet header word is a constant that does not encode a frame count; frames are located by the offset field and walked by
their 15-bit lengths; with that, the same sample rules as NHL 10–Legacy hold on 270 / 270 first blocks and 99.5 % of middle blocks.
**Residuals, kept:** 87 middle blocks (crowds 10, sfx 77) where the walked frame count × 512 ≠ the samples word — not explained (a
frame spanning the two packets of a sub-block is one candidate; not tested); the two header values (`0x08000000` vs `0x20000000`,
199 packets) — meaning unknown; the 4 loop bodies whose block-0 delta is not the class delta under the header reading (re-test under
the walk not done); the `packet_frame_count_0` control is **not applicable** on NHL 09 (there is no frame-count field to zero).
**Not done:** any decode of an NHL 09 stream — the ffmpeg `xma2` route relies on the header semantics that NHL 09 does not have, so the
R92 route is **not** expected to apply unchanged; a decoder for this class would have to reconstruct packets (or the walk has to feed a
different wrapper). This is listed as a separate dependency, not attempted here.

> **Note 2026-09-25 15:30 (R100, [13-archived-stream-decode-and-kit-name-key-positions.md](13-archived-stream-decode-and-kit-name-key-positions.md) §1):** "Not done: any decode … the R92 route is not expected to apply unchanged" is **withdrawn**: three X360 NHL 09 streams decode through the unchanged R92 XMA route with exact counts (the ffmpeg decoder walks frames by length and ignores the header word); a synthesised-header construction gives identical output. The header variant remains an encoder / header-trusting-parser concern. PS3 09 / 10 / 12 / 13: 15 / 15 sub-streams exact through the EAL3 route.

## 4. What this adds to the ledger, and what it does not

- Adds: framing coverage for the five archive-backed titles from their own members with hashes (PS3 09 / 10 / 12 / 13 complete,
  same grammar; X360 09 complete at block level with a header variant and a working frame walk); the loose-vs-archive identity on
  X360 09 (byte-identical); the loose-only `.ast` files on PS3 10; NHL 13's second audio archive (`cacheaudio.big`).
- Does not add: decoding (none), previews (none), replacement (unchanged: unqualified — REPLACEMENT_READINESS.md; NHL 09's header
  variant is one more element an encoder would have to reproduce), runtime or listening evidence.
- Residual variants retained: X360 09 (§3); cc 5 on PS3 13 (1 stream, framed, de-interleave inferred as in R91).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-25/probes/r98_ast_framing_archived.py docs/research/overnight-2026-09-25/data/r98-ast-framing-archived.json --summary local/runs/overnight-research-2026-09-25/blockmaps/r98-streams-archived.jsonl
python local/runs/overnight-research-2026-09-25/probes/r98b_x360_09_packet_header.py docs/research/overnight-2026-09-25/data/r98b-x360-09-packet-header.json
```
Executed: r98 smoke × 1 (X360 09 only, scratch JSON deleted), full × 1; r98b × 4 (loose-path lookup fix; walk bound fix — a constant
was passed instead of the file size and refused the long crowd streams; packet chunking inside a sub-block; per-sub-block comparison).
The JSONs are the final runs.

# AST_FRAMING — gen7 `.ast` stream-file block payload framing (Q-R89-1), measured on 11 loose trees

Research lane, 25 September 2026 (R91, with R92 decode checks). Read-only over the X360 NHL 10–Legacy and PS3 NHL 11 / 14 / 15 / Legacy
loose trees. Machine-readable: [`data/r91-ast-framing.json`](data/r91-ast-framing.json) (per tree: coverage by class, delta / trim
histograms, per-file byte accounting, controls); per-stream summary `local/runs/overnight-research-2026-09-25/blockmaps/r91-streams.jsonl`
(29 962 rows); full block maps (every block, every EA-frame / XMA sub-block) for 288 representative streams under
`local/runs/overnight-research-2026-09-25/blockmaps/representative/`. Parser: `local/runs/overnight-research-2026-09-25/probes/r91_ast_framing.py`.

**Structural coverage** (this file, §1–§5) is stated separately from **semantic interpretation** (§6): the first is a byte-level
tiling result on every stream; the second is what the sample words and the PCM block frame *mean*, supported by decoder measurements
(R92) but not by any game trace.

## 1. Grammar (derived on PS3 / X360 NHL 15 `playercalls` + `crowds`, then tested on every stream of the 11 trees)

```
.ast     'S10S' | u32 id (== the bank header word 0x2c, R89 T5) | zero bytes to 0x40 | streams …
stream   blocks { u8 flags | u24 size | u32 samples | payload (size − 8 bytes) }        size counts the 8 header bytes; big-endian
         flags: 0x00 = continuation, 0x80 = TERMINAL (last block of the stream); no other flag value exists (0 / 29 962 streams)
         the stream starts at the bank tree's O (0x40-aligned on both platforms, R89) and ends after its 0x80 block
alignment / padding (measured, platform-specific):
   PS3   streams tile contiguously; the terminal block's payload ends in ZERO padding that brings the stream end to the next 0x40
         boundary (every stream end is 0x40-aligned: 12 165 / 12 165 + 31 bodies); the terminal block SIZE is not otherwise rounded
   X360  the terminal block's size is a multiple of 0x40 (17 797 / 17 797); the block's payload ends in zero padding to that size;
         streams are then separated by all-zero gaps to the next 0x40-aligned start (R89: gaps all zero)
payload  by the S10A record's codec word (SNR h1, R82):
   0x05  EALayer3 v1 (PS3, every stream)   = a chain of EA-frames exactly as R77 read the ram segments:
           u8 pcm_flag (0x00 normal / 0xEE PCM block frame) | u8 common (MPEG version 2 or 3, sample-rate index, channel mode, ext)
           | side info (1 bit granule index; MPEG-1 & gi = 1: 4-bit scfsi per channel; per channel 12-bit main-data size + 47 (MPEG-1)
           / 51 (MPEG-2) side bits) | main data (Σ sizes bits, then byte-aligned) [| PCM block: u16 offset | u16 count | u32 0 | PCM]
         a frame cannot start with common byte 0 (MPEG version 0 is reserved) → a zero run is accepted ONLY as the trailing bytes of
         the terminal block; anywhere else it is refused. channel_config 3 / 5 = 2 / 3 interleaved streams, frame i → sub-stream i mod N
   0x03  EA-XMA (X360, every stream)       = sub-blocks { u32 w0 | one XMA2 packet }, w0 >> 2 = sub-block length INCLUDING w0, low 2
         bits recorded (census below); sub-blocks per block = number of XMA streams: 1 for cc 0 / 1, 2 for cc 3, 3 for cc 5 (measured:
         28 798 + … blocks with 1, 742 … with 2, 120 with 3 — never mixed within a stream); zero padding only in the terminal block
   other  none occurs in a loose `.ast` (0 / 29 962) — classified `unsupported-codec` by the parser if ever met
sample words (measured, §6 for meaning):
   middle blocks: samples == frames / N × 576 (EAL3) and == frame_count × 512 (XMA) on 100 % of middle blocks (0 nonzero deltas)
   first block  : decodable − declared = 1152 per sub-stream (EAL3: includes the PCM frame's samples) / 384 (XMA) on every non-loop
                  multi-block stream; single-block streams (first == terminal) show 1152 + trim / 384 + trim
   terminal     : decodable − declared = trim ∈ [1153, 2304] samples (EAL3: 2–4 granules, 4 on 18 streams) / [0, 884] (XMA)
loop records (16-byte S10A record, words a / b — R82): the stream at O is a ONE-block stub with (samples, size) == (a, b) (146 / 146);
   the body is the next stream (PS3 at the stub's end; X360 at the next 0x40 boundary), referenced by no list entry (146 / 146),
   with its own warm-up (first-block delta == 1152 / 384 like a fresh stream: 146 / 146); stub + body samples == num_samples (146 /
   146); the stub's own delta is 1152 + trim (EAL3: 3456 = 2 warm-up granules + 4 flushed) / 384 + trim (XMA 480–491)
granule index continuity (EAL3): on MPEG-1 streams the granule index alternates 0, 1, 0, 1 across block boundaries for every
   de-interleaved sub-stream (12 165 / 12 165 PS3 streams incl. cc 3 / 5); on MPEG-2 it is 0 throughout
```

## 2. Corpus result (every stream entry of every loose bank; loop bodies walked from the stubs)

| Tree | banks with streams | `.ast` | referenced streams (one-shot / loop) | loop bodies | refused | first-block delta (non-loop, multi-block) | terminal trim | files byte-accounted (0 unexplained) |
|---|---|---|---|---|---|---|---|---|
| X360 NHL 10 | 7 | 7 | 682 / 9 | 9 | 0 | 384 on all | 0–511 (11 streams ≥ 512 are single-block) | 7 / 7 |
| X360 NHL 11 | 7 | 7 | 719 / 11 | 11 | 0 | 384 | as above | 7 / 7 |
| X360 NHL 12 | 15 | 15 | 3 284 / 19 | 19 | 0 | 384 | 0–884 | 15 / 15 |
| X360 NHL 13 | 15 | 15 | 3 266 / 19 | 19 | 0 | 384 | " | 15 / 15 |
| X360 NHL 14 | 15 | 15 | 3 251 / 19 | 19 | 0 | 384 | " | 15 / 15 |
| X360 NHL 15 | 14 | 14 | 3 240 / 19 | 19 | 0 | 384 | " | 14 / 14 |
| X360 Legacy | 14 | 14 | 3 240 / 19 | 19 | 0 | 384 | " | 14 / 14 |
| PS3 NHL 11 | 7 | 7 | 1 104 / 4 | 4 | 0 | 1152 per sub-stream on all | 2 gr 594 / 3 gr 507 / 4 gr 3 | 7 / 7 |
| PS3 NHL 14 | 15 | 15 | 3 684 / 9 | 9 | 0 | 1152 | 2 007 / 1 672 / 5 | 15 / 15 |
| PS3 NHL 15 | 14 | 14 | 3 673 / 9 | 9 | 0 | 1152 | 2 003 / 1 665 / 5 | 14 / 14 |
| PS3 Legacy | 14 | 14 | 3 673 / 9 | 9 | 0 | 1152 | " | 14 / 14 |
| **total** | **137** | **137** | **29 816 / 146** | **146** | **0** | | | **137 / 137** |

Byte accounting per file: 0x40 header + Σ block headers + Σ payload bytes (frames / sub-blocks) + terminal zero padding + all-zero
inter-stream gaps (X360) + file tail == file size, with 0 unexplained nonzero bytes and 0 overlapping streams on 137 / 137 files.
The X360 single-block streams whose first-block delta is not 384 (95 streams: 385–884) are exactly the streams where the first
block is also the terminal block (delta = 384 + trim); the 11 X360 streams with a terminal trim ≥ 512 are those same single-block
streams. Every X360 `.ast` on NHL 12 … Legacy shares these 19 cases (same records; the trees are byte-identical for those banks).

Coverage by record class (title × platform × codec × rate × channels × loop) is in [COVERAGE.md](COVERAGE.md); each class is
`framed` (tiles) and, where R92 decoded a member, `decoded (diagnostic)`.

## 3. Controls (in memory, on the original `crowds.ast` / `crowds.abk` of each tree; the original file's hash re-verified afterwards)

| Mutation | X360 (7 trees) | PS3 (4 trees) | how it is refused / flagged |
|---|---|---|---|
| file truncated inside the terminal block header | detected 7 / 7 | 4 / 4 | "block header beyond the file — no terminal block" |
| file truncated 16 bytes before the terminal block's end | 7 / 7 | 4 / 4 | "size overruns the file" |
| block size 0 / 4 / 0xFFFFFF | 7 / 7 each | 4 / 4 each | "size < 8" / "overruns the file" |
| O beyond the file / O + 1 (unaligned) / O + 0x40 (mid-block) | 7 / 7 each | 4 / 4 each | offset checks; the mid-block walk fails the frame chain or the sample sum |
| samples word + 1 on a middle block | 7 / 7 | 4 / 4 | Σ samples ≠ num_samples |
| 0x80 flag cleared on the terminal block | 7 / 7 | 4 / 4 | walk runs into the next stream: end beyond next start / sum mismatch |
| 0x80 flag set on a middle block | 7 / 7 | 4 / 4 | Σ samples ≠ num_samples |
| loop word a + 1 | 7 / 7 | 4 / 4 | stub (samples, size) ≠ (a, b) |
| EAL3: one byte of a middle frame's common byte flipped | — | 4 / 4 | "MPEG version … reserved" / chain fails |
| EAL3: one byte of a frame's side info flipped | — | 4 / 4 | main data exceeds payload / chain fails |
| EAL3: one byte deep in main data flipped | — | **not detected 4 / 4 (expected)** | structurally invisible; only a decoder-side check could see it |
| XMA: w0 length + 4 | 7 / 7 | — | sub-block does not fit / trailing nonzero bytes |
| XMA: packet frame count 0 with metadata 1 | 7 / 7 | — | packet header refused |
| XMA: one byte of a packet body flipped | **not detected 7 / 7 (expected)** | — | structurally invisible |

The two undetectable classes are stated as such: the framing parser proves structure, not payload integrity.

## 4. Consumed-bytes accounting and what the parser reports separately

Per stream: header bytes (8 × blocks), payload bytes classified into frames / sub-blocks, terminal zero padding, `[end, next start)`
gap (X360, all zero), and any nonzero trailing bytes (0 everywhere). Per file: the identity above. The parser never treats an
unknown flag byte, an unknown codec, a non-multiple frame count (cc 3 / 5) or a zero run inside a non-terminal block as acceptable;
each is a named refusal (0 in the corpus, 15 / 15 in the controls).

## 5. Relationship between bank offsets and stream-file offsets

The bank tree's sample-list entry word O (R89) is the file-absolute start of the stream in the sibling `.ast` (29 962 / 29 962 walk
from O to a 0x80 block whose sample words sum to num_samples); the bank header word 0x2c equals the `.ast` id word (137 / 137); no
stream is referenced by two entries (multi_referenced_starts = 0 on every file); the loop bodies are the only unreferenced streams.
There is no other offset word: the record itself (8 / 16 bytes) carries none.

## 6. Semantic interpretation (supported by R92 decoder measurements; not game-verified)

- **Block → sample map.** Decoding the whole stream (EAL3 granules rebuilt into standard MPEG frames → ffmpeg `mp3float`; XMA packets
  padded to 0x800 → RIFF XMA2 → ffmpeg `xma2`) and then decoding again from a middle block k gives **bit-identical** output from the
  offset Σ(frames / N) × 576 (EAL3, after the first 2 granules of the restart) / Σ frame_count × 512 (XMA, immediately) on every
  vector (`data/r92-ast-decode.json`, D3). The sample words therefore index decoder output exactly; EAL3 blocks are not independently
  decodable without the previous blocks' MDCT / filter state (2 granules), XMA blocks are.
- **First-block delta 1152 (EAL3).** The EA encoder's two warm-up granules; the decoder output before index 1152 is the warm-up.
- **PCM block frame (EAL3 block 0, 47 samples; loop stubs 29).** Its samples equal the decoder's output at indices 1152 − 47 … 1151
  (corr 1.000 / 0.995 / 0.998 on the three mono speech / sfx vectors, 1.000 planar on the stereo loop body, 0.93 on the 4-channel body;
  the crowd ambience vectors carry silent PCM frames) with the **planar** layout (all left samples, then all right) for stereo. The
  candidate reconstruction is therefore: output = PCM frame (47 samples) followed by the decoder output from 1152 up to the terminal
  trim → exactly num_samples (D2b true on every non-stub part). Which of the two copies the game plays cannot be told from the bytes;
  the previews use this construction and say so.
- **Terminal trim.** The declared terminal count is what remains; the extra 1153–2304 decodable samples are the encoder's flush.
- **Loop.** Stub = the intro (word a samples; a self-contained encode), body = the loop; loop start = a (R82's ram loop_start rule,
  now verified on the stream side by the stub's own arithmetic, not only by counts).
- **XMA `w0` low bits**: census over 29 962 streams {0: 44 %, 1: <0.1 %, 2: 55 %, 3: ~3 %} — recorded, not interpreted (R84 saw 0 on
  mono / 2 on stereo SNS blocks; here both 0 and 2 occur on mono).
- **Not established:** any relation between the decoded PCM and what the console renders (mix, gain, resampling); the cc 3 / 5
  de-interleave rule beyond the structural evidence (frame order + per-sub-stream granule-index alternation); audibility.

## 7. Reproduce

```
python local/runs/overnight-research-2026-09-25/probes/r91_ast_framing.py docs/research/overnight-2026-09-25/data/r91-ast-framing.json --summary local/runs/overnight-research-2026-09-25/blockmaps/r91-streams.jsonl
python local/runs/overnight-research-2026-09-25/probes/r91_ast_framing.py <out.json> --trees ps3-nhl15,x360-nhl15 --banks playercalls,crowds --per-bank 60 --blockmaps <dir> --summary <file.jsonl>
```
Executed: two smoke runs on PS3 / X360 NHL 15 `playercalls` + `crowds` (the first exposed a too-strict zero-tail rule — terminal blocks
with 1–3 padding bytes — and the loop-stub fresh-start test, both corrected), the 11-tree run (57 s), two representative block-map runs.

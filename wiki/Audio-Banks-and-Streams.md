# Audio Banks and Streams

**Evidence:** measured. PS4 banks: 418 of 418 across seven titles (44 470 DSETs, 987 037 records). Gen7 effect banks: header and tree on 2 355 loose banks across 11 trees, stream files fully byte-accounted on 137 files. Table words marked open are decoded structurally but not semantically.

## PS4 `SBle` banks (SND10)

**Header:** `SBle`, u32 file size, u16 version (4), u16 DSET count, u32 bank id; an `MRAH` block at 0x10 and a pointer block at 0x50; a DSET list of (offset, slot) pairs; a `data` chunk, with a 0x104-byte stream-file path buffer when the bank streams from an `.sbs`.

**DSET (`TESD`):** record count at +0x38; 24-byte field descriptors from +0x48, each with a reversed four-character name, type, storage mode, shift and step, element size, base value and array offset. Known fields: `.SID` sound id, `.RAM` offset into the embedded data, `.OFF` offset into the `.sbs`, `.SBS` stream bank number (always 6), `.DUR` duration in seconds; three unnamed hashed fields on 36 DSETs are open.

**Storage modes**, all validated by an offset-lands-on-header test:

| Mode | Value of record *i* | Descriptors seen |
|---|---|---|
| 0 inline | the base value | 66 698 |
| 1 arithmetic progression | base + *i* × step | 32 533 |
| 2 array | base + (element[*i*] << shift) | 73 283 |
| 3 value table with bit-packed indices | table lookup | 290 |

Sampled offsets land on a stream header 36 780 times out of 36 780 on PS4 (3 343 of 3 344 on gen7 banks; one NHL 14 announcer pointer is out of range). `.DUR` agrees with the sample count and rate within 0.02 s on every one of 13 420 records.

## The SNS sample stream (every family)

A stream is a sequence of blocks `u8 type | u24 big-endian size`: a 0x48 header block carrying the 8-byte SNR header, 0x44 data blocks each followed by a u32 big-endian samples-in-block word, and a 0x45 end block. The SNR words are `version(4) | codec(4) | channel config(6) | sample rate(18)` and `type(2) | loop(1) | num samples(29)`; the rate and sample-count positions are confirmed by data, the exact bit split is the community's reading. An `.sbs` file is `data` plus 28 zero bytes, then streams back to back.

## Gen7 `SBKR` speech and music index

`SBKR | 0x110 | … | bank id | sample count`, then a 10-byte big-endian id table per sample (`u32 id | u16 count | u32 pointer`) leading to the `.sbs` offset. The `.sbs` starts `SBKS` plus 12 zero bytes. The Xbox 360 Legacy play-by-play bank holds 59 940 ids. Music bank slots: NHL 11 195, NHL 14 231, NHL 15 263, Legacy 239. There is no duration column.

## Gen7 `ABKC` effect banks

**Header:** `ABKC`; file size at 0x14; the `S10A` sample-table offset at 0x18; tree start at 0x1C (0x78 on PS3, 0x5C on Xbox 360); table data size at 0x24; the `.ast` id at 0x2C (0 when the bank has no stream file); three tail sections at 0x30, 0x34 and 0x38; a platform code (0x0a PS3, 0x05 Xbox 360).

**Sample table:** `S10A | 0 | count | header length`, then (count − 1) u32 offsets relative to the table, 0xFFFFFFFF marking an absent slot. Every record starts with the two SNR words; the second word's top bits give the type (0 ram, 1 stream), a loop flag and the sample count. Four record shapes:

| Type | Layout after the SNR words |
|---|---|
| ram, no loop | `u32 size` (including the 8 SNR bytes), `u32 samples`, codec bytes |
| ram, loop | `u32 loop_start`, then one or two {size, samples, bytes} segments (intro and loop body); loop_start equals the first segment's samples when there are two |
| stream, no loop | nothing more; the audio is in the bank's `.ast` |
| stream, loop | two further words |

Measured on 18 124 loose and 17 582 archived records with no failures.

**Tree:** tail A is empty on every bank; tail B is a pointer-fixup list (58 647 pointers; 8-byte slots on PS3, 4-byte on Xbox 360); tail C is a name directory of 130 389 names in three classes (`_snd`, `h_`, `_msg`). The name word is **`u16 bank key | u16 name key`**: the name key is constant per name across banks and platforms, the bank key constant per bank (0x7044 on goal-horn banks, 0x1be8 on period-horn banks, 0x2266 on chant banks). Neither hash was identified. Sample lists hang off named owners: `count` then entries of `{u16 record index (0xFFFF empty) | u8 weight (100, 90, 80, 70) | u16 X | u24 S | u32 O}`. Every record is referenced exactly once (48 751 of 48 751) and every list has a named owner (5 403). **O** is the file-absolute `.ast` stream start, 64-byte aligned, 0xFFFFFFFF for ram records. **S and X are open**: S on stream entries accumulates by about 2.01 × the sample count in an order that is neither list nor index order. Decoding the tree is for indexing, not for replacement.

Bank counts per tree: Xbox 360 NHL 10 199, 11 210, 12 219, 13 213, 14 218, 15 and Legacy 217; PS3 NHL 11 210, 14 218, 15 and Legacy 217. Record totals reconcile as present records plus absent slots, almost all absent slots in `fe.abk`.

## Gen7 `.ast` stream files

```
.ast    'S10S' | u32 id (equals the bank's 0x2C word) | zero to 0x40 | streams
stream  blocks { u8 flags | u24 size (incl. 8-byte header) | u32 samples | payload }   big-endian
        flags 0x00 = continuation, 0x80 = terminal; no other value in 29 962 blocks
PS3     streams tile contiguously; the terminal payload ends in zero padding to the next 0x40 boundary
X360    the terminal block size is a multiple of 0x40; streams are separated by zero gaps to the next 0x40 start
```

Sample-word rules per codec:

| Block | EALayer3 | EA-XMA |
|---|---|---|
| middle | samples = granules ÷ channels × 576 | frame count × 512 |
| first, over the declared count | +1 152 per sub-stream (two warm-up granules) | +384 |
| terminal trim | 1 153 to 2 304 samples | 0 to 884 samples |

**Loops:** the stream at O is a one-block stub whose (samples, size) equal the record's two extra words; the body is the next stream, referenced by no entry; stub plus body equals the declared count on all 146 loops.

**Coverage:** loose trees, 29 816 one-shot and 146 loop streams, 0 refused, 137 files fully accounted; archived PS3 NHL 09, 10, 12, 13 and Xbox 360 NHL 09, 9 987 streams and 28 loop bodies on 51 files, 0 refusals. Fifteen corruption mutations per platform are detected except one byte flip inside payload data, so the parser proves structure, not payload integrity.

## Horn banks

Gen7 ships one `{id:03}_goal.abk` and one `{id:03}_period.abk` per id for ids 0 to 29 and 100 (plus 194 from NHL 14), and an empty generic `horns.abk` on every tree; 135 of 217 horn banks hold a single sample. PS4 puts every horn in `hornbank.sbr` (NHL 21: 413 records in DSET 0 with ids 56 to 751, 5 in DSET 1; NHL 15: 334 records, two samples per id). The PS4 `applausebank.sbr` and `applausebankgoal.sbr` are 80-byte empty banks.

Last reviewed: 2026-10-20

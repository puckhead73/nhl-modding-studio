# Handoff R89 — the `ABKC` bank tree decoded as a **read / selection / stream-address contract**: the region between the file header and the S10A table is a serialized node graph with a pointer-fixup table (8-byte slots on PS3, 4-byte on X360), a name directory (hash + string per node, three classes `_snd` / `h_` / `_msg`) and **sample-list nodes** whose 12-byte entries reference every S10A record exactly once (48 751 / 48 751 on 2 355 banks of 11 trees, 0 duplicates, 0 missing) and carry, for stream records, the byte offset of the stream in the sibling `.ast` (29 962 / 29 962 walk to an end-flagged block whose sample words sum to `num_samples`); `crowds.abk`'s 86 records versus 89 chains are explained by looping streams (one-block stub + an unreferenced body whose samples add up exactly, 146 / 146 across the trees); every list is owned by a named picker node; three corruption controls refuse on every tree; stream *payload decoding*, the S / X words, the name hash and the other node classes stay open

Research lane, 24 September 2026, 13:26 → 13:47 MDT (review item "investigate ABKC offset 0x78 as a read / selection / stream-address contract").
Claim `research-r89-abk-bank-tree`. Read-only over the X360 NHL 10–Legacy and PS3 NHL 11 / 14 / 15 / Legacy loose trees (`X:`). Nothing
game-verified; nothing listened to; no writer proposed; replacement stays withheld (handoff 06 §F4 lists its separate dependencies).

Machine-readable: [`data/r89-abk-bank-tree.json`](data/r89-abk-bank-tree.json) (per tree: totals, every bank's SHA-256 / counts / owner
examples for the four vector banks, every `.ast`'s SHA-256 / mode / stream count / gap census, controls). Probe `r89_abk_bank_tree.py`
(≈ 4 min for the 11 trees; reuses the R82 table / record reader).

## Plain-language summary

The part of every gen7 sound bank that nobody had read is the bank's own table of contents: a small object graph that names every
sound ("goalHornMasterVolume", "boardPlaySamplePicker", …), lists which samples each picker may choose from, and — for the sounds
that stream from the companion `.ast` file rather than living inside the bank — records exactly where each stream starts. Every one
of the 48 751 sample records across 2 355 banks on eleven titles is named by exactly one such list entry, every recorded stream offset
walks cleanly to the stream's end marker with the right number of samples, and the three "extra" streams in the crowd file are the
bodies of looping ambiences that the bank points at through a one-block stub. What this gives Studio is a listing and a way to find
each sound's bytes; it does **not** decode the stream payloads (that is the next step), it does not explain two small words in each
entry, and it does not make replacement possible.

## 1. Grammar (observed on PS3 NHL 15, then tested on every bank of the 11 trees)

```
file header ('ABKC' | u32 0x01010300 | u32 w08 | … )
  w08      byte 0 = platform code (0x0a PS3, 0x05 X360); low u16 = number of 0x68-byte "handler" nodes at tree_start + 8
  0x14     file size            0x18 sample-table offset (S10A)        0x1c tree start (0x78 PS3, 0x5c X360)
  0x20     table offset again   0x24 table data size                   0x2c stream-file id (== `.ast` u32 at 4; 0 when no `.ast`)
  0x30 A   0x34 B   0x38 C      (three tail sections after table + data size; the `ffffffff` word sits at 0x48 PS3 / 0x40 X360)
tree region  [tree start, table): a u32 (0x2a1 / 0x7ce … per bank) + zeros, the handler nodes, then the node graph
tail  A  u32 count + count × u32                          — count 0 on all 2 355 banks
      B  u32 count + count × u32 LOC                      — pointer-fixup table: the pointer word is at LOC + 4 on PS3 (8-byte slot
                                                            `u32 0 | u32 ptr`) and at LOC on X360 (4-byte slot); every pointer is a
                                                            file-absolute offset inside the tree region
      C  u32 count + count × { u32 node | u32 name | u32 flags }   — node: file-absolute; name → { u32 hash | cstring, 4-padded };
                                                            flags byte 0 = class 0 `…_snd` / 1 `h_…` / 2 `…_msg`, low 24 bits per bank
sample list  (a pointer target whose first byte is 0):  u32 count | count × entry
      entry  u16 sample index (S10A record; 0xFFFF = empty slot) | u8 weight (100 / 80 / 70 / 90) | u16 X | u24 S | u32 O
             O = byte offset of the stream in the sibling `.ast` for stream-type records, 0xFFFFFFFF for ram records
`.ast`  'S10S' | u32 id | zeros to 0x40 | blocks { u8 flags | u24 size | u32 samples | payload }
      a block with flag 0x80 is the LAST block of its stream; stream starts are 0x40-aligned on both platforms; PS3 streams tile
      contiguously from 0x40 (the last block's size absorbs the alignment); X360 streams are separated by zero padding outside the
      blocks (walk each stream from its O); a loop-type stream record (16-byte S10A record, words a / b) points at a one-block stub
      whose (samples, size) == (a, b), and the loop body is the following stream, referenced by no entry
```

Pointer-target node classes by first byte: 0 = sample list (5 414), 1 (3 900) and 2 (3 800; `02 00 …  3f 80 00 00` + u16 tables —
curve-like) and 4 (48) — **not decoded**. Named-node classes: `_snd` 96 585, `_msg` 26 342, `h_` 7 462 (130 389 names; a name may be
attached to several nodes). Sample lists are reached from inside the extent of a named node in 5 403 / 5 403 cases — the **owner**:
`_msg` pickers 6 438 references, `_snd` 869, `h_` 33; 146 lists are reached from more than one named node (shared lists; kept in the JSON).

## 2. Measurements (every loose bank; per record, not sampled)

| Tree | banks | S10A records = list entries | lists | stream / ram entries | names | pointers | `.ast` files → streams | loop bodies | T1–T8 failures | controls refused |
|---|---|---|---|---|---|---|---|---|---|---|
| X360 NHL 10 | 199 | 2 741 | 408 | 691 / 2 050 | 10 419 | 4 483 | 7 → 700 | 9 | 0 | 3 / 3 |
| X360 NHL 11 | 210 | 2 809 | 420 | 730 / 2 079 | 10 677 | 4 575 | 7 → 741 | 11 | 0 | 3 / 3 |
| X360 NHL 12 | 219 | 4 695 | 483 | 3 303 / 1 392 | 13 010 | 5 636 | 15 → 3 322 | 19 | 0 | 3 / 3 |
| X360 NHL 13 | 213 | 5 107 | 526 | 3 285 / 1 822 | 14 056 | 6 256 | 15 → 3 304 | 19 | 0 | 3 / 3 |
| X360 NHL 14 | 218 | 5 099 | 525 | 3 270 / 1 829 | 11 927 | 5 521 | 15 → 3 289 | 19 | 0 | 3 / 3 |
| X360 NHL 15 | 217 | 5 088 | 524 | 3 259 / 1 829 | 11 924 | 5 520 | 14 → 3 278 | 19 | 0 | 3 / 3 |
| X360 Legacy | 217 | 5 088 | 524 | 3 259 / 1 829 | 11 924 | 5 520 | 14 → 3 278 | 19 | 0 | 3 / 3 |
| PS3 NHL 11 | 210 | 2 816 | 420 | 1 108 / 1 708 | 10 677 | 4 575 | 7 → 1 112 | 4 | 0 | 3 / 3 |
| PS3 NHL 14 | 218 | 5 110 | 525 | 3 693 / 1 417 | 11 927 | 5 521 | 15 → 3 702 | 9 | 0 | 3 / 3 |
| PS3 NHL 15 | 217 | 5 099 | 524 | 3 682 / 1 417 | 11 924 | 5 520 | 14 → 3 691 | 9 | 0 | 3 / 3 |
| PS3 Legacy | 217 | 5 099 | 524 | 3 682 / 1 417 | 11 924 | 5 520 | 14 → 3 691 | 9 | 0 | 3 / 3 |
| **total** | **2 355** | **48 751** | **5 403** | **29 962 / 18 789** | **130 389** | **58 647** | **137 → 30 108** (1.45 GB) | **146** | **0** | **33 / 33** |

Tests (per bank): T1 every record index in exactly one entry (0 duplicates, 0 missing on all trees); T2 every ram entry O = −1 and every
stream entry's O begins a walkable stream (PS3: O ∈ the contiguous chain's stream starts — 12 165 / 12 165 PS3 stream entries, an
*independent* check; X360: the chain is not contiguous, so each of the 17 797 streams is walked **from O** to its 0x80 block — there
the start rule is validated by the walk succeeding before the next start and by T3, not by an independent chain); T3 Σ block sample words == `num_samples` for every non-loop
stream; T4 every loop record: stub (samples, size) == (a, b) and stub + body samples == `num_samples` (146 / 146); T5 header 0x2c ==
`.ast` id (137 / 137); T6 every pointer inside the tree, every name inside the name area; T7 every list owned; T8 unreferenced streams =
exactly the loop bodies (0 other; X360 inter-stream gaps are all-zero bytes: e.g. NHL 15 `playercalls.ast` 487 gaps / 15 728 B).
Two structures that first failed and are now grammar, not failure: an entry with index 0xFFFF is an empty slot (`fe.abk` — its 65
records sit behind such slots; `skates_v2.abk` list at 0x4c7c8 holds `ffff` + records 328 / 329); X360 pointer slots are 4 bytes.

**Controls** (in memory, one bank per tree — `crowds.abk`): a pointer moved beyond the table → refused ("pointer … outside the tree");
a list count + 1 → refused (T1 duplicate); one stream O + 4 → refused (T2). 33 / 33.

## 3. The discriminating vector — `crowds.abk` / `crowds.ast`

| | PS3 NHL 15 | X360 NHL 15 |
|---|---|---|
| `crowds.abk` SHA-256 / size | `4382317d…` / 5 606 300 | `c375f2f5…` / 14 393 228 |
| S10A records (stream / ram) | 204 (86 / 118) | 198 (61 / 137) |
| `crowds.ast` SHA-256 / size / id | `e5fbafb7…` / 30 080 896 / `460ed680` | `06ef0606…` / 13 196 608 / `0b6fdf00` |
| `.ast` streams | **89 = 86 referenced + 3 loop bodies** | **70 = 61 + 9 loop bodies** |
| loop example | record 0 (loop, 689 988 samples, words 29 / 1 472) → stub at 0x40 (1 block, 1 472 B, 29 samples) + body at 0x600 (598 blocks, 689 959 samples): 29 + 689 959 = 689 988 | 9 stubs of (24 samples, 64 B) |
| owners (examples) | `cheerColorSamplePicker_msg` → records 40–57; `superFanSamplePicker_msg` → 58–75 (weight 90); `hiAmbColorSamplePicker02_msg` → 21–39; `base_value_reset_send` → single-record lists 0 / 80 / 88 (the loop ambiences) | same names, same list shapes |

`000_goal.abk` (1 record) is owned by `serviceShutdown_msg`; `000_chant.abk` (3 records) by `streamInterruptRecieve_msg`;
`000_period.abk` by `pauseBegin_msg` — the horn / chant banks' sample lists hang off service messages, not off a named "horn" sound;
the sound names that matter for those banks are the `_snd` parameter nodes (`firstSustain_snd` … `eighthGap_snd`, `hornsPauseDelayTime_snd`
in `000_goal`; the segment parameters R81 looked for in definition files live here, per bank).

## 4. Open (measured, not interpreted)

- **S** (u24): ram entries carry a small ordinal (1-based, per bank order; not the index); stream entries carry a cumulative value that,
  between consecutive entries of one list, is ≈ 2.01 × `num_samples` for mono 22 050 / 44 100 Hz lists and ≈ 4.01 × for stereo lists
  (looks like 16-bit PCM byte counts including a ~0.7 % tail), but the accumulation order is not the list order everywhere and the
  4-channel crowd streams do not follow it — **interpretation open**; not needed for addressing.
- **X** (u16): 0 / 1 / 2 / 3 / 4 / 0x11 / 0x12 / 0xdf1f / 0xcc34 / 0xcf30 … — not interpreted. **weight**: 100 / 80 / 70 / 90 — a pick
  weight by the owner names' semantics only.
- **Name hash**: 32-bit, e.g. `0x70442b7e` for `goalHornDuckerAmount_snd`; not CRC-32 / FNV-1(a) / djb2 / sdbm / Jenkins lookup2
  (seeds 0, 0xABCDEF00) — unknown; names are read from the strings, so nothing depends on it.
- **Node classes 1 / 2 / 4**, the handler nodes, the `_snd` parameter blocks (56 bytes typical; floats such as `bf800000`, u16 limits
  `7fff`), the `_msg` message bodies (they carry the list pointers) — structure not decoded; the `.csi` module scripts (`MOIR`) reference the
  same names (R81) — the event → picker binding is a runtime question.
- **Stream payload decode is NOT established here**: concatenating a PS3 stream's block payloads does not tile under the R77 EA-frame
  parser on most streams (28 / 499 on `playercalls`), i.e. `.ast` blocks carry per-block padding or framing that the ram-segment grammar
  does not cover; the same for X360 XMA blocks. That is the next package, not this one.
- **Archived trees** (PS3 09 / 10 / 12 / 13, X360 09) not measured — the loose tree grammar is expected to hold (same `ABKC` header
  words per R82) but is unmeasured there.

## 5. Consequence (developer; not Codex-accepted)

- **Listing contract (gen7 `.abk`, both platforms):** parse the tail (A / B / C), the pointer table with the platform slot width, the
  names, and the sample lists; present per bank "sound / picker name → sample records (weights)" and per record "ram (segment in the
  bank) / stream (`.ast` id, offset O, block count, samples)". Refuse when: section offsets are out of order, a section count does not
  fit, a pointer or node lies outside the tree, a list index ≥ record count, a record is referenced twice or never, the `.ast` id ≠ 0x2c,
  a stream does not walk from O to a 0x80 block before the next start, Σ block samples ≠ `num_samples`, or a loop stub ≠ (a, b).
- **Stream address contract:** stream bytes = [O, end of the 0x80 block]; loop = stub + following body. **Decode** of those bytes is a
  separate package (§4); until then the listing shows "stream located, preview not established".
- **Replacement:** unchanged — handoff 06 §F4 (a) / (b). This handoff supplies "entry indexing, stream addressing and names"; encoders,
  duration mirrors (the S / X words may be such mirrors — unknown), archive re-wrap and loader acceptance remain open.

```
python local/runs/overnight-research-2026-09-24/probes/r89_abk_bank_tree.py docs/research/overnight-2026-09-24/data/r89-abk-bank-tree.json
```
Executed: smoke on 3 trees (exposed the X360 4-byte pointer slot, the 0xFFFF empty slot and the X360 padded `.ast` layout), then a
10-tree run and the 11-tree run recorded here (PS3 NHL 15 had been missing from the tree list). Scratch: one smoke JSON, deleted.

> **Note 2026-09-25 (R91 / R94, [overnight-2026-09-25/AST_FRAMING.md](../overnight-2026-09-25/AST_FRAMING.md),
> [ABK_TREE_CONTRACT.md](../overnight-2026-09-25/ABK_TREE_CONTRACT.md)):** §4 "concatenating a PS3 stream's block payloads does not tile
> under R77 (28 / 499)" is explained — the terminal block ends in zero padding that the whole-stream parse read as a frame; parsed block
> by block, every payload tiles (29 962 / 29 962 streams, 11 trees) and Q-R89-1 is closed with the framing rules and a diagnostic decode.
> §1 / §4 "name → { u32 hash | cstring }" reads **`u16 bank key | u16 name key`** — the name key is constant per name across banks and
> platforms, the bank key per bank; the R89 algorithm search was run on the combined word and is superseded (the low half still matches
> no candidate). S / X remain uninterpreted (properties measured); "S ≈ 2.01 × num_samples" holds for adjacent entries of the true
> accumulation order, which is neither the list nor the index order.

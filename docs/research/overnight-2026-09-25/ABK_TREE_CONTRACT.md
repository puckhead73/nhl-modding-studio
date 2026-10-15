# ABK_TREE_CONTRACT — the `ABKC` bank tree after R89 + R91 + R94: demonstrated fields, unknown fields, supported variants

Research lane, 25 September 2026. Sources: R89 (handoff 07, 2 355 banks / 11 trees), R91 (`.ast` framing, this package), R94
(`data/r94-abk-sx-hash.json`, 1 279 banks of 6 trees for the word tests; the name-key facts re-checked on PS3 / X360 NHL 15). Nothing
here is a writer contract; the replacement boundaries are in [REPLACEMENT_READINESS.md](REPLACEMENT_READINESS.md).

## 1. Demonstrated (measured on every bank / stream named; refusals listed)

| Element | Contract | Evidence |
|---|---|---|
| header | `ABKC` · 0x14 file size · 0x18 S10A offset · 0x1c tree start (0x78 PS3 / 0x5c X360) · 0x24 table data size · 0x2c `.ast` id (== `.ast` word 4; 0 without `.ast`) · 0x30 / 0x34 / 0x38 tail sections A / B / C; w08 byte 0 = platform code 0x0a PS3 / 0x05 X360 | R89 (2 355 / 2 355), R91 T5 (137 / 137) |
| S10A table + records | R82 grammar (offsets relative to the table, 0xFFFFFFFF absent slot, ram = [loop_start] + 1–2 segments, stream = 8 / 16-byte stub) | R82 (35 706 / 35 706) |
| pointer-fixup table B | LOC list; the pointer word is at LOC + 4 (PS3, 8-byte slot) / LOC (X360, 4-byte slot); every target inside the tree | R89 (58 647 pointers) |
| name directory C | `{ u32 node | u32 name | u32 flags }`; node inside the tree; name → **`u16 bank key | u16 name key`** then cstring 4-padded (§2); flags byte 0 = class 0 `_snd` / 1 `h_` / 2 `_msg` | R89 (130 389 names); R94 (§2) |
| sample lists | pointer target with first byte 0: `u32 count` + `count × { u16 record idx | u8 weight | u16 X | u24 S | u32 O }`; idx 0xFFFF = empty slot; every S10A record referenced exactly once; every list owned by a named node | R89 (48 751 / 48 751; 5 403 / 5 403) |
| O | file-absolute `.ast` stream start (0x40-aligned) for stream records, 0xFFFFFFFF for ram records | R89 + R91 (29 962 / 29 962) |
| `.ast` stream framing | AST_FRAMING.md §1 (blocks, terminal flag, per-codec payload, padding, sample words, loop stub + body) | R91 (29 962 + 146, 0 refusals, 137 files byte-accounted) |
| loop records | stub (1 block, (samples, size) == (a, b)) + following unreferenced body with its own warm-up; a = loop start | R89 + R91 (146 / 146) |
| weight | 100 / 90 / 80 / 70 — a pick weight by the owner names' semantics only | R89 |

Refusals a reader must implement (all exercised by controls): section offsets out of order or counts that do not fit; a pointer or node
outside the tree; a list index ≥ record count; a record referenced twice or never; `.ast` id ≠ 0x2c; a stream that does not walk from O
to a 0x80 block before the next start; Σ block samples ≠ num_samples; loop stub ≠ (a, b); a block flag other than 0x00 / 0x80; block size
< 8 or overrunning the file; an EA-frame chain / XMA sub-block chain that does not tile its block; zero padding outside the terminal block.

## 2. The 32-bit "name hash" is two 16-bit fields (R94) — the R89 hash search was testing the wrong word

- **Low u16 = per-name key.** Constant for a given name across every bank and both platforms (1 217 distinct names on PS3 + X360 NHL 15:
  0 names with two values; 24 pairs of *different* names share a low16 on the 6-tree set — collisions consistent with a 16-bit hash,
  inconsistent with a sequential id). Examples: `sm_crowdAmbience_v` → 0x1ff3, `crowdReactionsMasterVol_snd` → 0x05b8,
  `streamInterruptRecieve_msg` → 0x6cab, `frolundaSamplePicker_msg` → 0x0cd6.
- **High u16 = per-bank key.** Constant within a bank (0 banks with two values), shared across banks of one family: 0x2266 on 117 of the
  217 PS3 NHL 15 banks (every `NNN_chant` bank but `001_chant`, which carries 0x605e), 0x7044 on the 33 goal-horn banks, 0x1be8 on the 31
  period-horn banks, 0x0218 on 15, … — a project / module identity of the authoring tool, not a hash of the bank file name (the same value
  spans differently named files) — **interpretation open**.
- **Algorithm search (low16):** 29 candidate functions (FNV-1 / 1a with two bases, djb2 add / xor / seed 0, sdbm, one-at-a-time, ELF,
  BKDR 31 / 131 / 1313, Java, x65599, AP, RS, DEK, MurmurHash3 seeds 0 / 0xABCDEF00, CRC-32 and its complement, lookup2 seeds 0 /
  0xABCDEF00 / 0x9e3779b9, CRC-16 CCITT (0xFFFF / 0) and ARC, byte sum) × 8 string variants (as-is, lower, upper, with NUL, class suffix
  stripped, suffix stripped + lower, `h_` prefix stripped, reversed) × 4 folds (low half, high half, xor-fold, add-fold) on 244 training
  pairs: best match 1 / 244 (chance). **No candidate reached the held-out test.** The 16-bit key stays unidentified; nothing in the reader
  depends on it (names are read from the strings).

## 3. Unknown words (measured properties only; no meaning assigned)

- **X (u16).** Constant within a list on 2 804 / 2 883 lists; small values 0–4 / 0x11 / 0x12 on stream lists; large values on stereo and
  4-channel entries (0xdf1f on cc 3 and on loop entries of both platforms, 0xcc34 on the stereo super-fan lists, 0xcf30 / 0xca36 / 0xce34
  elsewhere) identical on PS3 and X360 for the same record → derived from the authored asset, not from the encode. Not a duration in ms
  (tested), not a sample count. Open.
- **S (u24).** Ram entries: a small ordinal (equal to idx + 1 on 5 150 / 9 813, to list position + 1 on 2 255 — neither rule universal; R89's
  "1-based ordinal" is a partial description). Stream entries: identical on PS3 and X360 for the same (bank, record) on 2 006 of 3 079
  comparable pairs (the rest differ — some banks differ in record membership between platforms); multiples of 4; a cumulative sequence
  over the bank's stream entries in an order that is **neither** the list order (576 / 732 lists non-decreasing) nor the record-index
  order, interleaving runs of consecutive records; a successor chain (next S − S ∈ [2·ch·n, 2·ch·n × 1.02 + 64]) is unique for 491 / 500
  `playercalls` and 219 / 243 `sfx` entries and gives per-entry sizes = 2·ch·num_samples + overhead, overhead 100–8 000 (0.3–1.4 % of
  2·ch·n, not a multiple of 8, larger relative overhead at 22 050 Hz than at 44 100 Hz); not equal to: 2·ch·n, n, 4n, the PS3 or X360
  stream byte length, 8 × blocks (either platform), 8 × granules, 8 × ⌈n / 576 | 512 | 640 | 1024 | 1152 | 2048 | 8192⌉, 4 × ⌈n / 576⌉,
  2 × ⌈n / 128⌉. **Interpretation open** (a size in some authoring-side representation is the shape of it; nothing more is claimed). Not
  needed for addressing: O is the address.
- **Node classes 1 / 2 / 4, the 0x68-byte handler nodes, `_snd` parameter blocks, `_msg` bodies**: undecoded (R89); R95b adds that the
  crowd control bank's club `_msg` nodes are 28-byte (X360) / 56-byte (PS3) class-2 nodes whose first 20+ bytes are zero.

## 4. Supported variants

| Variant | Status |
|---|---|
| PS3 loose banks NHL 11 / 14 / 15 / Legacy (platform code 0x0a, tree 0x78, 8-byte pointer slots, EALayer3 v1 payloads) | read / list / locate / frame / diagnostic decode (cc 0 / 1 / 3; cc 5 framed only) |
| X360 loose banks NHL 10 … Legacy (0x05, tree 0x5c, 4-byte slots, EA-XMA payloads) | read / list / locate / frame / diagnostic decode (cc 0 / 1 / 3 as separate stereo sub-streams; cc 5 framed only) |
| archived banks PS3 09 / 10 / 12 / 13, X360 09 (RefPack / chunkzip / stored members) | records (R82) — tree and `.ast` unmeasured |
| banks without `.ast` (ram-only) | tree read; no stream side |
| PS4 | not this container |

## 5. The 89-versus-86 crowd count

Unchanged and now measured on the stream side as well: `crowds.ast` (PS3 NHL 15) holds 89 streams = 86 referenced by list entries + 3
loop bodies (records 0 / 80 / 88, the ambience loops owned by `base_value_reset_send`), each body walked from its stub's end with its
own warm-up signature and stub + body == num_samples; on X360 NHL 15, 70 = 61 + 9. Corpus: 146 loop bodies on 11 trees, 146 / 146.

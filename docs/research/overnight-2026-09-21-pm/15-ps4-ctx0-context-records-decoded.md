# Handoff R58 — PS4 commentary contexts (`0XTC` = little-endian `CTX0`, 64-bit pointers) decoded by pointer walk on NHL 15–21: the gen7 v4.5 model — trigger → conditions → items with EVT0-typed parameters — with every trigger, item, condition and script index resolving on all seven titles

> **Addition — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observation O-1).** A record's condition word addresses an **array of `u64` pointers, one per condition** (440 / 440 on NHL 15); a flat 24-byte array resolves only 67 / 440. The developer's reading is adopted; the 24-byte condition struct below is unchanged.

Research lane, 21 September 2026 pm, roadmap step 1.5 / 2.7 / 3.5 PS4 (parity of the gen7 context listing — the developer's P2-E contexts
card is gen7-only). Claim `research-r58-ps4-ctx0`. Read-only over the extracted PS4 trees (`audio/speech/eng/pbp_contexts.bin`, the two
MOIR `.csi` tables, `pbp_contextgasscript.bin`); nothing game-verified; no writer; the `.gas` bytecode stays out of scope.

Machine-readable: [`data/r58-ps4-ctx0-records.json`](data/r58-ps4-ctx0-records.json) (per title: file hash, version, counts, checks, kinds,
item namespaces, block-table histogram, refusals, the first 60 decoded records). Probe `r58_ps4_ctx0_records.py` (≈ 1 min for all seven).

## Plain-language summary

The PS4 games keep the commentary's context rules in the same kind of file as the last Xbox 360 / PS3 titles, written little-endian with
8-byte pointers: every record names its context, points at its trigger, lists its conditions (name + script index + the trigger slots it
reads) and carries one or two actions — a speech event with its parameters (immediates or `expression #k`) or a context trigger to
raise. Walking the pointers on all seven titles, every trigger name, every item (4 005 … 5 316 speech items and 273 … 777 context items
per title) and every condition name resolves by id **and** by the name spelled in the record, and every script index lies inside the
title's script file. Two file versions (3 on NHL 15–19, 4 on 20–21) share the layout; version 4 adds nothing the walk needs. So the
developer's context listing can cover PS4 15–21 with the same trigger → conditions → items presentation as gen7 11–Legacy, plus the
same undecoded header words (priority-like, time-like, kind) that handoff 05 labelled for gen7. Two things stay undecoded: a per-record
table of 16-byte entries (0–12 per record; the PS4 form of Legacy's slot block) and, as everywhere, the bytecode.

## 1. Layout (measured; the probe docstring has the byte map)

- File: `'0XTCTTEK' │ u32 version │ u32 n │ u32 A │ u32 n │ … │ u64 pool1 @0x30 │ u64 pool2 @0x38 │ 0x50: A × 16-byte group entries │ (n − 2) × u64
  record pointers │ T2 rows (u64 1, u64 ptr) … │ records │ pool1 (condition names) │ pool2 (trigger names)`. The count field is n but the
  pointer list holds n − 2 record pointers on every title (the next two u64 are the first T2 row) — recorded as an observation; the probe
  refuses those two "records" with the reason.
- Record header 0x58 B: `u32 context id │ u32 priority-like │ u8 ncond, u8 nextra, u8 nblk, u8 100 │ u32 flag │ u16 time-like, u16 kind │ … │ u64
  trigger-name offset (pool2; ff = none) │ u64 item slot 0 │ u64 item slot 1 │ u64 block-table ptr │ u64 condition list │ u64 extra ptr │ u64 name ptr`.
- Item (56-byte fixed part): `u64 0 │ u32 ref (u16 namespace, u16 event id) │ u32 0 │ u64 0 ×3 │ u64 name ptr │ u64 nparam │ nparam × (u64 kind, u64 ptr)`;
  kind 1 → immediate u32; kind 2 → `u64 0 │ u32 blob, u32 nargs │ u64 ff (nargs 0) or 0 │ nargs × (u64 1, u64 ptr → u32)`.
- Condition: `u32 nargs │ u32 blob │ u64 args ptr (ff when 0) │ u64 name offset (pool1)`.
- Block table (nblk entries of `u8 a │ u8 b │ u16 0 │ u32 v │ u64 w`): not decoded (values like `2 1 0 1153 ff…`, `255 255 65535 …`).

## 2. Checks (all seven titles; speech namespace = the title's speech MOIR namespace)

| Title | version | records (parsed / count − 2) | with trigger (all `trigger_*`, pool2 resolves) | speech items: id ∈ MOIR ∧ name = MOIR name | context items (raise) resolved | other namespaces | conditions (names in pool1; blob ≤ `.gas` count) | kind-2 params (blob ≤ `.gas`) | kinds | block entries | groups | `.gas` blobs |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| NHL 15 | 3 | 3 864 / 3 864 | 3 813 | 4 005 / 4 005 | 515 / 515 | 0 | 15 434 / 15 434 | 3 955 / 3 955 | 1: 3 813, 20: 51 | 0 | 108 | 2 537 |
| NHL 16 | 3 | 4 396 / 4 396 | 4 336 | 4 534 / 4 534 | 650 / 650 | 0 | 18 565 / 18 565 | 4 416 / 4 416 | 1: 4 336, 20: 60 | 19 | 110 | 2 735 |
| NHL 17 | 3 | 5 178 / 5 178 | 5 119 | 5 316 / 5 316 | 777 / 777 | 0 | 22 355 / 22 355 | 4 529 / 4 529 | 1: 5 119, 20: 59 | 11 697 | 121 | 3 112 |
| NHL 18 | 3 | 2 676 / 2 676 | 2 640 | 2 692 / 2 692 | 552 / 552 | 0 | 10 277 / 10 277 | 1 954 / 1 954 | 1: 2 640, 20: 36 | 8 073 | 93 | 3 088 |
| NHL 19 | 3 | 2 998 / 2 998 | 2 962 | 3 019 / 3 019 | 586 / 586 | 0 | 11 790 / 11 790 | 5 062 / 5 062 | 1: 2 962, 20: 36 | 9 284 | 97 | 3 258 |
| NHL 20 (install) | 4 | 1 418 / 1 418 | 1 397 | 1 414 / 1 414 | 273 / 273 | 0 | 2 704 / 2 704 | 2 224 / 2 224 | 1: 1 397, 20: 15, 60: 5, 10: 1 | 6 732 | 44 | 474 |
| NHL 21 | 4 | 2 153 / 2 153 | 2 119 | 2 147 / 2 147 | 479 / 479 | 0 | 4 313 / 4 313 | 4 894 / 4 894 | 1: 2 119, 20: 16, 60: 17, 10: 1 | 9 853 | 54 | 665 |

Every record's byte 3 of the count word is 100 on 15 / 16 and 96–99 % elsewhere (the remainder carry other values — recorded), `w5` / `w6`
are 0 everywhere, the `flag` word (gen7's `w3` `0x01000000`) is non-zero on 4–15 % of records. Kind 1 = triggered (every record with a
trigger); 20 / 60 / 10 = the trigger-less kinds, as handoff 05's gen7 table. The PS4 MOIR tables are little-endian with 24-byte entries
(`u64 0 │ u64 name ptr │ u64 id`) from 0x40 — a layout the 2026-09-20 `parse_moir` does not read; `parse_moir_le` in the probe does.

PS4 `.gas` (`CSIS`, little-endian): blob id words are sequential 1 … count on NHL 15 / 19 / 21 (2 537 / 3 258 / 665, offsets sorted, offset 0 =
table end) — the v4.5 index rule, i.e. the identity case of handoff 04's keyed lookup; the sampled condition blobs are within range.

## 3. Contract (extends handoff 03 rev. 2c of 2026-09-21 to PS4)

Context listing on PS4 15–21 exactly as gen7 13–Legacy: trigger → conditions (name, blob index, slots) → items (`speak <event>` with
EVT0-named parameters from the `0TVE` table of handoff 06 — the parameter *names* come from that table; `raise <trigger>`); the header
words labelled as candidates (priority-like / time-like / kind); the block table raw behind disclosure. Refusals: a file whose magic /
pointer walk fails the checks above (0 observed beyond the two trailing pointer-list entries, which are reported as "not a record");
the `.gas` blob count as the script-index bound (all inside). No runtime claim; the selection rule is as unobserved as on gen7.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r58_ps4_ctx0_records.py docs/research/overnight-2026-09-21-pm/data/r58-ps4-ctx0-records.json   (≈ 1 min)
```
Executed: two development runs on NHL 20 / 21 (the item fixed-part size was corrected from 48 to 56 bytes), one on 15 / 19, one final over
all seven; a word-by-word dump of NHL 21 record 0 in the shell (not retained). Inputs: the four `audio/speech/eng/pbp_context*` files per
title + `pbp_speecheventdef.csi` (SHA-256 of `pbp_contexts.bin` in the JSON).

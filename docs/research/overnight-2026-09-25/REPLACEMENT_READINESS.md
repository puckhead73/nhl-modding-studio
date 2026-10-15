# REPLACEMENT_READINESS — gen7 `.abk` / `.ast` audio: what safe replacement would still need (design only; nothing here is supported)

Research lane, 25 September 2026. Updates the two replacement-class prerequisite lists of handoff 06 §F4 / handoff 03 §5 for the
gen7 stream class after R89 (tree) + R91 (framing) + R92 (decode). **Diagnostic preview decoding is not replacement support**; every
row below is either `met (measured)` with its evidence or `open` with what is missing. No replacement was performed or designed
beyond this list; no writer exists.

## Class (a) — same-length encoded-payload substitution (a stream in `.ast`, or a ram segment in `.abk`)

| Prerequisite | gen7 `.ast` stream (PS3 EAL3 / X360 XMA) | gen7 `.abk` ram segment | status |
|---|---|---|---|
| entry indexing (record → bytes) | S10A record (R82) + sample-list entry O (R89) + block walk (R91) | S10A record + segment (R82) | **met (measured)** |
| framing the bytes to replace | blocks `flags | u24 size | u32 samples | payload` (R91); per-codec payload grammar | segments `size | samples | codec bytes` (R82) | **met (measured)** |
| an encoded payload of the same codec / rate / channels with identical byte length AND identical per-block sample words | requires an EALayer3 v1 (PS3) or XMA2 (X360) encoder producing the same block sizes, sample counts, warm-up (1152 / 384), PCM block frame (EAL3 block 0, planar), terminal trim and terminal padding | same codec constraints per segment | **open — no qualified encoder** (EAL3 v1: the public tool is unevaluated; XMA: Xbox SDK only); a donor entry of identical geometry is the only conceivable source |
| duration / size mirrors | the entry's S word (cumulative, unexplained — ABK_TREE_CONTRACT.md §3) and X may mirror the source length: a same-length swap keeps them, a different-length one would have to update S consistently for every later entry of the bank's S-order (order unknown) | same S / X words | **open** (S semantics unknown → any length change is unsafe) |
| pointer fixups | none for a same-length swap inside the `.ast` (O unchanged); the `.abk` tree's absolute pointers (58 647) are untouched | none if the segment size is unchanged | **met for same length only** |
| sharing | a stream is referenced by exactly one entry (multi_referenced_starts 0 / 137 files); banks are byte-identical across titles (chant 000 / 055–066 …) — a replacement is per file, per title | same | **met (measured), per-file scope** |
| checksums / hashes | none found in the `.abk` header or tail (R89 tail A is empty on 2 355 / 2 355); the `.ast` id word must equal 0x2c | none known | **open: absence of a checksum is measured, not proven** (the 16-bit per-bank key of the name words is unexplained) |
| archive re-wrap (archived titles) | RefPack / chunkzip members (PS3 09 / 10 / 12 / 13, X360 09): encoder fidelity + member size update | same | **open** (not this package) |
| loader / runtime acceptance | unknown; `na_en_loadfile.xml` lists banks by path + `descr` id only | same | **open (runtime)** |
| restore | copy of the original file + SHA-256 verified restore; no in-place partial state | same | design requirement, **not built** |

## Class (b) — arbitrary audio import (length may change)

All of (a) plus: a qualified encoder (EALayer3 v1 / EA-XMA / EA-Speex as applicable); the `.ast` re-layout (contiguous PS3 tiling with
terminal 0x40 padding / X360 0x40-multiple terminal size and zero gaps — R91 §1 gives the rules, but every later stream's O in the bank
tree moves and must be rewritten in the list entries); the S words of every later entry (unknown semantics → **blocked**); the `.abk`
table data size / file size words; loop stubs (a, b) if the intro changes. **Status: blocked on the encoder and on S.**

## What this package adds and what it does not

- Adds: the exact byte span and the exact framing of every stream; the sample-word rules an encoder would have to reproduce; the fact
  that a swap inside `.ast` moves nothing else when the length and the block sample words are preserved; the per-file uniqueness of a
  stream reference; the absence of a tail checksum in A; the two undetectable corruption classes (main data / packet body).
- Does not add: an encoder, a writer, S / X semantics, loader acceptance, or any observation of the console.

**Verdict per class:** (a) unqualified — the encoder and the S mirror are missing; (b) unqualified — blocked. Preview decoding stays
"diagnostic preview — not game-verified".

> **Note 2026-09-25 14:20 (R97 §4, [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md)):** every parser, decoder route and preview named here is research-owned (`local/runs/...`,
> `docs/research/`); none is Studio code or a shared fixture. MANIFEST.md / `data/manifest.json` are hash lists of inputs and outputs and
> are **not** evidence that any writer recovers; no writer exists in this lane. X360 NHL 09 (R98) adds a packet-header variant to class (a)'s
> "framing the bytes to replace" row: the header word is not a frame count there, so any encoder would also have to reproduce it.

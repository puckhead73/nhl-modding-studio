# Handoff R72 — gen7 EALayer3 v1 (SNS codec `0x05`) opened on its own vectors (48 samples, six trees: X360 15 / Legacy, PS3 15 / Legacy at 44 100 Hz, PS3 11 / 13 at 24 000 Hz): every block payload begins `00` + one byte that reads as MPEG version / sample-rate index / mono, blocks declare 16 granules (44.1 k) or 9 granules (24 k) with **+47 samples on the first block**, the PS4 sized-frame grammar does **not** apply (no size prefix tiles, no MPEG sync), and no per-granule side-info chain in a bounded search family tiles the payloads — the frame grammar stays **unidentified**, with the failure recorded exactly. No decoder, no preview, no encoder

> **Superseded 2026-09-23 by [R77](09-gen7-ealayer3-v1-grammar-established.md).** The frame grammar is now established on the same 48 vectors (EA-frame = one granule with a two-byte header, 1-bit granule index, scfsi on odd MPEG-1 granules, 12-bit `part2_3_length` + 47 / 51 side bits, byte-aligned main data; the first block's second frame is a `EE` PCM frame carrying 47 samples *after* its granule data). The structural observations below stand; §2's search family missed because it assumed a fixed post-length field width and no per-frame two-byte header, and §3's refusal text is replaced by a diagnostic-preview contract.

Research lane, 22 September 2026 pm (the morning campaign's named next task; prompt item 2, second half; coordinator review F2: each
codec class on its own vectors, no assumed transfer). Claim `research-r72-gen7-ealayer3-structure`. Read-only over the gen7 `pbp_eng`
banks (`.sbr` from the loose tree or the archive, `.sbs` loose or the archive member, exactly as r28 located them); nothing decoded,
nothing listened to, nothing written.

Machine-readable: [`data/r72-gen7-ealayer3-structure.json`](data/r72-gen7-ealayer3-structure.json) — per tree and vector: SNS SHA-256,
SNR words, every block's declared samples and payload bytes, the first eight payload bytes of the first four blocks, every size-prefix
reading's tiling result, the granule-chain readings per block and their intersection. Probe `r72_gen7_ealayer3_structure.py` (≈ 12 min).

## Plain-language summary

This morning's PS4 speech result came from noticing that each record carries its own length. The older consoles' speech uses a
different EA codec, and the review asked that it be examined on its own rather than assumed to be the same. It is not the same: none of
the length-prefix readings that solved the PS4 class fits a single block here, and no MPEG frame sync appears. What the data does show,
consistently on all 48 samples: every block starts with a zero byte and a second byte whose bits read like a standard MPEG header field
group — "MPEG-1, 44.1 kHz, mono" on the 44.1 kHz banks and "MPEG-2, 24 kHz, mono" on the 24 kHz banks — after which the bits are not
byte-aligned frames. Each full block holds 16 granules of 576 samples at 44.1 kHz and 9 at 24 kHz, the very first block claims 47 extra
samples (the same 47 that the MPEG delay arithmetic leaves over), and the totals match the header exactly. A bounded search for the
usual per-granule side-information layout under a family of headers found nothing that holds across the blocks of any sample, so the
inner grammar is still unknown; that is recorded precisely so the next attempt does not repeat it. Nothing here gives Studio a decoder
for these titles.

## 1. Measured on 48 / 48 vectors (8 per tree)

| Observation | 44 100 Hz (X360 15 / Legacy, PS3 15 / Legacy) | 24 000 Hz (PS3 11 / 13) |
|---|---|---|
| payload byte 0 (every block) | `0x00` | `0x00` |
| payload byte 1 (every block) | `0xcc` = `11 00 11 00` | `0x9c` = `10 01 11 00` |
| … read as MPEG header fields `version(2) sr_index(2) channel_mode(2) mode_ext(2)` | version 3 = MPEG-1, index 0 = 44 100, mode 3 = mono, ext 0 | version 2 = MPEG-2, index 1 = 24 000, mono, ext 0 |
| declared samples per full block | 9 216 = 16 × 576 (PS3 15 also 10 368 = 18 × 576 on 10 blocks) | 5 184 = 9 × 576 |
| first block | **9 263 = 9 216 + 47** on 32 / 32 | **5 231 = 5 184 + 47** on 16 / 16 |
| last block | remainder (3 287 … 8 808) | remainder (3 165 … 5 999) |
| Σ declared block samples = SNR `num_samples` | 32 / 32 | 16 / 16 |
| payload bytes per declared sample | 0.19–0.26 (X360), 0.25–0.37 (PS3) — VBR | 0.41–0.48 |
| MPEG frame sync (`FFF`) at any block start | none | none |
| size-prefix readings (u8 / u16 BE / u16 LE / u24, at 0 or 1, size incl. / excl., masks `7fff` / `0fff`) tiling **every** block of a vector | **0 readings** on 32 / 32 | **0 readings** on 16 / 16 |
| … tiling *some* block by chance | `u8@0+excl` on 1–4 blocks of a few vectors (coincidence) | none |

The byte-1 reading is *consistent* with the MPEG field group on both classes (two values only) and is stated as a reading, not an
established header. The +47 on the first block is an observation; its arithmetic coincidence with 1 152 − 1 105 (MPEG-1 encoder +
decoder delay) is noted, not interpreted. PS3 NHL 11 and NHL 13 share byte-identical first vectors (same SNS SHA-256): bank content is
reused across those titles.

## 2. Granule-chain search (recorded failure)

Hypothesis family tested on the first four blocks of every vector: `[h header bits] { [a bits][part2_3_length 12 bits][b bits][main
data part2_3_length bits] } × G` with G = ⌊samples / 576⌋ or ⌈…⌉, h ∈ 0..32, a ∈ 0..48, b ∈ 0..80, three layouts (interleaved;
interleaved with each granule's main data byte-aligned; all side info first then all main data). A reading counts when the chain ends
within byte padding of the payload end.

Result: individual blocks produce many chance readings (10–1 215 per block — the family is large enough that some `(h, a, b)` tiles any
byte string), but the **intersection across the four blocks of a vector is empty on 48 / 48 vectors**. Therefore: either the per-granule
main data is not delimited by a 12-bit length at a fixed offset from the granule start (a bit reservoir would break per-granule tiling,
as would a different field order), or the granule count per block differs from `samples / 576`, or the layout is outside this family.
Not tested: reservoir-aware chains (`main_data_begin` back-pointers), Huffman-coded side info, or any published EALayer3 description
(none consulted).

## 3. Consequence (developer; not Codex-accepted)

- Listing on gen7 (catalogue, alias set, offset, stored bytes, codec, rate, duration from `num_samples`) is unchanged and implementable.
- **Preview on gen7 EALayer3 v1 stays blocked**; the refusal text can now be specific: "EALayer3 v1 (codec 0x05, PS3 / Xbox 360
  speech): frame layout not identified; the PS4 EA-MP3 route does not apply (no length prefixes, no MPEG frame headers)".
- Do not build a gen7 decoder by adapting the `0x0b` parser; the two classes share only the SNS block wrapper.
- No encoder, no replacement; the alias / headroom / codec-per-slot consequences of the 2026-09-21 handoff 05 stand.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r72_gen7_ealayer3_structure.py docs/research/overnight-2026-09-22-pm/data/r72-gen7-ealayer3-structure.json --per-bank 8   (≈ 12 min)
```
Executed: three smoke runs (two trees / one tree; the first exposed a `None` guard, the second the floor / ceil granule count), one
full run. Temporary `.sbr` copies written to `%TEMP%` by the r28 locator and removed by it. Not executed: decoding, listening, writes.

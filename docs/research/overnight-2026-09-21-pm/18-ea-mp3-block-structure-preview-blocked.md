# Handoff R61 — PS4 EA-MP3 (SNS codec `0x0b`, NHL 15–17 speech): the block structure is measured — 9 MPEG-1 Layer III granule frames per data block, only the **first** frame header present — so the Opus-style "re-frame and hand to ffmpeg" preview route **does not** work here; preview stays blocked with a precise reason and a named next step

> **Correction note — 2026-09-22 (coordinator review F2; research handoff [R65](../overnight-2026-09-22/01-correction-checkpoint-f1-f3.md)).**
> The structural measurements below stand (block sample counts, prefixes, the single walkable frame per block, the failed conversion on
> 24 / 24 vectors, the false syncs). **Withdrawn as stated:** that the remaining eight frames "have had their headers removed", that
> `10368 = 9 × 1152` plus one walkable frame identifies nine frames or their boundaries, that header synthesis is *the* next step rather
> than *a* candidate, and that the same routine will also solve gen7 EALayer3 v1 — the variant relationship is untested. No granule past
> the first, its side information or its payload boundary has been identified. The reader/refusal text becomes "EA-MP3 (codec 0x0b,
> NHL 15–17): this speech format is not supported by Studio yet; the tested conversion failed", with duration, rate, block count, per-block
> bitrate and raw export still available. Retained as a candidate investigation, not a contract.
>
> **Resolved 2026-09-22 by research R67 ([handoff 03](../overnight-2026-09-22/03-ea-mp3-sized-frame-grammar-and-decode.md)).** The 2-byte
> prefixes are little-endian frame sizes (`0x8000` flag on the first frame, which also carries 1 152 PCM samples); every block is a chain
> of complete MPEG-1 Layer III frames, and the concatenated frames decode end to end on 24 / 24 vectors with two independent decoders.
> The "one walkable frame per block" observation below was an artefact of walking past a size prefix. Preview is **not** blocked.

Research lane, 21–22 September 2026, roadmap step 1.5 / 2.7 / 3.5 PS4 (prompt step 4, audio; the "other classes" line of handoff 07 §5).
Claim `research-r61-ea-mp3-structure`. Read-only over the extracted PS4 banks on `H:`; the only writes were temporary `.mp3` / `.wav`
files in the session scratchpad, removed. Nothing game-verified; no encoder, no writer.

Machine-readable: [`data/r61-ea-mp3-block-structure.json`](data/r61-ea-mp3-block-structure.json) (24 vectors — 8 per title — with SNR,
per-block prefixes and payload sizes, the MPEG header found in each block, the strict frame walk, the raw SNS SHA-256 and the naive
ffmpeg result). Probe `r61_ea_mp3_block_structure.py` (≈ 1 min).

## Plain-language summary

The speech on NHL 15–17 is stored in EA's own MP3 variant. Each data block of a sample holds exactly 10 368 audio samples — nine MP3
frames — but only the **first** of those nine frames still carries its MP3 header; the other eight have had theirs removed, and the
remaining bytes are packed without frame boundaries a decoder can find. So the trick that made NHL 18–21 previewable (handoff 07: hand
the stored packets to ffmpeg in a standard container) cannot work for 15–17: ffmpeg decodes the first frame or two and stops. Measured,
not assumed — every one of the 24 vectors fails, and the three runs that ffmpeg did not reject returned 2 304 samples out of 13 000+.
Producing a preview for these titles means rebuilding the missing frame headers from EA's granule container first — that is the next
piece of format work, and it is the same family as the gen7 EALayer3 problem, so solving it should cover both. Until then a listing can
show every 15–17 sample with its duration, rate and bitrate, and say "preview not available for this codec".

## 1. Measured structure (24 / 24 vectors, NHL 15 / 16 / 17, 8 each)

| Observation | Count |
|---|---|
| SNR codec `0x0b` (EA-MP3), 48 000 Hz, channel config 0, type `stream` | 24 / 24 |
| First data block's payload begins with a **6-byte** prefix, then an MPEG-1 Layer III header | 24 / 24 |
| Every later data block's payload begins with a **2-byte** prefix, then an MPEG-1 Layer III header | 24 / 24 |
| Exactly **one** walkable MPEG frame per data block (the strict frame walk stops after it) | 24 / 24 |
| Data-block `samples-in-block` = 10 368 = 9 × 1 152 on every block but the last | 24 / 24 |
| Σ `samples-in-block` = SNR `num_samples` | 24 / 24 |
| Payload tiles as MPEG frames (the naive hypothesis) | **0 / 24** |
| Naive concatenate-and-decode produces the declared length | **0 / 24** (3 runs returned 2 304 samples, i.e. two frames, and 21 were rejected outright: "Header missing") |

Per-block header fields seen: mono, 48 000 Hz, bitrate 48 / 80 / 96 / 128 / 160 kbps — i.e. the bitrate is chosen **per block**, which is
why each block re-states a header. The 6-byte first prefix is of the form `f6 89 80 04 00 00` / `26 8a 80 04 00 00` / `96 89 80 04 00 00`
(the low word is a constant `80 04 00 00` across all three titles); the 2-byte later prefixes take a small set of values (`f2 00`,
`82 01`, `22 01`, `e2 01`, …) — recorded as observations; their meaning is **not** established.

There are 115–242 byte pairs per sample that look like an MPEG sync (`FF Ex`) but are not valid headers — a sync scan is therefore not a
usable frame finder, and a reader must not try.

## 2. Consequence for the preview contract (extends handoff 07)

- **NHL 18–21 (codec `0x0c`, EA-Opus):** preview decodes — handoff 07, unchanged.
- **NHL 15–17 (codec `0x0b`, EA-MP3):** preview **blocked**. The refusal a reader should give is "EA-MP3 (codec 0x0b): the stored stream
  omits the per-frame MPEG headers; preview is not available for this title" — an observation about the container, not a claim that the
  data is unusable. Listing, duration (`.DUR`, SNR `num_samples` / 48 000), block count, per-block bitrate and export of the raw SNS bytes
  are all available today.
- Replacement / re-encoding is unchanged: no encoder exists for either class (handoff 07 §5, 2026-09-20 handoff 02 §3).

## 3. Named next step (research)

Reconstruct the missing frame headers: each block is one bitrate, one channel mode and nine frames, so a decoder can synthesise the eight
absent headers from the first one and re-derive the frame boundaries from the main-data lengths (the EALayer3 approach — the granule side
info gives `main_data_begin` and the part2_3 lengths). The same reconstruction is what the gen7 EALayer3 v1 class needs, so it should be
one piece of work covering PS3 gen7 and PS4 15–17. Not started here; this handoff is the structural measurement it starts from.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r61_ea_mp3_block_structure.py docs/research/overnight-2026-09-21-pm/data/r61-ea-mp3-block-structure.json --titles nhl15,nhl16,nhl17 --per-title 8   (≈ 1 min)
```
Executed: three exploratory shell scripts on NHL 15 / 16 / 17 (block dumps, frame walks, a sync map — deleted), then one final probe run.
Inputs: `audio/speech/eng/pbp_samples*.sbr` + `.sbs` per title (bank paths and the raw SNS SHA-256 per vector are in the JSON). Temporary
`.mp3` / `.wav` files were written to `$R61_SCRATCH` and removed by the probe (`--keep` retains them).

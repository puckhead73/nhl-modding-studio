# Handoff R77 — gen7 EALayer3 v1 (SNS codec `0x05`) inner grammar **established on 48 / 48 vectors** (six trees, both rates): each EA-frame is one Layer III granule — `00` + MPEG header byte, 1-bit granule index, scfsi on odd MPEG-1 granules, 12-bit `part2_3_length` + 47 / 51 side-info bits, byte-aligned main data — the first block carries **two extra granules and a 47-sample PCM frame** (`EE` flag; fields *after* the granule data), and the granules rebuilt into standard MPEG frames with a bit reservoir **decode with ffmpeg on 48 / 48** (samples = granules × 576). A diagnostic decode exists; the audible start and the discard rule are open; no encoder

> **Correction note — 2026-09-24 (coordinator review items 1–2; [R79](11-correction-checkpoint-r75-r78-review.md)).** Applicability is **narrowed to the six measured play-by-play sets** (X360 NHL 15 / Legacy and PS3 NHL 15 / Legacy at 44 100 Hz MPEG-1 mono; PS3 NHL 11 / 13 at 24 000 Hz MPEG-2 mono): §4's "PS3 09–13" and "pbp + ann" are withdrawn until the R80 survey releases further tuples; announcer, music, `.abk` sfx and stereo are untested. The summary's "very first record" is withdrawn: the PCM frame is the **second EA-frame of block 0** (48 / 48), its fields and 47 samples after that frame's padded main data. Counts are `granules × 576 samples`. The public reference is pinned: vgmstream `mpeg_custom_utils_ealayer3.c` at `4b6a02dd1aff6428255db912563d77d4cb0a143e`, SHA-256 `0bc1e0e9…39d8a`; in it the v1 PCM fields precede the granule side info — the NHL banks place them after the granule data, a **measured difference on these banks**, not a fault found in the upstream. Decoder acceptance and sample counts are evidence of a well-formed rebuild, not audible or game-equivalent validation.

Research lane, 23 September 2026 (brief item 2; Teams 2.7 / Leagues 1.5 / Players 3.5 gen7; supersedes the "unidentified" result of
handoff 05). Claim `research-r77-gen7-ealayer3-v1-grammar`. **Provenance of the hypothesis:** the bit layout was read from the maintained
public decoder's source (vgmstream, `src/coding/mpeg_custom_utils_ealayer3.c`, fetched from the public repository on 2026-09-23 and
summarised — no code copied; the brief permits referencing maintained decoders); the reading was then **tested and corrected on this
data** (the PCM-frame field position differs from the summary's "v1a" order — measured, §1). Read-only over the same banks as R72;
temporary rebuilt `.mp3` / decoded `.wav` files in the session scratchpad, removed by the probe. Nothing listened to; nothing
game-verified; no encoder; no writer.

Machine-readable: [`data/r77-gen7-ealayer3-v1-grammar.json`](data/r77-gen7-ealayer3-v1-grammar.json) — per vector: every block's
granule count, sizes, PCM frame fields, tiling result, the rebuild note and the ffmpeg decode (rc, sample count, PCM SHA-256, RMS,
first sample over 1 LSB). Probe `r77_gen7_ealayer3_v1_grammar.py` (≈ 6 min for 48 vectors; `--no-decode` for the parse only).

## Plain-language summary

Yesterday's search could not find the older consoles' speech frame layout. The maintained public decoder for this EA format describes
it, and its description — checked bit by bit against the 48 samples — holds once one detail is corrected from the data: each small
record is one MPEG granule with its own two-byte header, the compressed data padded to a byte, and the very first record of every
sound is flagged and carries 47 raw samples *after* its compressed data. With that reading every block of every sample parses exactly,
the granule counts match the declared samples (the first block holds two granules more than it declares — the decoder's warm-up — and
the last block is trimmed), and when the granules are packed back into ordinary MP3 frames, ffmpeg decodes all 48 without error to
exactly the expected number of samples. That is a diagnostic decode, like the PS4 one: what to do with the two warm-up granules and the
47 raw samples is the same open question as on PS4 (candidate rule, not verified), and nothing here lets Studio *write* this format.

## 1. Grammar (measured on 48 / 48 vectors: X360 15 / Legacy, PS3 15 / Legacy at 44 100 Hz MPEG-1; PS3 11 / 13 at 24 000 Hz MPEG-2)

```
block payload := ea_frame+                                   -- tiles exactly (T1: 48 / 48; every block of every vector)
ea_frame      := u8 pcm_flag (00 | EE)
                 u8 header   version(2) sr_index(2) channel_mode(2) mode_ext(2)   -- cc: MPEG-1 44.1k mono; 9c: MPEG-2 24k mono
                 bit granule_index(1)                        -- MPEG-1: alternates 0,1,0,1 within a block (T3: 48 / 48)
                 [ MPEG-1 and granule_index == 1: scfsi(4) ]
                 part2_3_length(12)  side_info(47 MPEG-1 | 51 MPEG-2)  -- the standard granule side info minus part2_3_length
                 main_data(part2_3_length bits)  pad-to-byte
                 [ pcm_flag == EE: u16 offset_samples (0)  u16 pcm_samples (47)  u32 zero  then pcm_samples × s16 ]
```

| Observation | 44.1 kHz (32 vectors) | 24 kHz (16 vectors) |
|---|---|---|
| EA-frames tile every block exactly (T1) | 32 / 32 | 16 / 16 |
| interior blocks: granules = declared / 576 (T2) | 16 per 9 216-sample block | 9 per 5 184 |
| **first block**: granules = declared − 47 PCM + **1 152** → **18** for 9 263 | 32 / 32 | **11** for 5 231: 16 / 16 |
| last block: granules × 576 ≥ declared (tail trimmed by the declared count) | 32 / 32 | 16 / 16 |
| exactly one `EE` frame per sample, always the **second** EA-frame of block 0, `offset_samples` 0, `pcm_samples` 47, u32 0 (T5) | 32 / 32 | 16 / 16 |
| header byte constant per rate class (T4) | `cc` | `9c` |
| Σ granules × 576 + 47 − `num_samples` = 1 152 + last-block trim (1 163 … 2 289) | 32 / 32 | 16 / 16 |

The PCM fields sit **after** the flagged granule's byte-aligned main data (the reference summary places them before the granule; the
data decides: read before, the walk fails at the second frame; read after, every block tiles and the next frame's `00 cc` follows the
47 samples exactly).

## 2. Diagnostic decode (T6) — rebuild into standard frames

Two EA granules → one MPEG-1 frame (one → one MPEG-2 frame) at the maximum standard bitrate (index 14; 1 044 B at 44.1 k, 480 B at
24 k), `main_data_begin` used as the standard bit reservoir (frame f's main data placed at byte P_f ≤ A_f in a continuous main-data
stream; max 511 / 255 reached and never exceeded), side info = the read bits, scfsi as read, ancillary zero. ffmpeg 8.1.1 `mp3float`:
**rc 0 on 48 / 48**, output samples = granules × 576 on 48 / 48 (e.g. 144 granules → 82 944; 2 716 → 1 564 416), RMS 3 400–6 000
(speech level), first sample over 1 LSB at 125–1 038 (decoder warm-up; the two extra first-block granules cover it). Without the
reservoir the MPEG-2 granules of 505–510 bytes do not fit a 480-byte frame (free format was tried and rejected by ffmpeg) — the
reservoir rebuild is the working route. PCM SHA-256 per vector is in the JSON (ffmpeg 8.1.1 `mp3float`, recorded options).

## 3. What is and is not established

- **Established (static):** the EA-frame grammar and its tiling; granule counts vs declared samples; the PCM frame's position and
  fields; that a standard decoder accepts the rebuilt stream and returns the expected sample count.
- **Open (Q-R77-1):** the audible start — whether the game discards the first 1 152 decoded samples and plays the 47 PCM samples in
  their place / before / at `offset_samples` (the same class of question as PS4 candidate A; the arithmetic 1 152 − 1 105 = 47 is
  noted, not interpreted); the end trim's exact placement; anything about the runtime. A listening card (A-EAL3-1) is the user's.
- **Not covered:** `ann_eng` announcer banks (24 kHz on PS3; same codec byte expected, untested), music and `.abk` sfx banks of codec
  `0x05`, stereo (`channel_mode` ≠ 3 never seen), the `0x09` class, EA-XMA, any encoder.

## 4. Consequence (developer; not Codex-accepted)

- Gen7 EALayer3 v1 speech (PS3 09–13 / 15 / Legacy pbp + ann; X360 15 / Legacy pbp) can get a **diagnostic preview** with the same
  shape as the PS4 EA-MP3 one: parse per §1 (refuse on any non-tiling block, unknown flag byte, header byte ≠ the rate class, PCM frame
  not the second frame of block 0 or `pcm_samples` ≠ 47), rebuild per §2, decode with any conforming Layer III decoder, verify sample
  count = granules × 576 and the JSON hashes (ffmpeg 8.1.1), label the output **"diagnostic preview — start handling unresolved (two
  warm-up granules + 47 PCM samples; not game-verified)"**. Replacement / encoding stays refused.
- Listing text replaces the R72 refusal: "EALayer3 v1 (codec 0x05): diagnostic preview available; replacement not supported".

## 5. Reproduce

```
set R77_SCRATCH=<scratch dir>
python local/runs/overnight-research-2026-09-22-pm/probes/r77_gen7_ealayer3_v1_grammar.py docs/research/overnight-2026-09-22-pm/data/r77-gen7-ealayer3-v1-grammar.json --per-bank 8   (≈ 6 min)
```
Executed: four smoke runs (the reference "v1a" PCM order failed; the measured order tiles; free format rejected; the reservoir rebuild
decodes), one full run. Temporary audio removed by the probe; `%TEMP%` `.sbr` copies removed by the r28 locator. Not executed:
listening, encoders, writes.

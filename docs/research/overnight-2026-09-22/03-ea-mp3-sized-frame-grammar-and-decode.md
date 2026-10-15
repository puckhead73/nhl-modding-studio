# Handoff R67 — PS4 EA-MP3 (SNS codec `0x0b`, NHL 15–17 speech): every data block is a chain of **sized frames** — a 2-byte size prefix before each complete MPEG-1 Layer III frame — so nothing was removed; stripping the prefixes yields a stream two independent decoders decode end to end on **24 / 24 vectors**. Preview contract established up to one open item (the role of 1 152 embedded PCM samples); no encoder, no writer

> **Correction note — 2026-09-22 pm (coordinator review F2; research handoff [R69](../overnight-2026-09-22-pm/01-correction-checkpoint-f1-f3-r66-r68.md)).** The grammar, the concatenated decode and the sample arithmetic stand on 24 / 24 vectors. **Restated:** ffmpeg's `mp3` and `mp3float` are a fixed / float consistency check inside one code base, not "two independent decoders" (lineage / independence not established); the preview contract of §4 is a **diagnostic** preview whose first 24 ms are unresolved — candidate A must not be selected silently and the output must not be described as verified game output; the PCM hashes belong to ffmpeg 8.1.1 with the recorded options, not to every conforming decoder; validation is recorded in three separate layers (extracted MPEG PCM / candidate-A rendering / tail trim); malformed or truncated sizes, `pcm_samples` bounds, later-frame flags, reservoir continuity and final counts are **not yet verified** (research R71, 2026-09-22 pm campaign). Listing text: "EA-MP3 (codec 0x0b): diagnostic preview available; replacement not supported".

Research lane, 22 September 2026 (prompt priority 2). Claim `research-r67-ea-mp3-sized-frames`. Read-only over the extracted PS4 banks on
`H:`; the only writes were temporary `.mp3` / `.wav` files in the session scratchpad, removed by the probe. Nothing game-verified; nothing
listened to (deferred to the user); no encoder and no writer claimed. Supersedes the *explanation* in the pm campaign's handoff 18 (its
measurements stand; see the F2 correction in [handoff 01](01-correction-checkpoint-f1-f3.md)).

Machine-readable: [`data/r67-ea-mp3-sized-frames.json`](data/r67-ea-mp3-sized-frames.json) — the same 24 vectors as r61 (8 per title,
raw SNS SHA-256 identical), each with every block's parsed frame list (sizes, bitrates, flags, PCM counts, `main_data_begin`), both
decoders' results and PCM SHA-256, and the junction measurements. Probe `r67_ea_mp3_sized_frames.py` (≈ 40 s).

## Plain-language summary

Yesterday's reading of this format assumed the MP3 frame headers had been stripped out. They had not. Each block of speech is a run of
small records: a two-byte length, then one *complete* MP3 frame (header included). The old probe stopped after one frame because the
two bytes after it are a length, not a sync word. Reading the lengths instead, every block of every one of the 24 test sounds parses
exactly — nine frames per full block, no leftover bytes — and when the lengths are removed the remaining bytes are an ordinary MP3 stream
that ffmpeg's two separate MP3 decoders both decode to the same audio (within one unit of rounding), at 48 kHz mono, with exactly
1 152 samples per frame. The bank's declared length is that total minus a short trim at the end of the last block, so the page can show
the right duration. One detail is still open: the very first record of each sound also carries 1 152 raw PCM samples (a frame's worth).
The evidence points to them standing in for the decoder's first frame, whose output is normally unusable, but that is not proven, and
the difference is 24 ms at the start of a clip. Nothing here lets Studio *write* this format; the bit reservoir runs across blocks, so a
replacement cannot be spliced in per block either.

## 1. Grammar (24 / 24 vectors, NHL 15 / 16 / 17, 8 each)

```
block payload  := ea_frame+                       -- sizes tile the payload exactly, no padding
ea_frame       := u16le hdr
                  [ u32le pcm_samples ]           -- present iff hdr & 0x8000
                  mpeg_frame                      -- complete MPEG-1 Layer III frame, header included, size from its own header
                  [ pcm_samples × s16le ]         -- present iff hdr & 0x8000
hdr & 0x7fff   == total bytes of the ea_frame (2 + [4] + mpeg + [2·pcm_samples])
```

| Observation | Count |
|---|---|
| Sizes tile every block exactly (`exact` on every frame, no error) | 24 / 24 |
| 9 frames in every block that declares 10 368 samples | 24 / 24 |
| `0x8000` flag set on the **first** frame of the sample only | 24 / 24 |
| That frame's `pcm_samples` = 1 152 (2 304 bytes s16le after the MPEG frame) | 24 / 24 |
| Every frame: MPEG-1, Layer III, 48 000 Hz, mono; bitrates vary **per frame** (32–224 kbps seen) | 24 / 24 |
| `main_data_begin` ≠ 0 on frames after the first, **including the first frame of later blocks** (bit reservoir crosses blocks) | 24 / 24 |
| First frame of the sample: `main_data_begin` = 0 | 24 / 24 |

So the r61 "6-byte prefix" is `hdr(2) + pcm_samples(4)` and the "2-byte prefix" is `hdr`; the r61 prefix values (`f2 00` = 242 = 240 + 2,
`82 01` = 386 = 384 + 2, `22 01` = 290 = 288 + 2, `e2 01` = 482 = 480 + 2) are the frame sizes, and the first prefix `f6 89` = `0x8000 | 2550`
with 2 550 = 2 + 4 + 240 + 2 304. The r61 hypothesis that eight headers per block were removed is **withdrawn**: no header is missing.

## 2. Decode (two independent implementations)

The MPEG frames were concatenated (prefixes and PCM removed) and decoded with ffmpeg 8.1.1's `mp3` (fixed-point) and `mp3float`
decoders, forced by `-c:a`:

| Check | Result |
|---|---|
| Both decoders return 0 and produce 48 000 Hz, 1 channel | 24 / 24 each |
| Output samples = frames × 1 152 (no Xing / LAME header, so ffmpeg trims nothing) | 24 / 24 |
| The two decoders' PCM differ by at most **1** LSB (never 0 exactly — fixed vs float rounding) | 24 / 24 |
| SNR `num_samples` = Σ block `samples-in-block` = frames × 1 152 − *tail trim*; trim = last block's (frames × 1 152 − declared), 204–1 097 | 24 / 24 |

Both decoders' PCM SHA-256 per vector are in the JSON, so a Studio implementation (any conforming Layer III decoder) can be checked
against them bit-for-bit (fixed) or within ±1 (float).

## 3. The embedded 1 152 PCM samples — open (Q-R67-1)

The flagged first frame carries 1 152 s16le samples (RMS 136–4 588; not silence). Two candidate roles were measured:

- **A — replacement:** the PCM stands in for decoded frame 0 (an MP3 decoder's first frame is degraded by the encoder / decoder delay;
  the decoded first 1 152 samples have RMS 25–1 592 against 152–7 391 for the next frame on these vectors). Junction jump at sample 1 152
  (PCM → decoded[1152:]), relative to the stream's own frame-boundary jumps: **≤ 0.1 on 11 / 24**, 0.1–1 on 7, **2.2–2.8 on 6**.
- **B — pre-roll:** the PCM precedes the whole decoded stream. Junction jump: ≤ 0.1 on 3 / 24, 0.1–1 on 10, 1.5–9.1 on 11.

A is favoured on 18 / 24 and the block-0 sample count (10 368 = 9 × 1 152, with the PCM not counted extra) is consistent with A, but the
best cross-correlation of the PCM against the decoded stream is only 0.16–0.77 at inconsistent lags (270–3 610), so **A is a candidate,
not a finding**. Failed hypothesis recorded: "the PCM is a copy of some later stretch of the decoded audio" — no lag reproduces it.

## 4. Preview contract input (rev. 1; not Codex-accepted)

- Applicability: SNR codec `0x0b`, sample rate 48 000, channel config 0 (mono), type `stream` — refuse anything else by field.
- Parse each `0x44` block's payload with the §1 grammar; refuse the sample if any size fails to tile its payload, any MPEG header is
  invalid, or the flag appears anywhere but the first frame.
- Concatenate the MPEG frames of **all** blocks (the reservoir crosses blocks — never decode a block alone) and decode with a
  conforming MPEG-1 Layer III decoder; expect frames × 1 152 samples; trim the tail to `num_samples`.
- First 1 152 samples: until Q-R67-1 is closed, a preview may use candidate A (replace decoded frame 0 with the embedded PCM) or the plain
  decoded stream, **labelled** as such; the difference is 24 ms at the start.
- Verification per vector: sample count, rate, channels and the PCM SHA-256 in the JSON — never a decoder exit code alone.
- Replacement / encode: **refused** — no encoder, and a per-block splice is impossible because of the cross-block reservoir. The listing
  should say "EA-MP3 (codec 0x0b): preview available; replacement not supported".

## 5. Gen7 EALayer3 (PS3 / X360, `EALayer3 v1`) — **not** covered

This grammar is the PS4 SNS `0x0b` form only. The gen7 class was not opened here; the relationship between the two is **untested** and
must not be assumed (review F2). It is the next audio item, on its own vectors.

## 6. Reproduce

```
set R67_SCRATCH=<scratch dir>
python local/runs/overnight-research-2026-09-22/probes/r67_ea_mp3_sized_frames.py docs/research/overnight-2026-09-22/data/r67-ea-mp3-sized-frames.json --titles nhl15,nhl16,nhl17 --per-title 8   (≈ 40 s)
```
Executed: one 3-vector smoke run (NHL 15), one single-vector `--keep` run for the lag / continuity analysis (files removed), one final
24-vector run. Inputs: the same banks and records as r61 (paths in the JSON; SNS SHA-256 per vector identical to r61's). Temporary
`.mp3` / `.wav` written under `R67_SCRATCH` and removed by the probe.

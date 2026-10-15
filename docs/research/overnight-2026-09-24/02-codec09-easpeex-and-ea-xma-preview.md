# Handoff R83 / R84 — codec `0x09` **identified as EA-Speex** (Speex ultra-wideband, 32 kHz mono, u8-length-prefixed frames, 509-sample encoder delay) from independent structure on 96 / 96 vectors of all 25 observed families (X360 11–Legacy announcer, X360 11–14 and PS3 14 play-by-play, PS4 15–21 rink announcer), decoded through two separate ffmpeg Speex decoders after a lossless Ogg rewrap; and EA-XMA (`0x03`) bounded to a **diagnostic-preview contract**: every SNS block is one XMA2 packet, every `.abk` segment a packet run, a RIFF XMA2 rewrap decodes on 91 / 91 vectors to exactly the declared sample count, and that count is shown not to come from the header

Research lane, 24 September 2026, 05:24 → 05:33 MDT (clock corrected against file times) (brief item R2; Leagues 1.5 / Teams 2.7 / Players 3.5 / Misc 5.4). Claims
`research-r83-codec09-easpeex`, `research-r84-ea-xma-preview`. Read-only over the trees (`X:` gen7, `H:\Emulators\ShadPS4\Games`
PS4 15–21 extracted data); temporary `.spx` / `.xma` / `.s16` in the session scratchpad, removed by the probes. Nothing listened to;
nothing game-verified; no encoder; no production dependency chosen.

Machine-readable: [`data/r83-codec09-speex.json`](data/r83-codec09-speex.json), [`data/r84-ea-xma-preview.json`](data/r84-ea-xma-preview.json).
Probes: `r83_codec09_speex.py` (≈ 4 min), `r84_ea_xma_preview.py` (≈ 8 min).

## Plain-language summary

The one speech codec on the older consoles and the PlayStation 4 that nobody had named is Speex — the open-source speech codec — in
its 32 kHz "ultra-wideband" mode, one frame every 20 ms with a one-byte length in front of each frame. That reading was reached from the
data alone (block sizes, frame counts, the frame header bits) and then found to agree with the maintained public decoder's handling,
including the 509 samples the game drops at the start. Wrapped losslessly into a standard container, two different Speex decoders
inside ffmpeg both return exactly the predicted number of samples on every announcer and play-by-play sample tested across fourteen
titles. Their outputs are similar but not identical, which is recorded rather than hidden — nothing here was listened to. EA's Xbox
360 XMA audio was also examined: each block is one XMA2 packet that only needs zero-padding, ffmpeg's XMA decoder consumes a plain
RIFF wrapper and returns the exact declared length on every vector, and a control shows that length comes from the bitstream, not from
the header we wrote. Both results are diagnostic previews: they say nothing about writing audio back.

## Part A — codec `0x09` = EA-Speex (R83)

### A.1 Observed structure (measured on NHL data before consulting any reference)

```
SNR h1 : version 1 | codec 0x9 | channel_config 0 | 32 000 Hz         (every vector; PS4 SBle banks carry the same words)
SNS    : 0x48 header | 0x44 data blocks (u24 size, u32 samples-in-block) | 0x45 end
block payload : { u8 length (1..0x100) | frame } × k, consuming the payload exactly (T1)
frames per block : 10 for a 6 400-sample block (640 samples = 20 ms at 32 kHz);   first block: 11 frames, declares 6 531 = 11 × 640 − 509 (T2)
middle blocks : samples = k × 640 (T3);   last block : 0 ≤ k × 640 − samples < 640 (T4);   Σ block samples = num_samples (T5)
single-block stream (PS4 NHL 21 base bank, 12 frames): k × 640 − 509 − samples = 252 (delay and tail trim on the same block)
frame byte 0 : bit 7 = 0 on 10 707 / 10 707 frames (no Speex in-band request), bits 6–3 = narrowband mode 1–8 (census: 7 → 4 284, 6 → 4 135, 5 → 1 276, 4 → 514, 3 → 396, 2 → 79, 8 → 13, 1 → 10)
frame lengths : 15 … 110 bytes (variable-rate)
```

### A.2 Reference (pinned; read, not vendored; upstream behaviour stated apart from the measurements)

vgmstream revision `4b6a02dd` (fetched with curl from the public repository, 24 September 2026), license `COPYING` (ISC-style,
SHA-256 `fcc26d35…`): `src/coding/speex_decoder.c` (SHA-256 `1c0e0439…`) — the `EA` Speex type reads `u8 frame size | frame`,
initialises libspeex in **UWB** mode, forces 32 000 Hz ("forced in EA's code"), and discards an encoder delay of **509** samples;
`src/meta/ea_eaac.c` (`978c9a37…`) maps codec `0x09` to `EAAC_CODEC_EASPEEX` and notes "first block samples count frames' samples
subtracting encoder delay". Borrowed: nothing executed; the constant 509 and the UWB mode are the reference's — the same numbers fell
out of T2 on the data (11 × 640 − 6 531 = 509) before the file was read. The 2026-09-20 handoff 02's remark that the public table's
label was "implausible for these platforms" is superseded: it is plausible and now measured.

### A.3 Independent decode (diagnostic; decoder acceptance only)

Frames rewrapped as Ogg Speex (80-byte Speex header: rate 32 000, mode 2 = UWB, frame size 640, 1 frame per packet, VBR; one packet
per frame; RFC 3533 pages with the Ogg CRC) and decoded twice by ffmpeg 8.1.1 — `speex` (FFmpeg's own decoder, `libavcodec/speexdec.c`)
and `libspeex` (the reference library through FFmpeg). These are two source trees, not one; whether the native decoder was derived from
libspeex's algorithm is not claimed either way.

| Family | vectors | structure T1–T5 | `speex` samples = frames × 640 | `libspeex` same | decoded − 509 − tail = num_samples | decoders identical | rms(diff) / rms | corr | malformed refused (4 / 4) |
|---|---|---|---|---|---|---|---|---|---|
| X360 11 / 12 / 13 / 14 / 15 / Legacy `ann_eng` (32 kHz) | 24 | 24 | 24 | 24 | 24 | 0 | 0.07–0.29 | 0.96–1.00 | 6 / 6 |
| X360 11 / 12 / 13 / 14 `pbp_eng` | 15 (+ 1 skipped) | 15 | 15 | 15 | 15 | 0 | " | " | 4 / 4 |
| PS3 14 `pbp_eng` | 4 | 4 | 4 | 4 | 4 | 0 | " | " | 1 / 1 |
| PS4 15–20 `rink_ann_samples` (302 records each) | 24 | 24 | 24 | 24 | 24 | 0 | " | " | 6 / 6 |
| PS4 15–21 `rink_ann_samples_inxp` (301 / 302 records) | 28 | 28 | 28 | 28 | 28 | 0 | " | " | 7 / 7 |
| PS4 21 `rink_ann_samples` (1 record, 12 frames, single block) | 1 | 1 | 1 | 1 | 1 | 0 | 0.89 | 0.46 | 1 / 1 |

Totals: 97 vectors sampled (4 per family: first / two spaced / last `.sbs` offset; the PS4 picks by `.OFF`), 96 well-formed; the skipped
vector is X360 NHL 14 `pbp_eng` offset 820 125 120, beyond the truncated loose stream already recorded by R80. Sample counts: the decoded
length equals frames × 640 from both decoders on 96 / 96; `decoded − 509 − tail_trim == num_samples` on 96 / 96 (tail trims 3–604).
The two decoders never produce identical PCM; on speech their difference is 7–29 % rms with correlation ≥ 0.96 — consistent with two
implementations of one codec differing in post-filtering, **not** measured further and not heard. Malformed controls (per family, first
vector): last block truncated by 3 bytes, a frame length of 0, a frame length of 0xff overrunning the block, a block sample word + 1 —
refused 4 / 4 on 25 / 25 families with the check named.

### A.4 Consequence (developer; not Codex-accepted)

- Listing: "EA-Speex (codec 0x09): Speex ultra-wideband, 32 kHz mono; diagnostic preview available; replacement not supported".
- Parser contract: walk SNS blocks (gen7 `SBbe` / PS4 `SBle` banks alike); per block, the u8-length frame chain must consume the payload
  exactly (refuse otherwise, naming block and offset); frame length 1..0x100; first block samples + 509 = frames × 640 (single-block
  streams: frames × 640 − 509 − samples ∈ [0, 640)); middle blocks samples = frames × 640; last block tail trim ∈ [0, 640); Σ = num_samples.
- Decode contract (diagnostic): any Speex decoder in UWB mode at 32 000 Hz, one frame per packet, yields frames × 640 samples; drop the
  first 509 and the tail trim → num_samples. **Label "diagnostic preview — not game-verified"**; the 509-sample start is the reference's
  and the data's arithmetic, not a heard result (card A-SPX-1 below). A pure-Rust Speex decoder is not surveyed here; the constraints
  a candidate must meet are: UWB mode (the sub-band layers), VBR frames, per-frame packet decode, no in-band requests needed.
- Fixture: X360 NHL 15 `audio/speech/ann_eng_samples.sbs` (loose = `nocache.big` entry 1, 131 679 664 B) offset 16: SNS SHA-256
  `6157e642f1e69864…`, 9 596 bytes, 15 blocks, 143 frames, num_samples 90 795, tail trim 216; `libspeex` PCM SHA-256 in the JSON. PS4
  NHL 15 `rink_ann_samples.sbs` offset 32: 89 frames, 56 160 samples, trim 291. (The X360 15 and PS4 15 `_inxp` first vectors share
  num_samples 90 795 and 143 frames but not bytes — different encodes of one line, SNS hashes `6157e642…` vs `7a818fb6…`.)

## Part B — EA-XMA (`0x03`) diagnostic-preview assessment (R84)

### B.1 Observed structure (measured; the reference read afterwards agrees)

```
SNS-blocked (speech / music `.sbs`): block payload = u32 w0 | packet bytes,   w0 >> 2 = payload length (low 2 bits: 0 on mono blocks, 2 on stereo — recorded, not interpreted)
   packet bytes = ONE XMA2 packet: header u32 BE (6-bit frame count | 15-bit first-frame bit offset | 3-bit metadata | 8-bit skip) + frames, length < 0x800 (max seen 2 052 incl. w0) → zero-pad to 0x800
   first block: frame count × 512 − samples = 384 on 27 / 27 SNS vectors (encoder delay); last block: tail < 512
raw `.abk` ram segment: u32 w0 (w0 >> 2 = segment length, low bits 0 / 1 / 2) | consecutive 0x800 packets, last partial → pad
> 2 channels: one `u32 size | data` sub-block per stream in each block (reference; NHL cc 3 / 5 entries not decoded here)
```

### B.2 Reference (pinned)

vgmstream `4b6a02dd`: `src/meta/ea_eaac_streamfile.h` (SHA-256 `d10ae126…`) — EA-XMA block = num-samples, per stream `(size × 4 +
packets)`, data padded to `XMA_FRAME_SIZE 0x800`; `src/meta/ea_eaac.c` — EA-XMA is always played as XMA2 (XMA1 detection "isn't so
easy"; playing XMA1 as XMA2 works, not the reverse), through FFmpeg with `ffmpeg_make_riff_xma2` (`src/coding/ffmpeg_decoder_utils.c`,
`801434aa…`: WAVEFORMATEX tag 0x166, streams = (channels + 1) / 2, speaker mask, block size 0x10000). The reference's "size × 4" is the
low-bits-zero case of the measured `w0 >> 2`.

### B.3 Decode (diagnostic) and the header-independence control

RIFF XMA2 (as the reference builds it) + the padded packets → ffmpeg 8.1.1 `xma2` (`libavcodec/wmaprodec.c`; the only XMA decoder available — no second implementation).

| Family | vectors | structure ok | decoded | decoded = declared samples | first-block delay 384 / tail < 512 | robust 3 / 3 |
|---|---|---|---|---|---|---|
| X360 09 / 10 `ann_eng`, `pbp_eng` (24 kHz mono, SNS) | 16 | 16 | 16 | 16 | 16 | 4 / 4 |
| X360 09 / 10 / 11 / 12 `music_samples` (44.1 kHz stereo, SNS) | 11 (+ 1 skipped: NHL 11 offset 298 517 360 beyond the loose stream) | 11 | 11 | 11 | 11 | 4 / 4 |
| X360 10–Legacy loose `000_goal` / `000_chant` / `sfx.abk` (raw, cc 0 / 1) | 49 | 49 | 49 | 49 | 49 | 21 / 21 |
| X360 09 archived (`data0.big`) `000_goal` / `000_chant` / `sfx.abk` + X360 09–Legacy `000_period.abk` (36 kHz mono) | 15 | 15 | 15 | 15 | **0** — packet headers read frame count 2 / metadata 0 on every packet (frame sum ≪ samples): a different packet-header class (XMA1-style?) — not interpreted; decode unaffected | 11 / 11 |

**Control.** The RIFF's sample-count field was set to the declared value, declared − 1 000, declared + 100 000 and 0 on four vectors
(X360 10 `ann_eng` 16, X360 15 `000_goal` / `000_period` / `sfx.abk` entry 0): the decoded length was identical in all four cases
(32 057 / 298 234 / 99 683 / 60 712). The exact match to `num_samples` therefore comes from the XMA bitstream (frame end flags) as the
decoder reads it, not from the wrapper — decoder acceptance evidence, still not audible correctness. Malformed controls (per family):
last block / segment truncated by 3 bytes → w0 mismatch refused; w0 + 4 → refused; first packet frame count forced to 0 → refused.

### B.4 Consequence (developer; not Codex-accepted)

- Preview contract (diagnostic): SNS block → strip `w0`, pad the packet to 0x800; raw segment → strip `w0`, split into 0x800 packets,
  pad the last; feed a RIFF XMA2 (channels, rate, streams = (ch + 1) / 2) to an XMA2-capable decoder; expect exactly `num_samples`
  (Σ block samples) — refuse when w0 >> 2 ≠ the payload / segment length, when a packet header's frame count is 0 with metadata 1, or
  when the decoder's count ≠ the declared count. Label "diagnostic preview — not game-verified". cc 3 / 5 entries: "multi-stream
  layout (per-stream sub-blocks) not measured — listed only".
- Dependency constraints (not a choice): an XMA2 decoder is required — FFmpeg's `wmaprodec` is the only maintained implementation
  known to this lane; a pure-Rust XMA decoder is not known; a shell/ffmpeg dependency would be a production decision the developer must
  document, not a research recommendation.
- Fixtures: X360 NHL 10 `ann_eng_samples.sbs` offset 16 (SNS SHA-256 `f8a4ef2a…`, 13 940 B, 8 blocks, 64 frames, 32 057 samples;
  decoded PCM SHA-256 `e2f75986…`); X360 NHL 12 `music_samples.sbs` offset 16 (`3960e0c4…`, 199 blocks, 785 408 samples, stereo);
  loose X360 NHL 15 `000_goal.abk` entry 0 (segment 133 619 B, SHA-256 `af4ce412…`, 66 packets, 298 234 samples, decoded `b44a196b…`);
  the header-class residual: loose X360 NHL 15 `000_period.abk` entry 0 (22 532 B, `1fee5ec9…`, 11 packets, 36 kHz, 99 683 samples).
- No encoder, no replacement, no bank / event mutation follows from either part.

## Cards (deferred; no listening this session)

- **A-SPX-1** — EA-Speex start: for X360 NHL 15 `ann_eng` offset 16 and PS4 NHL 15 `rink_ann_samples` offset 32, listen to (S) the
  `libspeex` rendering from sample 509 and (S0) from sample 0; control: the same line's PS4 `_inxp` copy. Outcome: clean onset at S and a
  click / dead air at S0 → the 509 rule is audibly plausible; it does not identify the game's reconstruction beyond that.
- **A-XMA-1** — EA-XMA: X360 NHL 10 `ann_eng` offset 16 and NHL 15 `000_goal.abk` entry 0 through the RIFF rewrap: audible plausibility
  of the diagnostic route (start / end / pitch at 24 kHz vs 44.1 kHz), nothing more.

## Reproduce

```
set R83_SCRATCH=<scratch>   python local/runs/overnight-research-2026-09-24/probes/r83_codec09_speex.py docs/research/overnight-2026-09-24/data/r83-codec09-speex.json
set R84_SCRATCH=<scratch>   python local/runs/overnight-research-2026-09-24/probes/r84_ea_xma_preview.py docs/research/overnight-2026-09-24/data/r84-ea-xma-preview.json
```
Executed: r83 smoke (2 trees) + full run + full rerun after the single-block rule; r84 smoke (2 trees, exposed the stereo `w0` flag bits
and the need for a header control) + full run + the four-vector header control (inline). References fetched once, hashed, kept in the
scratchpad `ref/` only.

> **Correction note 13:25 MDT (R88, [handoff 06](06-correction-checkpoint-campaign-review.md) §F4):** (1) A.1 / A.4 "frame sizes 1..0x100" is
> wrong — the u8 length byte is accepted as **1..255** (0 refused by the malformed control; 256 not encodable); observed 15..110. (2) The
> summary's "the 509 samples the game drops" is superseded: 509 is a measured count relationship (block 0 declares 11 × 640 − 509) and the
> pinned reference decoder's rule, with no game trace. (3) A.3's last row is an **outlier, not a range member**: PS4 NHL 21 base
> `rink_ann_samples` (single block, 12 frames) — rms ratio 0.89, correlation 0.46 between the two decoders (all other 95 vectors 0.07–0.29,
> ≥ 0.96); cause unidentified; on that class neither decoder is validated. Sample-count agreement is not sample fidelity or start
> reconstruction anywhere in this handoff.

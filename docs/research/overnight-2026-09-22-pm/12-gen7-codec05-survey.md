# Handoff R80 — gen7 codec-`0x05` (EALayer3 v1) sources surveyed on all 16 trees by title / platform / rate / channel / container, and the R77 grammar applied per class on independent vectors: **established** on PS3 announcer banks (24 kHz mono, 32 / 32, eight titles), PS3 play-by-play on the three unmeasured 24 kHz titles (09 / 10 / 12: 12 / 12), `music_samples` at 44.1 kHz **stereo** on X360 13–Legacy and PS3 11 / 14 / 15 / Legacy (24 / 24, with two-channel side info and a stereo rebuild) and PS3 `.abk` sfx raw streams (24 / 28; one chant entry does not tile — residual); **not this codec**: X360 announcer / pbp 11–Legacy (`0x09`), X360 09 / 10 speech, X360 09–12 music and every X360 `.abk` (EA-XMA). Malformed inputs refused on every codec-`0x05` class

Research lane, 24 September 2026 (brief item 1; Teams 2.7 / Leagues 1.5 / Players 3.5 / Misc 5.4 gen7). Claim
`research-r80-gen7-codec05-survey`. Read-only over the 16 gen7 roots (loose `audio/speech`, `audio/music`, `audio/sfx`; speech banks
inside archives where r28 located them); temporary rebuilt `.mp3` / decoded `.wav` in the session scratchpad, removed by the probe.
Nothing listened to; nothing game-verified; no encoder.

Machine-readable: [`data/r80-gen7-codec05-survey.json`](data/r80-gen7-codec05-survey.json) — per tree and class every vector's SNR
words, SNS SHA-256 (or `.abk` entry hash), the R77 results (tiling, granule rule, PCM frame, rebuild note, decode rc / samples /
channels / PCM SHA-256), the malformed-input results, and a per-tuple summary. Probe `r80_gen7_codec05_survey.py` (≈ 10 min).

## Plain-language summary

Yesterday's speech grammar was proven on six play-by-play banks and, on review, restricted to exactly those. This survey opens every
other place the same codec byte appears on the older consoles and tests each on its own samples. The PlayStation 3 announcer banks
(all eight titles), the remaining PlayStation 3 play-by-play titles, the menu-music banks on both consoles from NHL 13 on, and the
PlayStation 3 sound-effect banks all parse, rebuild and decode the same way — the music needed the two-channel form of the same
grammar, which was added and checked. Xbox 360 announcers and most Xbox 360 play-by-play use a different, still-unidentified codec, and
every Xbox 360 sound-effect bank uses EA's XMA; those are not covered by this grammar and say so. One sample of one chant bank does
not parse and is left as a named residual rather than forced.

## 1. Codec map by class (16 trees; codec byte from the SNR words; vectors per tree: 4 speech, 3 music, 1–3 per `.abk`)

| Class | EALayer3 v1 (`0x05`) | `0x09` (unidentified) | EA-XMA (`0x03`) | not present / not surveyed |
|---|---|---|---|---|
| `ann_eng` | **PS3 09–Legacy**, 24 000 Hz mono (8 trees) | X360 11–Legacy, 32 000 Hz | X360 09 / 10, 24 000 Hz | — |
| `pbp_eng` (unmeasured trees) | **PS3 09 / 10 / 12**, 24 000 Hz mono | X360 11–14, PS3 14, 32 000 Hz | X360 09 / 10 | (X360 14: one sampled offset beyond the truncated loose stream) |
| `music_samples` | **X360 13 / 14 / 15 / Legacy, PS3 11 / 14 / 15 / Legacy**, 44 100 Hz **stereo** (`channel_config` 1) | — | X360 09 / 10 / 11 / 12, 44 100 Hz stereo | PS3 09 / 10 / 12 / 13: no loose music bank (archive members not surveyed) |
| `000_goal.abk` | **PS3 11 / 14 / 15 / Legacy** (44 100; stereo on 11 / 14 / 15, mono on Legacy) | — | X360 10–Legacy | X360 09, PS3 09 / 10 / 12 / 13: no loose `.abk` |
| `000_chant.abk` | **PS3 11 / 14 / 15 / Legacy**, 44 100 stereo | — | X360 10–Legacy, 32 000 stereo | same |
| `sfx.abk` | **PS3 11** (22 050 mono), **PS3 14 / 15 / Legacy** (44 100 mono) | — | X360 10–Legacy (22 050 / 36 000 / 44 100) | same |

## 2. R77 grammar per codec-`0x05` tuple (established = every vector tiles, granule rule holds, header constant, rebuild decodes to `granules × 576` samples with the right channel count)

| Tuple | trees | vectors | tiles | granule rule + PCM frame (2nd EA-frame of block 0, 47 samples) | decoded = granules × 576 | malformed refused (3 / 3 per class) | status |
|---|---|---|---|---|---|---|---|
| `ann_eng`, 24 000 Hz mono, SNS blocks | 8 (PS3 09–Legacy) | 32 | 32 | 32 | 32 | 8 / 8 | **established** |
| `pbp_eng`, 24 000 Hz mono, SNS blocks | 3 (PS3 09 / 10 / 12) | 12 | 12 | 12 | 12 | 3 / 3 | **established** (joins the six R77 sets) |
| `music_samples`, 44 100 Hz **stereo**, SNS blocks | 8 | 24 | 24 | 24 | 24 (2 channels) | 8 / 8 | **established** — per-channel side info (scfsi × 2, 12 + 47 bits per channel), main data ch0 ‖ ch1 per granule, 32-byte MPEG-1 side info in the rebuild, header `channel_mode` / `mode_extension` carried as read |
| `.abk` raw streams (no SNS blocks; S10A entry: SNR h1 h2, u32 size **counting the 8 SNR bytes**, u32 h2, then `size − 8` stream bytes) | 4 (PS3 11 / 14 / 15 / Legacy) | 28 | 24 | 24 | 24 | 4 / 4 | **established** for 24 / 28; **residual**: `000_chant.abk` entry 1 (byte-identical on the four trees) does not tile from the computed entry offset (no PCM frame found, 67 373 samples short) — the S10A offset / size reading for that entry is not established; not forced |

Every codec-`0x05` stream, including `.abk` raw streams and stereo music, carries the same first-block structure (two warm-up granules
+ one 47-sample PCM frame as the second EA-frame; `granules × 576 + 47 − num_samples` = 1 152 + tail trim). The `.abk` entry-size rule
(size counts the SNR header) was found on the first full run — the trailing 8 bytes of every entry were the next entry's SNR words —
and corrected before the rerun.

Malformed input (one vector per class): last block truncated by 3 bytes → "EOF inside granule"; flag byte `0x01` → "pcm_flag 01";
PCM count overwritten with 4 096 → "PCM block exceeds payload" — refused on 100 % of the codec-`0x05` classes (23 / 23 vectors tested).
Reservoir: `main_data_begin` reached 511 (MPEG-1) / 255 (MPEG-2) and never exceeded on any rebuild.

## 3. Consequence (developer; not Codex-accepted) — queue row 12

The row-9 diagnostic preview contract applies, per measured tuple, to: PS3 `pbp_eng` 09–13 / 15 / Legacy (24 kHz on 09–13, 44.1 kHz on
15 / Legacy; **PS3 14 pbp is `0x09`**), X360 `pbp_eng` 15 / Legacy, PS3 `ann_eng` 09–Legacy, `music_samples` on X360 13–Legacy and PS3
11 / 14 / 15 / Legacy (stereo rebuild), PS3 `.abk` sfx (`size − 8` stream; the chant entry-1 residual refused by name). Not covered:
X360 announcer / pbp 11–Legacy and PS3 14 pbp (`0x09`), X360 09 / 10 speech, X360 09–12 music, every X360 `.abk` (EA-XMA); PS3 09 /
10 / 12 / 13 music and PS3 12 / 13 sfx (archive members, not surveyed). The parser contract gains the three malformed-input refusals
and the reservoir / count assertions. The label stays "diagnostic preview — start handling unresolved; not game-verified".

## 4. Reproduce

```
set R80_SCRATCH=<scratch dir>
python local/runs/overnight-research-2026-09-22-pm/probes/r80_gen7_codec05_survey.py docs/research/overnight-2026-09-22-pm/data/r80-gen7-codec05-survey.json   (≈ 10 min)
```
Executed: one smoke run (X360 NHL 15; exposed the stereo rebuild gap), two full runs (the first exposed the `.abk` size rule; the
second is the published JSON). The R77 probe gained the stereo rebuild (`nch`-aware side info and main-data interleave). Temporary audio
removed by the probe.

> **Note 2026-09-24 (R82, [2026-09-24 handoff 01](../overnight-2026-09-24/01-gen7-archive-audio-coverage-and-abk-records.md)):** the `.abk` entry rule in §2 ("S10A entry: SNR h1 h2, u32 size counting the 8 SNR bytes, u32 h2, then size − 8 stream bytes") holds for non-loop ram records only; loop-flagged records carry `u32 loop_start` before one or two `u32 size | u32 samples | bytes` segments (Σ samples = num_samples), stream-type records are 8 / 16-byte stubs, and 0xFFFFFFFF table offsets are absent slots. The `000_chant.abk` entry-1 residual is explained by that record layout (the loop_start word was read as the size), not by the codec: its codec bytes tile and decode (70 272 samples). The "not surveyed" cells of §1 (PS3 09 / 10 / 12 / 13 music, PS3 12 / 13 sfx) are now measured there: established. `channel_config` 3 / 5 entries are two / three interleaved stereo streams (named, not decoded).
> **Note 2026-09-24 (R83 / R84, [2026-09-24 handoff 02](../overnight-2026-09-24/02-codec09-easpeex-and-ea-xma-preview.md)):** the `0x09` column of §1 is identified — EA-Speex (Speex UWB 32 kHz, u8-length frames, 509-sample delay), diagnostic decode on 96 / 96 vectors; the EA-XMA column has a diagnostic RIFF-XMA2 rewrap route (91 / 91 vectors).

# Handoff R49 — PS4 EA-Opus (NHL 18–21) sample-preview decode contract: u16-prefixed 20 ms CELT-FB mono packets re-framed as Ogg Opus decode with the local ffmpeg on 24 / 24 vectors; decoded length = 960 × packets, declared length = `.DUR` × 48 000 (the difference ≤ 1 258 samples); no encoder claimed

Research lane, 21 September 2026 pm, roadmap step 1.5 / 2.7 / 3.5 PS4 (prompt step 4: "prioritize usable sample-preview decoder contracts
with source identities and known vectors, while keeping decode and encode claims separate"; continues the 2026-09-21 handoff 05 which
verified the packet framing but attempted no decode). Claim `research-r49-ea-opus-preview`. Read-only on the banks; the only writes were
24 temporary Ogg + WAV files in the session scratchpad, removed by the probe. No listening review (the decoded PCM was measured, not heard).

Machine-readable: [`data/r49-ea-opus-preview-vectors.json`](data/r49-ea-opus-preview-vectors.json) (per title: bank, DSET, 6 vectors with
`.SID`, `.OFF`, SNS byte length + SHA-256, SNR header, block sample counts, packet count, TOC configurations, decoded WAV frames / channels /
rate, PCM peak and PCM SHA-256). Probe `r49_ea_opus_preview_vectors.py` (≈ 1 min; needs `ffmpeg` on PATH — 8.1.1 gyan.dev full build here).

## Plain-language summary

The PS4 commentary samples on NHL 18–21 are stored as ordinary Opus packets, each prefixed by a two-byte length, inside EA's block stream.
Tonight's probe takes 24 samples (six per title), splits the packets, reads each packet's one-byte header (every one is a 20 ms full-band
CELT mono packet), wraps them in a standard Ogg Opus container and hands that to the ffmpeg that is already installed: all 24 decode to
48 kHz mono PCM whose length is exactly 960 samples × the packet count, and whose loudness peaks well above silence. The bank's declared
sample count (which equals the record's duration × 48 000 on every vector) is 359–1 258 samples shorter than the decoded length, i.e. the
encoder's priming plus the last packet's padding; a preview should trim to the declared count. That is a complete, reproducible decode
contract for the PS4 EA-Opus class — the first speech class Studio can preview. It says nothing about encoding: no EA-Opus encoder or
re-packer is claimed, and replacing a sample stays refused.

## 1. Vectors (six per title; `pbp_samples_inxp.sbr` / `.sbs`, the largest DSET of each bank; identities in the JSON)

| Title | DSET (records) | `.SID` | SNS bytes | packets | TOC (all packets) | decoded frames = 960 × packets | declared `num_samples` (= `.DUR` × 48 000) | excess |
|---|---|---|---|---|---|---|---|---|
| NHL 18 | 4092 (2 596) | 15466 / 36985 / 40440 / 44448 / 47977 / 88943 | 3 548 / 2 240 / 4 072 / 2 512 / 2 644 / 2 812 | 26 / 16 / 30 / 18 / 19 / 21 | config 31 = CELT-FB 20 ms, mono, code 0 | 24 960 / 15 360 / 28 800 / 17 280 / 18 240 / 20 160 | 24 601 / 14 406 / 27 875 / 16 518 / 17 194 / 19 179 | 359 … 1 046 |
| NHL 19 | 4459 (2 777) | 7239 / 38600 / 44104 / 69517 / 95692 / 100423 | 6 232 … 7 676 | 47 / 40 / 29 / 36 / 58 / 55 | same | 45 120 … 55 680 | 43 983 … 54 744 | 508 … 1 137 |
| NHL 20 | 2658 (2 460) | 7748 / 12744 / 21863 / 23123 / 23616 / 25373 | 2 492 … 4 188 | 18 / 31 / 28 / 25 / 25 / 26 | same | 17 280 … 29 760 | 16 835 … 29 065 | 445 … 1 258 |
| NHL 21 | 3693 (3 237) | 7748 / 12901 / 22175 / 23592 / 32577 / 38843 | 2 492 … 3 576 | 18 / 26 / 20 / 25 / 26 / 17 | same | 17 280 … 24 960 | 16 835 … 24 424 | 445 … 1 235 |

Checks (24 / 24 on every row): SNR codec `0x0c`, 48 000 Hz, channel config 0; the u16 BE length prefixes tile every data-block payload
exactly; every TOC byte is `0xF8` (config 31, mono, one frame); ffmpeg return code 0; WAV = 48 000 Hz, 1 channel, 16-bit; decoded frames =
Σ TOC samples = 960 × packets; PCM peak 19 439–31 040 (not silence); declared `num_samples` = round(`.DUR_f` × 48 000). NHL 20 and NHL 21
share sample 7748 byte-for-byte (SNS SHA-256 `5f19a9e8…`, PCM `7e210a63…`) — bank content is reused across those two titles for that id.

## 2. Contract (decode / preview only)

1. Locate the sample: `.sbr` DSET record → `.OFF` into the sibling `.sbs` (streamed banks) or `.RAM` into the embedded data chunk; walk the
   SNS blocks (`0x48` header carrying the SNR, `0x44` data blocks, `0x45` end) as `snd10_sbr.parse_blocks` does; refuse a codec ≠ `0x0c`,
   a rate ≠ 48 000 or a channel config ≠ 0 (nothing else was measured).
2. Concatenate the data-block payloads' packets: repeat `u16 BE n | n bytes`; refuse a prefix that does not tile the payload (0 / 24 observed).
3. Decode with libopus (via ffmpeg / Ogg framing as here, or the raw API): 48 kHz output; expect 960 samples per packet (refuse a TOC
   whose config / frame count differs from the measured single shape until measured — none observed).
4. Present `num_samples` samples: the decoded stream is 359–1 258 samples longer. Whether the discarded samples are leading (encoder
   priming ≈ 300) or trailing (last-packet padding < 960) or both is **not established** statically — a preview may trim the tail; a
   waveform-level alignment card is unnecessary for preview but would be for any sample-accurate use.
5. Source identity: the decode applies to the files a session opens per the 2026-09-20 handoff 02 §4 chain (`pbp_samples*.sbr/.sbs` under
   `audio/speech/eng`); the vectors above are the reproducibility set (SNS SHA-256 per vector).

## 3. Not claimed (kept separate)

- **Encoding / replacement:** no EA-Opus encoder, no packet re-framer, no proof that the console accepts a stream with different packet
  sizes or counts; the same-slot bound and alias-set consequences of handoff 05 stand — replacement stays refused.
- **Other classes:** EA-MP3 (PS4 15–17), EALayer3 / EA-XMA / codec `0x09` (gen7) remain undecoded locally (handoff 05).
- **Playback semantics:** which sample the game picks and when; the meaning of the data blocks' `samples-in-block` word (10 248 per
  full block on every vector; not the packet sum).
- **Runtime:** nothing.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r49_ea_opus_preview_vectors.py docs/research/overnight-2026-09-21-pm/data/r49-ea-opus-preview-vectors.json --titles nhl18,nhl19,nhl20,nhl21 --per-title 6   (≈ 1 min; ffmpeg on PATH; --keep retains the Ogg / WAV in %TEMP%/r49_ea_opus or $R49_SCRATCH)
```
Executed: one 3-vector NHL 21 smoke run (after two import / table fixes) and the final 24-vector run; temporary Ogg / WAV files removed by the
probe (verified: the scratch directory is empty). Not executed: listening, encoders, writes to any source.

# Handoff R28 / R29 — Gen7 speech banks on every title (09–Legacy, PS3 + Xbox 360): aliasing, stream facts, replacement headroom, the codec map, and what a local decoder can do today

Research lane, 21 September 2026, Priority 1.1 (bank index and sample-range interpretation, codec identification and existing-decoder
feasibility, sample sharing / aliases, stored versus decoded sizes, header / offset consequences of replacement). Claims
`research-r28-gen7-bank-streams`, `research-r29-codec-feasibility`. Nothing game-verified; read-only on every source (one sample per codec
class was copied into the session scratchpad for the decode attempts and deleted); no launch; **no writer, no encoder**.

Machine-readable: [`data/r28-gen7-bank-streams.json`](data/r28-gen7-bank-streams.json) (32 banks) and
[`data/r29-codec-feasibility.json`](data/r29-codec-feasibility.json). Probes `r28_gen7_bank_streams.py` (≈ 40 s; sampled 400 entries + first /
last per bank) and `r29_codec_feasibility.py` (≈ 2 s; needs the winget `ffmpeg` 8.1).

## Plain-language summary

Every gen7 commentary bank (play-by-play and rink announcer, 09 through Legacy, both consoles) is now measured the same way: how many
sample ids it has, how many of them are aliases of the same audio, where the audio lives, and what codec it is in. Two facts matter for
any future replacement: (1) on NHL 10 and later, roughly half of all sample ids are **aliases** — several ids share one 6-byte record and
one stream offset (up to 40 ids on 10–14, up to 68 on 15 / Legacy) — so replacing "a sample" replaces it for every id that aliases it;
(2) samples are packed with **0–12 bytes** between them, so an in-place replacement has no headroom beyond the original stored length.
The codec is not one thing: Xbox 360 uses EA-XMA on 09 / 10, an unidentified 32 kHz codec (`0x09`; the community table calls it EA-Speex,
unverified) on 11–14 and for the announcer on 11–Legacy, and EALayer3 v1 for play-by-play on 15 / Legacy; PS3 uses EALayer3 v1 everywhere
except NHL 14 play-by-play (`0x09`). Of the five codec classes, the locally installed ffmpeg decodes none of them as shipped: EA-Opus
(PS4 18–21) is the closest (its packets are plainly length-prefixed Opus and libopus can decode them), EA-MP3 (PS4 15–17) needs its
per-block prefix understood, EA-XMA needs EA's packetization, `0x09` needs identification, and EALayer3 has no ffmpeg decoder at all.

## 1. Bank census (measured; `SBKR` + `.sbs`; the same numbers on both platforms unless stated)

| Title | pbp ids (range) | ids aliasing a record / records shared / max ids per record | distinct offsets | ann ids (range) | ann aliases | stream identity |
|---|---|---|---|---|---|---|
| NHL 09 | 45 589 (77 269–136 903) | **0 / 0 / 1** | 45 589 | 14 132 (2–14 322) | 0 | loose `.sbs` X360 1 313 694 092 B, PS3 1 371 025 504 B (in-archive `.sbr`, refpack) |
| NHL 10 | 53 276 (77 269–161 966; PS3 53 278) | 23 314 / 5 640 / 40 | 35 602 | 13 496 (2–18 482) | 303 / 148 / 6 | loose X360 1 484 614 140, PS3 1 533 561 864 |
| NHL 11 | 61 576 (77 269–177 171) | 27 718 / 6 387 / 40 | 40 245 | 14 515 (45–33 503) | 70 / 35 / 2 | X360 in `audiostream.big` (612 402 324 B, no loose copy); PS3 loose = `audiostream.big` member size |
| NHL 12 | 73 194 (77 269–193 761) | 35 132 / 9 174 / 40 | 47 236 | 18 148 (45–63 264) | 3 040 / 1 520 / 2 | X360 loose = `nocache.big` member; PS3 loose 1 912 589 672 (ann from `nocache.big`) |
| NHL 13 | 83 453 (77 269–208 407) | 42 780 / 11 001 / 40 | 51 674 | 21 312 (45–66 455) | 3 040 / 1 520 / 2 | X360 loose = member 751 296 056; PS3 loose 2 094 403 520 (ann from `nocache.big`) |
| NHL 14 | 89 734 (1–91 905) | 45 591 / 12 126 / 40 | 56 269 | 23 255 (0–68 492; **8 entries without an offset**, `count ≠ 1`) | 3 026 / 1 510 / 8 | **X360 loose truncated** (650 117 120 < member 820 131 756; 63 / 402 sampled offsets beyond it); PS3 loose 1 128 772 804 |
| NHL 15 | 45 442 (58–55 834) | 29 399 / 6 956 / **68** | 22 999 | 24 876 (45–78 673) | 3 018 / 1 509 / 2 | X360 loose = member 731 829 012; PS3 loose 1 153 780 608 |
| Legacy | 59 940 (58–75 273) | 37 958 / 9 709 / 68 | 31 691 | 24 841 (45–82 778) | 2 148 / 1 058 / 4 | X360 loose = member 789 000 896; PS3 loose 1 095 933 360 |

Other facts on every bank: ids sorted; every `count` field = 1 and every 6-byte record's first word = 0 except the 8 NHL 14 ann entries;
offsets are *not* in id order; every sampled offset lands on an SNS `0x48` header block and walks to a `0x45` end block (402 / 402 per bank
except the truncated NHL 14 X360 stream); the gap from a sample's end to the next sample's offset is **0–12 bytes** on every bank (≈ 25 % of
samples have 0). The NHL 09 banks have no aliases at all; aliasing starts with NHL 10.

## 2. Codec map (measured on 400 sampled samples per bank; channel config 0 = mono everywhere)

| Platform | Title | pbp (play-by-play) | ann (rink announcer) |
|---|---|---|---|
| Xbox 360 | 09, 10 | EA-XMA (`0x03`), 24 000 Hz, ≈ 9.9 kB/s | EA-XMA, 24 000 Hz |
| Xbox 360 | 11, 12, 13, 14 | **`0x09`**, 32 000 Hz, ≈ 3.7 kB/s | `0x09`, 32 000 Hz |
| Xbox 360 | 15, Legacy | EALayer3 v1 (`0x05`), 44 100 Hz, ≈ 10.6 / 9.0 kB/s | `0x09`, 32 000 Hz |
| PS3 | 09–13, 15, Legacy | EALayer3 v1, 24 000 Hz (09–13) / 44 100 Hz (15, Legacy), ≈ 10.5 / 16.4 / 12.0 kB/s | EALayer3 v1, 24 000 Hz |
| PS3 | 14 | **`0x09`**, 32 000 Hz, ≈ 5.1 kB/s | EALayer3 v1, 24 000 Hz |

(The PS4 map of 2026-09-20 handoff 02 stands: `0x0b` EA-MP3 on 15–17, `0x0c` EA-Opus on 18–21, both 48 000 Hz.) Stored bytes per sample:
medians 5–30 kB; the SNR header's `num_samples / rate` gives the duration (no `.DUR` column on `SBKR`).

## 3. Decoder feasibility (one sample per class, local ffmpeg 8.1; `r29-codec-feasibility.json`)

| Class | Payload framing (measured) | Attempt | Result |
|---|---|---|---|
| EALayer3 v1 (`0x05`; PS3 all, X360 15 / Legacy pbp) | data blocks begin `00 cc …` (EALayer3 frame header) | none possible — ffmpeg has no EALayer3 decoder | **no local decoder**; the public vgmstream implements EALayer3 v1 (not consulted) |
| `0x09` (X360 11–15 / Legacy, PS3 14 pbp) | data blocks are not u16-length-prefixed (`6e 3b 84 81 …`); ≈ 905 B per 6 531 samples | none (identification first) | **unidentified**; community EA SNS codec table names `0x09` EA-Speex (unverified); ffmpeg's `speex` decoder would need the frame size and Ogg framing |
| EA-XMA (`0x03`; X360 09 / 10) | each data block payload starts with a u32 (`0x1f54`, `0x1e7c` … — candidate bit length) then XMA frames | synthesized RIFF `WAVEFORMATEX 0x166` (XMA2, block 0x800) + ffmpeg `xma2` | **decoder ran but rejected the frames** ("would have to skip N bits"): EA's XMA packetization differs from RIFF XMA2; needs the EA-XMA re-packing rule |
| EA-MP3 (`0x0b`; PS4 15–17) | each data block payload = a prefix (6 bytes on the first block, 2 on the others: `f6 89 80 04 00 00`, `22 01` / `f2 00`) then MPEG-1 Layer III frames (`ff fb 64 c4` = 48 kHz mono 80 kbps) | ffmpeg `-f mp3` on the raw payloads: "no two consecutive frames"; after stripping to the first sync per block: "Header missing" (a 4.6 kB partial WAV) | **not decodable as plain MP3**; the per-block prefix and the frame continuation across blocks must be understood (EA-MP3's frame layout) |
| EA-Opus (`0x0c`; PS4 18–21) | each data block payload = **u16 BE length-prefixed Opus packets that tile the block exactly** (174, 99, 114, 129 × 8 …; 10 248 samples per block) | not attempted (needs Ogg packetization or the raw libopus API) | **decodable in principle with libopus** — the only class with a plain, verified packet framing |

## 4. Consequences for the replacement contract (extends 2026-09-20 handoff 02 §6; still a proposal, no writer)

1. **Aliases:** an in-place replacement at an offset changes the audio of every id sharing that record / offset (up to 40 / 68 ids). A
   listing must show the alias set; a "replace this line" operation must state it replaces all of them; a "replace for one id only" needs a new
   stream span **and** a new 6-byte record (append at the end of the `.sbs`; `SBKR` offsets are absolute u32, so growth is expressible) — the
   `.sbr` also lives in `audioboot.big` (chunklzx / chunkzip) on 11+ and in BIG4 refpack on 09 / 10, so a record change is an archive-member
   change under the packaging rules of the 2026-09-20 handoffs 03 / 10.
2. **Headroom:** same-slot replacement is bounded by the original stored length + 0–12 bytes; anything longer is an append.
3. **Codec per slot:** a replacement must match the slot's codec (`0x09` on X360 11–14 pbp, EALayer3 on PS3, XMA on X360 09 / 10 …), rate
   and channel config; today no encoder exists for any of them locally (ffmpeg encodes MP3 / Opus / Speex but not EA's framings, and
   nothing for EALayer3 / XMA).
4. **Truncated NHL 14 X360 stream:** refuse the loose file (handoff 02 §2); the 8 NHL 14 ann entries without an offset are listed, not
   played.
5. **Listing / preview status:** catalogue (id, alias set, offset, stored bytes, codec, rate, duration) is implementable now on every gen7
   title; **preview** (PCM) is blocked on every gen7 class and on PS4 EA-MP3; PS4 EA-Opus preview is the first realistic target
   (packet framing verified).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r28_gen7_bank_streams.py docs/research/overnight-2026-09-21/data/r28-gen7-bank-streams.json   (≈ 40 s; --sample N, --titles a,b)
python local/runs/overnight-research-2026-09-21/probes/r29_codec_feasibility.py  docs/research/overnight-2026-09-21/data/r29-codec-feasibility.json  (≈ 2 s; ffmpeg on PATH)
```
Inputs: the 16 gen7 tree roots (`.sbr` loose or from `audioboot.big` / `audio.big` / `data0.big`; `.sbs` loose or from `nocache.big` /
`audiostream.big`), the PS4 `speech/eng/pbp_samples.{sbr,sbs}` of CUSA00561 / CUSA18182 (H:). Executed: r28 (2 runs), r29 (4 development runs
+ 1 final). Temporary: `r28_*.sbr`, the five per-class sample copies and two partial WAVs in the scratchpad — removed. Not executed:
launches, writes, encoders.

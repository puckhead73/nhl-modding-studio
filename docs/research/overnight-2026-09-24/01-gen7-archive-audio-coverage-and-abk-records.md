# Handoff R82 — archive-backed gen7 audio coverage completed (PS3 09 / 10 / 12 / 13 music and `.abk` sound-effect banks, PS3 12 / 13 horn sources, Xbox 360 NHL 09 `.abk` codec identity), and the R80 chant entry-1 residual **explained by the `ABKC` sample-record grammar**, not by the codec: loop-flagged records carry a loop-start word and one or two `size | samples | codec-bytes` segments; every one of 18 124 loose + 17 582 archived records tiles under that grammar with zero failures; two further record classes (4- and 6-channel entries as interleaved stereo EA-frame streams) and the 1-sample placeholder banks are named as classes, not forced

Research lane, 24 September 2026, 04:57 → 05:23 MDT (brief item R1; Leagues 1.5 / Teams 2.7 / Misc 5.4 gen7). Claim
`research-r82-gen7-archive-audio-coverage`. Read-only over the trees on `X:` (PS3 09 / 10 / 11 / 12 / 13 / 14 / 15 / Legacy, Xbox 360
NHL 09) and the committed anchors; archive members decoded in memory; temporary rebuilt `.mp3` / decoded `.wav` in the session
scratchpad, removed by the R77 decode helper. Nothing listened to; nothing game-verified; no encoder; no runtime claim. (This file was
re-created at 05:50 MDT after a failed text-encoding write emptied the 05:23 original; content unchanged except this note and the two
bank identities marked "filled".)

Machine-readable: [`data/r82a-abk-entry-records.json`](data/r82a-abk-entry-records.json) (every loose PS3 `.abk` record on four trees)
and [`data/r82b-gen7-archive-audio-coverage.json`](data/r82b-gen7-archive-audio-coverage.json) (archives, copies, music vectors,
every archived `.abk` record on five trees, decoded vectors, horn joins). Probes: `r82_abk_entry_records.py` (≈ 2 min) and
`r82_gen7_archive_audio_coverage.py` (≈ 35 min, dominated by pure-Python RefPack and the full decode of every `sfx.abk` ram entry —
a wider decode population than planned; kept as evidence).

## Plain-language summary

The last "not surveyed" cells of the older-console audio table are now measured from the archives themselves: the menu-music banks of
PlayStation 3 NHL 09, 10, 12 and 13 and the sound-effect banks of those four titles parse, rebuild and decode with the same grammar as
the loose titles, and Xbox 360 NHL 09's archived sound-effect banks are EA's XMA like every other Xbox bank. The one sample that would
not parse yesterday was never a codec problem: the bank's record for a *looping* sound has a different header than a one-shot sound —
an extra word naming where the loop starts, then one or two audio segments (an intro and the loop body) whose sample counts add up to
the declared total. Reading records that way, every record in every bank on eight PlayStation 3 titles and one Xbox title ends exactly
where the next begins. Two more shapes were found and are recorded rather than forced: sounds with four or six channels are stored as
two or three stereo streams interleaved frame by frame, and a handful of loading-loop banks hold a one-sample placeholder. The horn
numbers that the European and minor-league arenas use are absent from the PlayStation 3 NHL 12 / 13 archives too, exactly as on the
loose titles.

> **Note 2026-09-26 01:18 (R110b, [overnight-2026-09-26/04-archive-backed-bank-source-map.md](../overnight-2026-09-26/04-archive-backed-bank-source-map.md) §2):** the "RefPack" / "chunkzip" labels on the archived `.abk` population rows (§1) and `.abk` coverage rows (§3) are wrong for the banks: every archive-backed `.abk` / `.ast` member on PS3 09 / 10 / 12 / 13 and X360 09 starts with its payload magic (`ABKC` / `S10S`, 959 + 47 members) — stored, unwrapped. The labels describe the music `.sbr` members. Record counts and grammar results are unaffected.

## 1. `ABKC` sample-record grammar (observed structure; supersedes the 2026-09-20 handoff 02 §2.2 record line and the R80 `.abk` entry rule)

```
file    'ABKC' … | 0x14 u32 file size | 0x18 u32 sample-table offset | 0x1c 0x78 | 0x20 table offset again | 0x24 u32 data size (4-aligned)
table   'S10A' | u32 0 | u32 count | u32 header length = 16 + 4·(count − 1)  (0 when count = 0)
        | (count − 1) × u32 offset of samples 2..n from the table start; 0xFFFFFFFF = absent slot (no record; 153 slots across the 862 loose files)
record  SNR h1 | SNR h2            h2: bits 31–30 type (0 ram, 1 stream), bit 29 loop, bits 28–0 num_samples
  ram, no loop    : u32 size (counts the 8 SNR bytes) | u32 samples (= num_samples) | (size − 8) codec bytes
  ram, loop       : u32 loop_start | { u32 size | u32 samples | (size − 8) codec bytes } × 1..2, Σ samples = num_samples;
                    loop_start = 0 with one segment, = segment-1 samples with two (intro + loop body)
  stream, no loop : nothing more (8-byte record; the audio is in the sibling `<bank>.ast`)
  stream, loop    : u32 a | u32 b (16-byte record; a = 29 and b ∈ {256, 512, 1472} on the crowd / ambience entries seen — the `.ast`'s
                    first block header words on one checked entry; not interpreted)
records are not padded; the next record starts at the previous one's end; the last record ends 0–3 bytes before table + data size.
```

Tests over the whole population (per record, not sampled):

> **Note 2026-09-26 03:45 ([overnight-2026-09-26/13-abk-record-total-reconciliation.md](../overnight-2026-09-26/13-abk-record-total-reconciliation.md)):** the archived rows' "records" are present records; their absent slots ("—" below) are in the JSON: PS3 09 25, PS3 10 24, PS3 12 37, PS3 13 43, X360 09 25. Present + absent = the table slot count R98 reports. The RefPack / chunkzip labels on these rows describe music members, not the banks (R110b).

| Population | files / members | records | absent slots | ram | ram+loop (1 seg / 2 seg) | stream | stream+loop | T1 tiling fail | T2 Σ-samples fail | T4 loop_start fail |
|---|---|---|---|---|---|---|---|---|---|---|
| loose PS3 NHL 11 | 210 | 2 816 | 24 | 1 570 | 138 (91 / 47) | 1 104 | 4 | 0 | 0 | 0 |
| loose PS3 NHL 14 | 218 | 5 110 | 43 | 1 277 | 140 (85 / 55) | 3 684 | 9 | 0 | 0 | 0 |
| loose PS3 NHL 15 | 217 | 5 099 | 43 | 1 277 | 140 (85 / 55) | 3 673 | 9 | 0 | 0 | 0 |
| loose PS3 Legacy | 217 | 5 099 | 43 | 1 281 | 136 (94 / 42) | 3 673 | 9 | 0 | 0 | 0 |
| archived PS3 NHL 09 (`audioboot.big` + `data0.big`, RefPack) | 186 | 2 594 | — | 1 951 | 137 | 504 | 2 | 0 | 0 | 0 |
| archived PS3 NHL 10 (`audio.big` / `audioboot.big` / `audioboot2.big` / `stadium.big`, RefPack) | 199 | 2 748 | — | 1 512 | 128 | 1 104 | 4 | 0 | 0 | 0 |
| archived PS3 NHL 12 (`nocache.big` / `audioboot.big` / `audioboot2.big`, chunkzip) | 219 | 4 697 | — | 834 | 144 | 3 710 | 9 | 0 | 0 | 0 |
| archived PS3 NHL 13 (same + `cacheaudio.big`) | 213 | 5 117 | — | 1 272 | 137 | 3 699 | 9 | 0 | 0 | 0 |
| archived X360 NHL 09 (`audioboot.big` + `data0.big`) | 186 | 2 426 | — | 1 816 | 137 | 471 | 2 | 0 | 0 | 0 |

The residual chant entry (`000_chant.abk` entry 1, byte-identical on PS3 10 / 12 / 13 / 11 / 14 / 15 / Legacy, stored SHA-256
`28bedd55…` for the whole bank) reads `05 04 ac 44 | 20 01 07 2d | 00 00 00 00 | 00 00 79 e5 | 00 01 07 2d | 00 c6 49 73 …`: loop bit
set, loop_start 0, one segment of 31 205 bytes / 67 373 samples, ending exactly at entry 2. R80 read the loop_start word as the size
(0) — a record-interpretation error, now corrected; the codec bytes tile under R77 and decode to 70 272 samples (trim 1 794).
Two-segment example (`055_chant.abk` entry 0, PS3 NHL 15): loop_start 5 636; segments (3 533 bytes / 5 636 samples), (66 490 bytes /
137 472 samples); 5 636 + 137 472 = 143 108 = num_samples.

## 2. Codec-`0x05` segment classes by `channel_config` (parse-only test T3 on every ram segment; decode on the vectors of §3)

| `channel_config` | reading (measured) | R77 shape | segments checked (loose 4 trees) | ok | residual |
|---|---|---|---|---|---|
| 0 | mono | one EA-frame stream; PCM frame = frame 1, 47 samples; G + 47 − samples = 1 152 + trim, trim 2–3 granules (4 on 4 entries) | 4 399 | 4 399 | 4 entries at exactly trim = 4 granules (`colors1/2`, `playercalls`, `ambientskatesandpunches`) — accepted by the widened window, listed |
| 1 | stereo | one 2-channel stream (R80 stereo side info) | 1 482 | 1 476 | 6 = the `loadloop*` 1-sample placeholders: num_samples 1, PCM frame 1 sample, 6 granules |
| 3 | 4 channels | **two** stereo EA-frame streams interleaved frame by frame: frames 0 / 1 warm-up, frames 2 / 3 PCM (47 each), G + 94 − 2·samples = 2 304 + trim (4–7 granules) | 265 | 265 | — |
| 5 | 6 channels | **three** stereo streams interleaved (PCM frames 3 / 4 / 5, 141 samples; G = 3 × (samples + 1 152 + trim)) | 4 (loose) + 25 (archived) | — | **unclassified**: the shape is consistent on all 29 but no decode was attempted (a de-interleave rule for cc 3 / 5 is an inference from the frame order, not a measured output) |

`channel_config` = channels − 1 is the reading consistent with all four values; it is measured on the frame pattern, not on any audible output.

## 3. Coverage table — archive-backed tuples (replacing R80's "not surveyed" cells)

| Tuple | source identity | vectors | tiles / granule rule / PCM frame | decoded = granules × 576 × ch | malformed refused | status |
|---|---|---|---|---|---|---|
| PS3 NHL 09 `music_samples`, EALayer3 v1 44 100 Hz stereo, SNS blocks | `.sbr` `audioboot.big:audio/music/music_samples.sbr` (RefPack, 2 304 → 3 502 B, SHA-256 `a7c01f3a…`; 166 entries); `.sbs` `data0.big` member @ 5 362 688, 331 871 344 B (head `SBKS`) — no loose copy | 3 (offsets 16 / 147 278 768 / 331 391 696; SNS SHA-256 `d1ffd96d…`, `c610b17f…`, `f7a37c14…`) | 3 / 3 / 3 | 3 / 3 (789 120 / 1 134 720 / 935 424) | 3 / 3 | **established** |
| PS3 NHL 10 `music_samples`, same | `.sbr` `audioboot.big` member (RefPack, 2 443 → 3 722 B, SHA-256 `2cb000c5…` — filled; 177 entries); `.sbs` **loose** `audio/music/music_samples.sbs` 356 581 276 B (no archive copy) | 3 | 3 / 3 / 3 | 3 / 3 | 3 / 3 | **established** |
| PS3 NHL 12 `music_samples`, same | `.sbr` `audioboot.big` (chunkzip, 1 981 → 3 842 B, `8907cd9d…`; 183 entries); `.sbs` `nocache.big` @ 17 440, 338 841 488 B | 3 (SNS `24b8e2c4…`, `1bbcd0b0…`, `c8d81a39…`) | 3 / 3 / 3 | 3 / 3 | 3 / 3 | **established** |
| PS3 NHL 13 `music_samples`, same | `.sbr` `audioboot.big` (chunkzip, 2 110 → 4 082 B, `b08ea083…` — filled; 195 entries); `.sbs` `nocache.big` @ 16 304, 368 780 068 B | 3 | 3 / 3 / 3 | 3 / 3 | 3 / 3 | **established** |
| PS3 NHL 09 `.abk` (`000_goal` 2 entries cc 1, `000_period` cc 0, `000_chant` 3 entries cc 1, `sfx.abk` 479 ram entries cc 0 / 1) | `data0.big` (stored) / `audioboot.big` (RefPack) | 485 segments | 485 | 485 / 485 | 4 / 4 | **established** |
| PS3 NHL 10 `.abk` (same four banks: 1 + 1 + 3 + 509) | `audio.big`, `audioboot2.big` (RefPack) | 514 | 514 | 514 / 514 | 4 / 4 | **established** |
| PS3 NHL 12 `.abk` (1 + 1 + 3 + 196) | `nocache.big`, `audioboot2.big` (chunkzip) | 201 | 201 | 201 / 201 | 4 / 4 | **established** |
| PS3 NHL 13 `.abk` (1 + 1 + 3 + 196) | `nocache.big`, `audioboot2.big` (chunkzip) | 201 | 201 | 201 / 201 | 4 / 4 | **established** |
| X360 NHL 09 `.abk` (all 186 members, 2 426 records) | `audioboot.big`, `data0.big` | — | — | — | — | **EA-XMA (`0x03`) on 2 426 / 2 426 records** (22 050 / 24 000 / 36 000 / 44 100 Hz; cc 0 / 1 / 3 / 5) — not this codec; records tile under §1 |
| X360 NHL 09 `music_samples` | loose `.sbr`; `.sbs` loose = `data0.big` member (both 303 521 160 B) | 3 | — | — | — | **EA-XMA** 44 100 stereo — not this codec |

Coverage class: the music rows are 3-vector samples (first / middle / last `.sbs` offset); the `.abk` rows are complete enumerations of
the named banks' ram entries (every entry decoded), and the grammar test is a complete enumeration of every record in every member.
Speech on these titles was already covered (R77 / R80: PS3 09 / 10 / 12 / 13 `pbp_eng` and `ann_eng` from the same archives).

Alternative copies inventoried (basename in more than one place): none on PS3 09 / 10 / 12 / 13 among audio members (each bank exists in
exactly one archive or loose, never both — the PS3 12 / 13 `ann_eng` bank + stream live in `nocache.big`, `pbp_eng` `.sbr` in
`audioboot.big` with a loose `.sbs`); on X360 NHL 09, 14 context / definition files exist loose and as (differently sized, wrapped)
archive members — listed in the JSON, none chosen as the runtime source. PS3 NHL 10 additionally ships `ann_fin` / `ann_swe`
announcer banks (`audio.big`) not surveyed by any campaign.

## 4. Horn / chant ids on the archive-only PS3 titles (extends R78 / R81 to PS3 09 / 10 / 12 / 13)

| Title (anchor `exhibitionarena`) | rows | numbered members (`goal` / `period` / `chant`) | `goalhorn` rows with a member | residual `goalhorn` ids (no member in any inspected archive) | `periodhorn` residuals | `chantaudioid` residuals | `horns.abk` | `aemshorn.csi` |
|---|---|---|---|---|---|---|---|---|
| PS3 NHL 09 (`data0.big`) | 188 | 0–29 + 100 / same / 73 ids | 90 | 35, 40, 45, 61, 136, 146 | 32, 40, 45, 61, 136, 146 | 20 values (89–117, 136, 146) | `audioboot.big:audio/sfx/ram/horns.abk`, count 0 | `audioboot.big:audio/sfx/events/aemshorn.csi`, `h_goalHorn` + 8 sustain / gap pairs |
| PS3 NHL 10 (`audio.big`) | 182 | same sets | 108 | 30–39, 41, 45, 61, 136 (14) | same | 25 values | `audioboot2.big`, count 0 | `audioboot.big`, present |
| PS3 NHL 12 (`nocache.big`) | 243 | same sets | 171 | 30–38, 41, 45, 61, 136 (13) | 30–34, 36–38, 41, 45, 61, 136 (12) | 24 values | `audioboot2.big`, count 0 | present |
| PS3 NHL 13 (`nocache.big`) | 174 | same sets | 141 | same 13 | same 12 | 24 values | `audioboot2.big`, count 0 | present |
| X360 NHL 09 (`data0.big`) | 188 | same sets | 90 | as PS3 09 | as PS3 09 | as PS3 09 | count 0 | present |

The PS3 12 / 13 residual sets equal the X360 12 / 13 sets of R78 / R81 (same anchors' values, same member ids); the NHL 09 sets are
new (146 appears only on 09). Members without a row: 0 and 100 (goal / period) on every title; the four-digit chant stems are not
counted here (3-digit only, as R78). Absence is "absent in the inspected archives", not global; the two R81 candidates (segment-built
horn, unreferenced bank 0) are unchanged. (Handoff 03 of this campaign adds the loader's own asset table to the inspected sources.)

## 5. `.ast` stream files (observation only; not a join)

Every bank with stream-type records has a sibling `<bank>.ast` on the loose trees (50 / 50): header `'S10S' | u32 (bit 30 set on all;
low bits vary) | 0x38 zero bytes`, then from 0x40 a chain of blocks `u8 flags | u24 size | u32 word | …` that walks to the file end on
50 / 50 files; flag 0x80 marks 89 blocks on `crowds.ast` against 86 stream records, 262 against 261 on `ambientskatesandpunches`, equal
on 36 / 50 banks. The per-block u32 is not the record's num_samples; the order join does not hold; the record's data offset is not in
the 8-byte record — the undecoded bank tree at 0x78 is the remaining candidate. Not pursued (crowd / player-call streams are Misc /
Players scope).

## 6. Consequence (developer; not Codex-accepted)

- **Parser contract (gen7 `.abk`, replaces R80's entry rule):** read records by §1; refuse a table whose header length ≠ 16 + 4·(count − 1),
  an offset that is neither 0xFFFFFFFF nor inside the file, a segment whose size < 8 or overruns the file, a ram record whose Σ segment
  samples ≠ num_samples, a loop record whose loop_start ≠ 0 / segment-1 samples; treat `stream` records as "audio in `<bank>.ast`,
  offset unresolved" (list, never preview); class codec-`0x05` segments by `channel_config` (0 / 1 decodable through the R77 diagnostic
  route; 3 / 5 listed as "interleaved stereo pair / triple — diagnostic decode not established"). The R80 malformed-input refusals
  (truncation, flag byte, PCM count) hold on every archived class tested (16 / 16 vectors).
- **Coverage (queue row):** the row-12 diagnostic-preview contract extends to `music_samples` on PS3 09 / 10 / 12 / 13 and to the ram
  entries of PS3 09 / 10 / 12 / 13 `.abk` banks, sourced from the archives named in §3 (RefPack on 09 / 10, chunkzip on 12 / 13). X360
  NHL 09 `.abk` and music are EA-XMA (handoff 02 gives that class a diagnostic route). The "not surveyed" cells of R80 §1 are closed;
  PS3 NHL 10 `ann_fin` / `ann_swe` are the only speech banks on gen7 never sampled.
- **Fixture contract (minimal, archive-backed):** PS3 NHL 12 `nocache.big` member `audio/sfx/000_chant.abk` @ 637 110 800, stored
  217 584 B (chunkzip → 217 584 B, SHA-256 `28bedd5591fda8b6…`), 3 records: entry 0 (cc 1, 227 527 samples, 100 434 codec bytes,
  SHA-256 `4173c965e011…`, decodes to 230 400 samples), **entry 1 (loop, loop_start 0, 31 197 bytes, `749f68e68276…`, 70 272 samples)**,
  entry 2 (79 640 bytes, 192 384 samples); the same bank bytes ship on PS3 10 (`audio.big` @ 750 848), 13 (`nocache.big` @ 705 667 952)
  and loose on 11 / 14 / 15 / Legacy. A two-segment fixture: loose PS3 NHL 15 `audio/sfx/055_chant.abk` entry 0 (§1). A 4-channel
  fixture: loose PS3 NHL 15 `002_chant.abk` entry 0 (cc 3, 367 533 samples, 1 284 granules, PCM at frames 2 / 3). Music: PS3 NHL 12
  `.sbs` offset 16 (SNS SHA-256 `24b8e2c4e695c28e…`, 785 408 samples, decodes to 787 968), identical bytes on NHL 13.

## 7. Reproduce

```
python local/runs/overnight-research-2026-09-24/probes/r82_abk_entry_records.py docs/research/overnight-2026-09-24/data/r82a-abk-entry-records.json
set R82_SCRATCH=<scratch dir>
python local/runs/overnight-research-2026-09-24/probes/r82_gen7_archive_audio_coverage.py docs/research/overnight-2026-09-24/data/r82b-gen7-archive-audio-coverage.json
```
Executed: r82a four times (first run crashed on an absent-slot offset; second exposed the padding and stream-record misreadings;
third exposed the too-narrow trim window; fourth is the published JSON); r82b one smoke run (PS3 NHL 12) + one full run.
Inputs: the nine trees, `anchors/ps3/nhl{09,10,12,13}` and `anchors/xbox360/nhl09` `nhlng.db` + meta, ffmpeg 8.1.1 (`mp3float`) as the
decoder oracle (decoder acceptance only).

> **Note 13:47 MDT (R89, [handoff 07](07-abk-bank-tree-read-selection-stream-address.md)):** the "undecoded bank tree at 0x78" of §5 / §6 is
> decoded as a read / selection / stream-address contract on the 11 loose trees (pointer-fixup table, name directory, sample lists
> referencing every S10A record once, `.ast` stream offsets, loop stub + body); the 89-vs-86 `crowds` count is explained (3 loop
> bodies). Stream payload decode and the cc 3 / 5 de-interleave stay open.

# Receipt 36 — more audio previews: PS3 NHL 09 / 10 / 12 / 13 menu music, and Xbox 360 NHL 09 sound banks

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged; nothing staged or committed). Claim `dev-audio-admissions-r82-r131`.
Direction: [overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md) §2
(NHL 09 variants where the contracts suffice). Research consumed: R82 (2026-09-24 handoff 01 §3; named ready by
2026-09-26 handoff 19 A1), R98 / R98b / R100 / **R131** (2026-09-26 handoff 24). Helper: receipt 32. **Not in either
review bundle.**

## 1. Plain-language summary

- **PS3 NHL 09, 10, 12 and 13 menu music** now lists and previews like the other PS3 music banks. Their banks
  live only inside `audioboot.big` (compressed two different ways) and their streams in `data0.big`, loose or
  `nocache.big`; Studio reads them where they are. All 12 research samples match the research on source bytes,
  granule count and frame count.
- **Xbox 360 NHL 09 sound banks** (crowds, chants, horns, player calls, colour commentary…) are now listed,
  framed and previewed through the XMA helper. NHL 09 uses an older packet header that is not a frame count,
  so Studio does not read it as one (as research found, reading it would wrongly reject 47 good sounds); the
  only check is "decoded length = declared length". The two sounds research found short are refused by name.
- Each NHL 09 `.ast` stream ships twice (loose and inside `data0.big`), byte for byte the same. Studio now
  accepts identical copies of a stream as one (it already did for banks); copies that differ still refuse.
- Still not previewed: two-segment in-bank records (their join is not measured), looping stream entries,
  more than two channels.

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-snd/src/ealayer3.rs` | four R82 `music_samples` tuples (PS3 NHL 09 / 10 / 12 / 13) |
| `crates/tdb-snd/src/eaxma.rs` | `HeaderRule` (`Checked` / `Nhl09Variant`); `block_packets` and `raw_segment_packets_with` take it — the variant skips the header check and splits multi-packet sub-blocks at 0x800; unit test |
| `crates/studio-domain/src/gen7_sfx_banks.rs` | Xbox 360 NHL 09 layout (loose `audio/sfx/ram` + `streams`, archive copies); `header_rule`; NHL 09 in `XMA_STREAM_TITLES`; looping in-bank records admitted on NHL 09 only (R131: 176 / 177 exact, played once); notes cite R131; `stream_for` accepts byte-identical copies |
| tests | `studio-domain/tests/gen7_music_r82.rs` (**new**), `x360_nhl09_xma_banks.rs` (**new**); `gen7_sfx_banks.rs` coverage assertion updated (NHL 09 Xbox is measured now) |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `r36-unit.log`: `tdb-snd` lib 22 / 22 (incl. the header-variant case: a two-packet sub-block with a zero header
  is refused as oversize under `Checked` and split into two padded packets under the variant; the length word is
  still checked under both).
- `r36-audio.log`: `gen7_music_r82` 2 / 2 — per title the bank resolves from `audioboot.big` with R82's decoded
  SHA-256 and entry count (166 / 177 / 183 / 195); every one of R82's 12 vectors previews with R82's SNS SHA-256,
  granule count and rebuilt-frame count and decodes to granules × 576 samples per channel (e.g. NHL 10 sample 93:
  33 324 granules → 19 194 624 samples). R82's PCM hashes are FFmpeg's; Studio decodes with Symphonia, so PCM is
  not compared.
- `r36-nhl09.log`: `x360_nhl09_xma_banks` 2 / 2 (729 s) and `gen7_sfx_banks` 7 / 7 (the coverage assertion updated) — see §6.

## 4. Limits

- Decoder agreement is not listening. The NHL 09 count rule admits what research decoded exactly; which sample
  the game plays and how it handles the header variant are not observed.
- R131's oracle was FFmpeg 8.1.1 and it recorded counts, not PCM hashes, for NHL 09; Studio's helper (8.1.3)
  reproduces every count (below). No PCM comparison exists for NHL 09.
- A differing second copy of a stream still refuses; only identical copies are accepted.

## 5. Cards

- **A-XMA-4** (listening, a build that includes receipt 36): X360 NHL 09 → Sound banks → `playercalls.abk` record
  254 (R100 vector; 49 382 samples) and `000_goal.abk` record 0 → Decode: a player call and a goal horn, clean.
- **A-EAL3-2** (listening): PS3 NHL 10 → Speech catalogue → sample banks → music → sample 1 → Decode: stereo music.

## 6. NHL 09 sweep result (Studio's helper `dc3533ed…`, every record of all 186 banks)

| Class | Studio | R131 |
|---|---|---|
| non-loop mono / stereo `.ast` stream entries decoded exactly | **471** | 471 / 471 |
| single-segment in-bank records decoded exactly (loops played once) | **1 901** | — (R131 counts segments) |
| two-segment in-bank records — listed only (join not measured) | 41 (= 82 segments) | decoded per segment |
| in-bank segments, reconciled | 1 901 + 82 + 2 = **1 985** | 1 983 exact + 2 short = 1 985 |
| refused by the count rule | **2**: `0293_chant.abk` record 0 (273 945 vs 288 281), `crowds.abk` record 4 (676 416 vs 705 600) | the same two, same numbers |
| looping stream entries — listed only | 2 | 2 (listed only) |
| more than two channels — listed only | 9 | 9 |

No framing refusal remains (the loose / `data0.big` stream copies are byte-identical and accepted as one).

## 7. Cleanup

No temporary files; scratch scripts deleted.

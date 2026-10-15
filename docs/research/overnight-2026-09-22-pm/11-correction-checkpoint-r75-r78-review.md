# Handoff R79 — Correction checkpoint for the coordinator review of R75–R78 (2026-09-24): R77's applicability is narrowed to the six measured play-by-play sets, its PCM-frame position is reconciled to the measured second EA-frame of block 0 with the public reference pinned by revision and hash, and R78's city / arena numbers are restated as "no established binding from these tests" with the numerical coincidences reported; a developer-facing R76 correction claim (`chunkzip`, not `chunklzx`) is published

Research lane, 24 September 2026 (first package of the continuation; applies
[research-r75-r78-review-20260923.md](../../validation/research-r75-r78-review-20260923.md) items 1–3). Claim
`research-r79-corrections-r75-r78`. **No raw evidence is changed; no probe was rerun.** Dated notes are placed on handoffs 09 / 10 and
the queue; the originals stay in place.

## Plain-language summary

Three narrow corrections and one developer-facing claim, published before the survey they gate. The gen7 speech grammar handoff had
extended its "who can use this" line beyond what was measured; it now names exactly the six play-by-play banks it was tested on and
nothing else until the survey of this continuation adds coverage. Its summary said "the very first record" carries the raw samples while
its table said the second frame of the first block — the table is right, and the public source the reading came from is now pinned to an
exact revision and file hash, with the one place where the NHL data differs from that source stated as a measured difference on these
banks, not as a fault in the upstream. The arena-audio handoff said the city and arena numbers "match nothing"; its own table shows
dozens of numerical hits per family, which are now reported as coincidences with no established binding. And the developer receives a
claim of record that the PS3 NHL 12 / 13 kit members are `chunkzip`-wrapped, so the `chunklzx`-encoder blocker does not describe them —
which removes that one stated dependency and nothing else on the way to any replacement.

## 1. R77 (handoff 09, queue row 9) — applicability, PCM position, units, reference provenance

- **Applicability narrowed to the measured tuples.** Established (48 / 48 vectors, `pbp_eng_samples` only): X360 NHL 15 and NHL Legacy
  (44 100 Hz, MPEG-1, mono); PS3 NHL 15 and NHL Legacy (44 100 Hz, MPEG-1, mono); PS3 NHL 11 and NHL 13 (24 000 Hz, MPEG-2, mono).
  **Withdrawn from §4 and queue row 9:** "PS3 09–13" as a class, announcer banks (§3 already called them untested), music and `.abk` sfx
  members, stereo. Parse-field gates (header byte, flag, tiling) are not a substitute for title / bank coverage; each further tuple is
  released only by the survey (R80).
- **PCM frame position reconciled.** The measured position is the **second EA-frame of block 0** on 48 / 48 vectors (the summary's "very
  first record" is withdrawn); its fields `u16 offset_samples` (0), `u16 pcm_samples` (47), `u32` (0) and the 47 samples sit **after**
  that frame's byte-aligned main data.
- **Units.** Decoded sample counts are stated as `granules × 576 samples` (MPEG-1: two granules = one 1 152-sample frame; MPEG-2: one
  granule per 576-sample frame); the first block carries `(declared − 47) / 576 + 2` granules; Σ `granules × 576 + 47 − num_samples` =
  1 152 + the last-block trim.
- **Reference pinned.** vgmstream, `src/coding/mpeg_custom_utils_ealayer3.c`, latest revision touching the file
  `4b6a02dd1aff6428255db912563d77d4cb0a143e` (2026-07-21), permalink
  `https://raw.githubusercontent.com/vgmstream/vgmstream/4b6a02dd1aff6428255db912563d77d4cb0a143e/src/coding/mpeg_custom_utils_ealayer3.c`,
  fetched 2026-09-24, **SHA-256 `0bc1e0e95dc007d2b0c2c63e4b01667072e8082f7b401c7080e84164ec039d8a`** (842 lines; read, not vendored).
- **Researcher's reading vs the upstream.** In that file the v1 PCM fields are read immediately after the flag / common header
  (`v1_offset_samples`, `v1_pcm_samples`, v1b `v1_pcm_unknown`; lines 331–345) and the granule side info afterwards, with the PCM
  *data* placed after the granule's common part (`eaframe_size = pre_size + common_size + pcm_size`). On the NHL banks the **fields** are
  after the granule's padded main data. R77's first attempt applied the upstream order and failed on the second frame; the measured order
  tiles every block. This is a **measured difference on the NHL banks** (a variant the upstream may or may not model — its own targets
  were not examined), not a claim that the upstream decoder is wrong.
- Decoder acceptance and matching sample counts remain **evidence of a well-formed rebuild**, not audible or game-equivalent validation.

## 2. R78 (handoff 10, queue row 10) — city / arena ids

- **Withdrawn:** "`cityaudioid` / `arenaaudioid` are not sfx-bank ids", "match nothing". **Restated:** on the three loose families searched
  (`<id>_goal.abk`, `<id>_period.abk`, `<id>_chant.abk` 3- and 4-digit), the columns' values coincide numerically with bank ids on many
  rows — NHL 15 X360: `cityaudioid` 61 rows with a chant id / 40 with a goal id, `arenaaudioid` 57 / 36 (of 194 non-zero rows; 155 and
  157 distinct values, ranges 1–511 and 1–701) — with 104–127 distinct values outside every family. **No established binding** follows
  from these tests: the hits are consistent with overlapping small-integer id spaces and are reported as coincidences; the searched
  scope was the loose `audio/sfx` numbered banks only. The 2026-09-21 handoff 07 reading (speech sample-group ids; tag open, Q-R35-1)
  stands as the candidate; absence of a direct loose-file match does not identify the alternate consumer.
- **Preserved:** the exact-match horn listing (`goalhorn` / `periodhorn` → `<id:03>_goal/period.abk`, per-title residuals) and the chant
  candidate. Presence of a file is separate from what the game selects or plays (Q-R78-1 for the non-NHL ids; no source is invented).

## 3. R76 — developer-facing correction claim (`research-r76-chunkzip-correction-for-developer`)

PS3 NHL 12 / 13 kit members (`rendering/{jersey,pant,sock}/*.rpsgl` in `nocacherender.big` / `cacherender.big`) are **`chunkzip`-wrapped
on 100 % of 3 012 + 2 010 and 1 782 + 1 782 members** (R76 §1); receipt 04's "`chunklzx` payloads" wording and its §5 "no `chunklzx`
encoder" blocker do not describe these members. That removes **that specific** stated dependency and nothing else: the complete
copied-target replacement path (codec round-trip, placement / slot rules, archive transaction, loader acceptance) is the developer's to
assess; research does not authorise a live write and does not qualify an encoder.

## Changed paths

Dated notes on handoffs 09 and 10; queue rows 9 / 10 (rev. 1a); claims `research-r77` / `research-r78` (`result` note); new claims
`research-r79-corrections-r75-r78` and `research-r76-chunkzip-correction-for-developer`. Unchanged: every `data/*.json`, every probe.

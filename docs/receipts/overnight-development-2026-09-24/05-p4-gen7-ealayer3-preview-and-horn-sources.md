# Receipt 05 (P4.3 + P4.4) — gen7 EALayer3 v1 diagnostic preview and gen7 horn sources

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** Branch `feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Nothing staged,
committed, pushed or launched; the game trees on `X:` were read only. Claim:
`dev-p4-gen7-ealayer3-and-horn-sources`. Research consumed: R77 (2026-09-22 pm handoff 09) as
narrowed by R79 (handoff 11), R80 (handoff 12), R78 (handoff 10) as corrected by R79, R81
(handoff 13); the `r77_gen7_ealayer3_v1_grammar.py` probe as the reference algorithm (no code
copied). Clock: 13:35 → 13:58 (MDT).

## 1. Plain-language summary

The Xbox 360 / PS3 titles store commentary in EA's own "EALayer3 v1" form: MP3 granules with a
compact side-info header, packed into EA frames, with two warm-up granules and a 47-sample raw
frame at the start. Research established that grammar on 48 play-by-play vectors and then surveyed
which title / bank / rate / channel combinations carry it (adding announcer, more 24 kHz sets and
stereo music). Studio now parses those granules, refuses every malformed class by name, rebuilds
them into standard MPEG frames through the bit reservoir exactly as the research did, and decodes
them with the same pure-Rust decoder as the PS4 route — **only on the measured tuples**, and always
under the label *"diagnostic preview — start handling unresolved (two warm-up granules + 47 PCM
samples; not game-verified)"*. All 116 research vectors (both rate classes, mono and stereo) pass
with the research's granule and frame counts. On the team page a name-call sample id can now be
selected and decoded this way (play-by-play bank, loose trees).

Separately, the arena page's gen7 horn columns now read against the install's numbered `audio/sfx`
banks the way research R78 / R81 established: a goal or period horn id resolves to its bank **file**
by exact presence (or to an archive member by name where the title keeps them there), a chant id is
shown as a **candidate** only, city / arena audio ids state that no binding is established, and an
id without a bank says exactly which sources were inspected — never "no horn".

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-snd/src/ealayer3.rs` (**new**) | MSB-first bit reader / writer; `parse_block` (R77 §1 grammar, mono + stereo, MPEG-1 / MPEG-2); `contract_parse` (tiling, header class = version + rate + channel mode, granule counts per block incl. the +2 warm-up rule, exactly one 47-sample PCM frame as block 0 frame 1, MPEG-1 alternation); `rebuild` (R77 §2: two / one granules per frame, bitrate index 14, reservoir placement ≤ 511 / 255, scfsi carried, odd count padded); `MEASURED` tuple table (R77 six pbp sets + R80 ann / pbp / music sets); `LABEL`; 2 unit tests (both rate classes, 8 refusal classes) |
| `crates/tdb-snd/src/eamp3.rs`, `decode.rs` | `MpegHeader::parse_any` (MPEG-2 headers, 72-slot rule); the decoder takes MPEG-1 or MPEG-2 streams (576 samples per MPEG-2 frame), `Decoded.samples_per_frame` |
| `crates/tdb-snd/tests/ealayer3_real_vectors.rs` (**new**, `--ignored`) | every R77 / R80 block-stream vector on the `X:` trees (loose `.sbs` or the recorded archive member offset) |
| `crates/studio-domain/src/gen7_speech_preview.rs` (**new**) | `coverage` (measured tuples; `pbp_eng` / `ann_eng` only), `preview_sample` (SBKR bank → `.sbs` offset → SNS walk → codec gate → grammar → rebuild → decode → WAV of the whole decoded stream, no start / end trim applied — Q-R77-1); `tests/gen7_speech_preview.rs` (`--ignored` real X360 NHL 15 pbp + PS3 NHL 15 ann) |
| `crates/studio-domain/src/gen7_horn_sources.rs` (**new**) | `Gen7HornSources::probe` (loose `audio/sfx` numbered banks; `nocache.big` EB members; `data0.big` BIG4 members via the `.bh` sidecar; `horns.abk` length; `aemshorn.csi` presence; refusals for PS4 / PS3 12–13 / NHL 07–08) and `resolve` per column class; `tests/gen7_horn_sources.rs` (`--ignored` real X360 NHL 15 + NHL 09) |
| `crates/studio-domain/src/arena_venue.rs`, `view.rs`, `session.rs` | `build` takes the horn sources; the venue view resolves `goalhorn` / `periodhorn` / `chantaudioid` / `cityaudioid` / `arenaaudioid` through them on non-PS4 titles (PS4 keeps its `goalhorns.csv` join); the session probes the attached install |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `Gen7SpeechPreviewDto`, `gen7_speech_preview` command; export roots 155 → 156 |
| `app/src/bindings/Gen7SpeechPreviewDto.ts` (generated), `app/src/lib/client.ts` | shape |
| `app/src/components/Gen7SpeechPreview.tsx` (**new**) + `.test.tsx`, `TeamNameCallsCard.tsx` | sample id chips select; explicit **Decode diagnostic preview**; label / diagnostics / identities; one owned `<audio>` released on any change; late replies dropped; the name-calls card mounts it for `pbp_eng` |

## 3. Evidence

- **Research vectors** ([`p4-ealayer3-real-vectors.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p4-ealayer3-real-vectors.log)):
  **116 / 116** — 48 R77 (X360 15 / Legacy, PS3 15 / Legacy at 44.1 kHz; PS3 11 / 13 at 24 kHz) + 68
  R80 (PS3 `ann_eng` 09–Legacy, PS3 `pbp_eng` 09 / 10 / 12, `music_samples` stereo on X360 13–Legacy
  and PS3 11 / 14 / 15 / Legacy): SNS SHA-256 = research; granule counts, frame counts and
  `granules × 576` decoded samples = research; `main_data_begin` ≤ 511 / 255; RMS within 5 % of
  ffmpeg `mp3float` on every vector. Streams read from the loose `.sbs` or, for X360 15 / Legacy and
  PS3 11, the recorded `nocache.big` / `audiostream.big` member offset. `.abk` raw streams not covered.
- **Domain, real trees** ([`p4-gen7-speech-preview-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p4-gen7-speech-preview-real.log)):
  X360 NHL 15 `pbp_eng` sample 58 — 9 blocks, 144 granules (18/16×7/14), 72 frames, SNR 80 418 →
  82 944 decoded, first > 1 LSB at 1 030; PS3 NHL 15 `ann_eng` sample 45 — 13 blocks, 124 granules,
  124 MPEG-2 frames (mdb ≤ 255), SNR 68 096 → 71 424 decoded. Both labelled diagnostic.
- **Horn sources, real trees** ([`p4-gen7-horn-sources-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p4-gen7-horn-sources-real.log)):
  X360 NHL 15 loose goal / period banks = {0–29, 100, 194} (R78 / R81), `horns.abk` 3 268 B,
  `aemshorn.csi` present, `nocache.big` lists 32 / 32 / 118 numbered members; ids 1–29 resolve to
  files, the R81 residual ids (30–36, 38, 41, 45, 61, 136) produce the inspected-sources text; X360
  NHL 09 has no loose banks and `data0.big` lists goal {0–29, 100} — **the r81 JSON places them in
  `data0.big`, the handoff §1 text says `nocache.big`**; the data was followed (observation for research).

## 4. Checks run (see also [`closeout-aggregate.log`](../../validation/evidence/overnight-development-2026-09-24/logs/closeout-aggregate.log))

| Run | Result |
|---|---|
| `cargo test -p tdb-snd --lib` | 12 passed |
| `cargo test -p tdb-snd --test ealayer3_real_vectors -- --ignored` | **1 passed** (116 vectors, 71 s) |
| `cargo test -p studio-domain --lib -- gen7_horn_sources arena_venue` | 4 passed |
| `cargo test -p studio-domain --test gen7_speech_preview -- --ignored` | 1 passed |
| `cargo test -p studio-domain --test gen7_horn_sources -- --ignored` | 1 passed |
| `cargo test -p studio-api --test arena_venue` | 2 passed, 1 ignored |
| `npx vitest run src/components/Gen7SpeechPreview.test.tsx src/components/TeamNameCallsCard.test.tsx` | 7 passed (3 new) |
| `npx tsc --noEmit`; `export-bindings --check` | clean; in sync (156 roots) |

## 5. Decisions and limitations

- **Coverage is the measured-tuple table**, not the codec byte: a title / bank outside
  `MEASURED` refuses with the tuple named (X360 announcer / pbp 11–14 and PS3 14 pbp are other
  codecs; X360 09 / 10 speech is EA-XMA). Music banks are measured tuples but their bank index is
  not the speech `SBKR` table; they are listed as measured and **not previewed**.
- **No start / end trim**: the WAV is the whole decoded stream (granules × 576). Which samples the
  game discards (two warm-up granules, the 47 PCM samples, the tail) is Q-R77-1; applying a rule
  would be a guess. Listening is the user's (card A-EAL3-1).
- **Streams inside archives** (X360 15 / Legacy `nocache.big`, PS3 09 / 10 / 12 / 13 banks and
  streams in `audioboot.big` / `data0.big` / `audio.big`) are **not** read by the domain route — it
  refuses with the archive named. The research-vector test reads them at the recorded offsets
  only to prove the codec path; extraction through Studio's archive readers is a follow-up.
- Header class excludes `mode_extension` (varies per granule on joint-stereo music; R80 "carried as
  read") — a correction to my first reading, recorded for research.
- Horn banks are **not decoded** (the `.abk` S10A container is not read here); presence and names
  only. `chantaudioid` is a candidate join; `cityaudioid` / `arenaaudioid` have no binding (R79).
- No desktop run: cards V-49 / V-50 in `MORNING-REVIEW.md`.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_p4_gen7_api.py`, `patch_p4_horns.py` | kept until closeout, then deleted |
| Temp roots `nhlms-horns-*` | removed by the unit test |
| Background processes | the closeout aggregate (finishes on its own); none at closeout |

## 7. Next action

Closeout: STATUS, ledger, morning review, coordinator review list, cleanup; then the next ready
item (P4.5 catalogue integration of the previews into commentary / name-call rows across surfaces,
or Phase 3 if all League / Team work is exhausted).

> **Note 2026-09-26 (receipt 24):** superseded in part. The archive-backed PS3 NHL 09 / 10 / 12 / 13 sound banks are now listed, framed and previewed from their own members; a single-sub-stream loop record is gated as "no preview" in the tree as well as the route (NHL 09 had one marked previewable); the horn probe reads PS3 12 / 13 (`nocache.big`) and PS3 10 (`audio.big`) and names members by their real path (NHL 09: `audio/sfx/ram/…`); the chant note follows R97 §1.4 and never resolves a chant id to a bank. See [receipt 24](24-p48-archive-sound-banks-chant-wording-horn-links.md).

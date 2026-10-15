# Handoff R126 (P22) — audio: which speech / music tuples the current source admits for preview, which it only lists or refuses, and the one contract-ready gap (PS3 NHL 09 / 10 / 12 / 13 music)

Research lane, 27 September 2026, published 05:20 MDT (system clock). Prompt
[`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) package 5. Claim `research-p22-audio-admission-residuals`.
Static reading of the working tree (not executed): `crates/tdb-snd/src/ealayer3.rs` (`MEASURED`), `easpeex.rs` (`MEASURED`),
`crates/studio-domain/src/ps4_speech_bank.rs`, `gen7_speech_preview.rs`, `gen7_sfx_banks.rs`, developer receipts 22–31. No decoder run, no
vector added, nothing listened to; the SNS / XMA wiring and helper stay the developer's.

## Plain-language summary

Studio's audio previews only open for combinations of console, game and sound bank that research has measured; everything else is listed
and refuses with a reason. Reading those lists against the research shows one combination that was measured but never added: the music
banks of PS3 NHL 09, 10, 12 and 13, which research established on 24 September from the archives (same codec as the other PS3 music).
Everything else that is missing is either the Xbox XMA family (the developer's next task, and blocked on a helper program for playback)
or has no research measurement yet.

## 1. Preview admission today (source reading)

| Family | Admitted tuples (preview) | Listed / framed but not previewed | Source of the contract |
|---|---|---|---|
| gen7 EALayer3 v1 speech | X360 NHL 15 / Legacy `pbp_eng` 44.1 k; PS3 NHL 15 / Legacy `pbp_eng` 44.1 k; PS3 NHL 09–13 `pbp_eng` 24 k (09 / 10 / 12 R80, 11 / 13 R77); PS3 NHL 09–Legacy `ann_eng` 24 k | — | R77, R80 |
| gen7 EALayer3 v1 music (stereo 44.1 k) | X360 NHL 13 / 14 / 15 / Legacy; PS3 NHL 11 / 14 / 15 / Legacy | **PS3 NHL 09 / 10 / 12 / 13** — not in `MEASURED` (listing refuses the tuple by name) | R80; **R82 (2026-09-24 handoff 01 §3) established the PS3 09 / 10 / 12 / 13 music tuples, 3 vectors each** |
| gen7 EA-Speex speech (32 k) | X360 NHL 11–Legacy `ann_eng`; X360 NHL 11–14 `pbp_eng`; PS3 NHL 14 `pbp_eng` | — | R83 (receipts 29 / 31) |
| PS4 speech | EA-MP3 (`0x0b`) samples in the speech banks of PS4 NHL 15–21, admitted per sample by codec (R71 vectors come from NHL 15–17); EA-Speex rink announcer NHL 15–21 | EA-Opus and other codecs refuse with their label | R71, R83 |
| gen7 `.abk` / `.ast` (PS3) | loose 11 / 14 / 15 / Legacy and archive-backed 09 / 10 / 12 / 13, cc 0 / 1, stereo pairs, loop stub + body | cc 5 de-interleave | R89–R100 (receipts 22, 24, 25) |
| Xbox 360 EA-XMA | ram records of X360 NHL 10 → Legacy wired behind the helper gate | **every XMA tuple framed only** (helper not built); X360 NHL 09 banks refused; SNS speech / music of X360 NHL 09–12 not wired (developer's next task) | R84, R92, R100 |
| Legacy Recompiled | not admitted anywhere (no vector) | — | — |

## 2. Residual gaps

| # | Gap | Class | Next action |
|---|---|---|---|
| A1 | PS3 NHL 09 / 10 / 12 / 13 `music_samples` — established by R82 (`.sbr` in `audioboot.big`, RefPack on 09 / 10, chunkzip on 12 / 13; `.sbs` in `data0.big` (09), loose (10), `nocache.big` (12 / 13); 3 vectors per title with SNS hashes) but absent from `ealayer3::MEASURED` | unimplemented, contract ready | developer: add the four tuples with the R82 vectors as regression, reusing receipt 08's archive resolution |
| A2 | X360 NHL 09–12 SNS speech / music (EA-XMA) | unimplemented (developer's stated next task) + helper blocked | none from research (R84 vectors exist) |
| A3 | X360 NHL 09 archive-backed banks: packet-header variant (R98b frame walk exact on 19 138 / 19 225 middle blocks; residual 87 blocks, 2 header values, 4 loop bodies) | unproven residual; playback helper-gated anyway | no research action until the helper exists |
| A4 | PS3 NHL 10 `ann_fin` / `ann_swe` never sampled | missing measurement | small research package if Finnish / Swedish announcer preview matters for the beta |
| A5 | Every preview's audible correctness | manual acceptance | listening cards (A-SPX-2, A-MP3-1, A-EAL3-1, A-AST-1, A-LOOP-1) |

No replacement contract exists for any audio family; decoder agreement is not listening acceptance.

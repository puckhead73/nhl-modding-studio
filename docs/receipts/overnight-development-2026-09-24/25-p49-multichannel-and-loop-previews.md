# Receipt 25 (P4.9) — four- / six-channel and loop sound-bank entries preview as diagnostic parts

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged). Claim: `dev-p49-multichannel-loop-previews`. Clock:
03:20 → 03:28 (real clock, MDT, 26 Sep 2026). Direction: the refreshed ledger's next row ("cc 3 / 5
de-interleave, loop stub + body preview — ready, bounded"), after receipt 24.

## 1. Plain-language summary

Crowd and ambience sounds on the PS3 titles are often four- or six-channel (two or three stereo
pairs woven together) or looping (a short intro plus a loop body). Until now the sound-bank card
listed them with "no preview". It can now play them as diagnostic previews: a four- or six-channel
entry one stereo pair at a time, chosen from a small selector, and a loop as its intro followed by
one pass of the body, with the loop point shown. The way the pairs are separated is the research's
inference (every Nth audio frame belongs to the same pair), not a rule checked against a reference
player, and the label says so. Xbox 360 stays list-only; nothing writes.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-snd/src/abk.rs` | `split_sub_streams` (EA frame *i* of a block → sub-stream *i* mod N; refuses a non-multiple frame count or frames that do not tile), `stub_pcm` (exactly one PCM frame, sample count = the record's word `a`; planar → interleaved), `eal3_loop_diagnostic` (stub PCM + body R92 construction), `EAL3_PART_LABEL` |
| `crates/studio-domain/src/gen7_sfx_banks.rs` | the gate admits channel configs 3 / 5 and stream loops (unmeasured configs and X360 still refuse); `frame_stream` walks a loop's body at the stub's end on the same identity-checked handle and requires stub + body = the record's count (R91 §1); `preview_part(…, sub_stream)` — a multi-stream entry without a choice, or with an out-of-range one, refuses in words; the preview reports `sub_stream`, `sub_streams`, `loop_start` |
| `crates/studio-api/src/{dto,registry,commands}.rs`; `app/src/bindings/Gen7SfxPreviewDto.ts` (generated); `app/src/lib/client.ts` | optional `subStream` on `gen7_sfx_preview`; preview DTO fields |
| `app/src/components/Gen7SfxBankCard.tsx` + test | "stereo pair k of N" selector on multi-stream entries (tooltip states the inference); preview line shows the pair and "loop point at sample a (played once)"; new jsdom case |
| `crates/studio-domain/tests/gen7_sfx_banks.rs` | NHL 15 crowds record 0 loop per pair; every loop on the four archive titles must frame stub + body; R100's cc 3 vectors, every cc 5 entry and one multi-stream loop per title decoded per pair to the exact declared count |

## 3. Evidence

- [`p49-sfx-parts-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p49-sfx-parts-real.log) —
  `gen7_sfx_banks` 5 / 5 with `--include-ignored`; `tdb-snd` abk 3 / 3.

  | Vector | research | Studio, each pair |
  |---|---|---|
  | PS3 NHL 15 `crowds` 0, cc 3 loop | PREVIEW_REVIEW #4–#9: stub 29 + body 689 959 | loop point 29, 689 988 stereo frames, body at 1 536 (598 blocks) |
  | PS3 NHL 10 `crowds` 144, cc 3 | R100: 1 102 blocks / 1 270 528, 2 × 2 210 granules | 1 270 528 frames, 2 210 granules |
  | PS3 NHL 12 `crowds` 6, cc 3 | R100: 1 270 528, 2 × 2 210 | 1 270 528, 2 210 |
  | PS3 NHL 13 `crowds` 18, cc 3 | R100: 228 096, 2 × 400 | 228 096, 400 |
  | PS3 NHL 13 `sfx` 206, **cc 5** (the one R98 entry) | framed only | 3 pairs × 455 519 frames, 796 granules |
  | loops on 09 / 10 / 12 / 13 (crowds 0 / 121 / 7 / 113) | R98: stub + body = count on 2 / 4 / 9 / 9 | loop points 29 / 32 / 32 / 29; each pair exactly the count |

  Every loop entry of the four archive titles (24) frames stub + body in the tree. Multi-stream
  entries per title: 1 / 53 / 106 / 107.
- [`p49-bindings-api-app.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p49-bindings-api-app.log) —
  `export-bindings --write` (1 written) then `--check` in sync; `studio-api --lib` 31 / 31; vitest
  40 files / 247 tests; `tsc` clean after replacing an ES2022 `.at()` in the new test.

## 4. Limits

- The de-interleave is the research's inference (frame order + granule-index alternation); R92's
  single 4-channel RIFF was refused by the reference decoder, so no reference split exists. Pairs are
  offered separately, never mixed.
- A loop plays once (stub, then one body pass); the loop point is shown, not looped. Whether the
  console repeats exactly there is runtime (card A-AST-1).
- Long loops make large previews (PS3 NHL 09 `crowds` 0: 4 974 069 stereo frames ≈ 20 MB WAV), the
  same order as the music preview; decoded on request only.
- X360 entries (EA-XMA) and PS4 are unchanged: no decoder decision.

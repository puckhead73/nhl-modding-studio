# Handoff R91–R95 — gen7 `.ast` block payload framing established on every stream of the 11 loose trees (29 962 referenced streams + 146 loop bodies, 137 files, 0 refusals, every byte accounted); the located streams decode diagnostically with exact counts and bit-identical restart-from-block continuity; the NHL 21 single-block Speex "decoder disagreement" is silence; the bank tree's "name hash" is two 16-bit keys; the S / X words stay uninterpreted; club-chant selection is narrowed to a loader-by-number gate with no offline binding; replacement stays unqualified

Research lane, 25 September 2026, 05:36 → (see STATUS) MDT. Claims `research-r91-ast-framing`, `research-r92-ast-decode-previews`,
`research-r93-speex-outlier`, `research-r94-abk-sx-hash`, `research-r95-chant-selection` (all `ready`). Read-only over the X360
NHL 10–Legacy and PS3 NHL 11 / 14 / 15 / Legacy loose trees and the PS4 NHL 21 extracted install (CUSA18182); generated audio kept
local. Nothing game-verified; nothing listened to; no writer; no replacement. Documents: [AST_FRAMING.md](AST_FRAMING.md),
[COVERAGE.md](COVERAGE.md), [PREVIEW_REVIEW.md](PREVIEW_REVIEW.md), [ABK_TREE_CONTRACT.md](ABK_TREE_CONTRACT.md),
[CHANT_SELECTION.md](CHANT_SELECTION.md), [REPLACEMENT_READINESS.md](REPLACEMENT_READINESS.md), [MANIFEST.md](MANIFEST.md),
[STATUS.md](STATUS.md), [CHECKPOINT.md](CHECKPOINT.md), [USER-CARDS.md](USER-CARDS.md), [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md).

## Plain-language summary

Yesterday's package could find every sound's bytes inside the big stream files but could not read them. Tonight the layout of those
bytes is measured on every one of the thirty thousand streams the games ship on the older consoles: each stream is a chain of blocks,
each block a self-contained piece of the compressed audio (EA's MP3-like frames on PlayStation 3, Xbox's XMA packets on Xbox 360),
with padding only at the very end, and the sample counts written on each block are exact. Decoding the first pieces through an
ordinary decoder returns exactly the expected number of samples, and decoding again from the middle of a stream lands on the very same
samples the block map predicts — which is how the map was checked independently of the parser. One wrinkle became clear: the first
block carries 47 raw samples that turn out to be a copy of what the decoder produces just before its warm-up ends, which pins where
the sound is meant to start. The odd PlayStation 4 announcer sample that two decoders "disagreed" on is silence — the disagreement
was rounding noise. The bank's mystery "hash" is really two 16-bit keys, one per name and one per bank; two other words are still
unexplained but do not affect reading. For the Swedish club chants, the shipped files show the banks are loaded by number, and no
database value except one Finnish arena's ever names a club bank; what would pick a club at runtime could not be found in the files.
None of this makes replacing audio possible: there is still no encoder and one unexplained bookkeeping word.

## 1. Newly demonstrated (each with its evidence file)

| Capability | Result | Evidence |
|---|---|---|
| **Parsing / framing** (Q-R89-1) | `.ast` grammar per AST_FRAMING.md §1: block header `u8 flags (0x00 / 0x80) | u24 size | u32 samples`; PS3 payload = EA-frame chain, X360 payload = `w0 | XMA2 packet` sub-blocks (1 per XMA stream); zero padding only in the terminal block (PS3: to the next 0x40 boundary; X360: terminal size % 0x40 == 0 + zero gaps); sample words exact on middle blocks, first-block delta 1152 / sub-stream (EAL3) / 384 (XMA), terminal trim 1153–2304 / 0–884; loops = one-block stub (a, b) + unreferenced fresh-start body, stub + body == num_samples; MPEG-1 granule index alternates across block boundaries per de-interleaved sub-stream. **29 962 / 29 962 referenced streams + 146 / 146 bodies, 0 refusals, 137 / 137 files byte-accounted.** Controls: 15 named mutations per platform; 14 / 13 detected; a byte flipped deep in main data / a packet body is not structurally detectable (stated). | `data/r91-ast-framing.json`, `blockmaps/r91-streams.jsonl`, `blockmaps/representative/*.json` (288 full block maps), `r91_ast_framing.py` |
| **Decoding (diagnostic)** | EAL3 → granules × 576 on every sub-stream; XMA → exactly the declared count, header-independent; restart from the middle block bit-identical at the predicted offset (EAL3 after 2 granules, XMA at once); the block-0 PCM frame == decoder output 1105–1151 (corr ≥ 0.995 mono; **planar** per channel on stereo, corr 1.00 / 0.93); loops decoded as stub + body; 4-channel as two stereo sub-streams (inferred de-interleave; the single 4-channel RIFF was refused by the decoder — recorded). 14 vectors, PS3 11 / 14 / 15 / Legacy, X360 10 / 12 / 15 / Legacy. | `data/r92-ast-decode.json`, `r92_ast_decode.py`, 47 WAV under `local/runs/overnight-research-2026-09-25/previews/`, PREVIEW_REVIEW.md |
| **Speex outlier** (R88 §F4) | Explained, not a decoder disagreement: the PS4 NHL 21 base `rink_ann_samples` single-block record is silence (peaks 5 / 4 LSB, rms 0.40 / 0.90, max diff 2 LSB, 10 / 12 frames Speex NB mode 1); corr 0.46 is rounding noise; the same title's `_inxp` control: corr 0.98. It validates neither decoder and cannot test the 509 rule. | `data/r93-speex-outlier.json`, `r93_speex_outlier.py`, 4 WAV `OUTLIER-*` |
| **Bank-tree contract extension** (Q-R89-2) | The u32 before each name = `u16 bank key | u16 name key`: name key constant per name across banks and platforms (0 exceptions / 1 217 names), bank key constant per bank and shared by families (chant 0x2266, goal 0x7044, period 0x1be8); 29 hash algorithms × 8 variants × 4 folds fail on the name key (≤ 1 / 244 — chance); X constant per list, clusters by channel config; S platform-independent, 4-byte multiples, cumulative in a non-list non-index order, increments 2·ch·n + 0.3–1.4 %, no formula of 14 fits. | ABK_TREE_CONTRACT.md, `data/r94-abk-sx-hash.json`, `r94_abk_sx_hash.py` |
| **Selection binding** (R90 / T-CHANT-1) | Loader: every team-chant bank is `AemsCrowdTeamChantBank … descr="<number>"` like the goal horns — loading is by bank number; the only database value in 55–66 is the Finnish arena's `chantaudioid` 61; `aemscrwd.csi` = 196-name MOIR table **plus a 12 KB undecoded body** (u16 words hold 8 name ids, none a club picker); `controlCenter.abk` holds the 12 club picker names as empty class-2 `_msg` endpoints (no parent, no constants). **No binding**; R90's "selected by the compiled module" narrowed to "named by the module and the control bank; loading gated by number"; smallest observation = a game at the Finnish arena (T-CHANT-1 rev. 2). | CHANT_SELECTION.md, `data/r95-crowd-script.json`, `data/r95b-controlcenter.json` |
| **Replacement** | **Unqualified**, both classes: (a) same-length substitution needs an EAL3 v1 / XMA2 encoder reproducing block geometry, and the unexplained S mirror; (b) blocked on the encoder and S. This package supplies the framing an encoder would have to match and shows a same-length swap moves no pointer. | REPLACEMENT_READINESS.md |

> **Note 2026-09-25 14:20 (R97, [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md)):** §1 "Speex outlier — silence" reads **"near-silent output under both tested decoders;
> unsuitable for validating onset or decoder fidelity"**; "14 / 13 detected" reads **14 / 14 detected + 1 structurally undetectable per
> platform**; "Selection binding — the only database value in 55–66 …" is withdrawn as stated (handoff 10 §1: that arena's horn ids are 61
> too; team / player audio ids also fall in 55–66; no consumer traced); the PCM-frame match is established on loud frames only (R96,
> handoff 10 §6); "every stream of the 11 loose trees" = PS3 11 / 14 / 15 / Legacy + X360 10–Legacy, archive-backed titles by R98 (handoff 11).

## 2. Counterexamples and limitations carried

- **Undetectable corruption:** a flipped byte inside EA-frame main data or an XMA packet body passes the framing parser; only a decoder-side
  check could see it (and a decoder "returning audio" is not that check either).
- **cc 3 / 5 de-interleave** is inferred (frame order + granule-index alternation per sub-stream); the 4-channel XMA RIFF path was
  refused by ffmpeg and the sub-stream path is an inference of the same kind.
- **PCM-frame start** (output = PCM frame + decode from 1152) is a data-supported candidate; the game's actual reconstruction is unobserved.
- **X360 single-block streams** show first-block deltas 385–884 (= 384 + trim): the rule "first block delta 384" holds only when the
  first block is not the terminal block (95 streams).
- **S word:** the successor chain is unique for 98 % of `playercalls` entries but only 16 / 86 of `crowds` (long streams, ambiguous
  windows); the overhead has no fitted formula; the accumulation order is unexplained.
- **Chant selection:** the PS3 `aemscrwd.csi` table layout differs (not parsed); the MOIR body opcode format is unknown; the executable
  was not inspected (encrypted / no tool authorised) — the key, if any, is out of offline reach.
- **Archived trees** (PS3 09 / 10 / 12 / 13, X360 09): `.ast` members not extracted — framing expected, unmeasured.
- **Nothing listened to; nothing game-verified.**

## 3. Corrections to earlier statements (dated notes placed on the 2026-09-24 package)

- Handoff 07 §4 "concatenating a PS3 stream's block payloads does not tile under R77" → the terminal block's zero padding was parsed as
  a frame; per-block chains tile on 100 % of streams (R91).
- Handoff 06 §F4 / queue row 3 outlier → silence, resolved (R93); the class label "neither decoder validated" stays true but for a
  different reason (nothing to validate on a silent record).
- Handoff 07 §4 name hash "32-bit, not CRC-32 / FNV …" → two u16 keys (R94); the search must target the low half.
- Handoff 08 / card T-CHANT-1 "selected by the compiled crowd module by name" → narrowed (R95); card revised (rev. 2).

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-25/probes/r91_ast_framing.py docs/research/overnight-2026-09-25/data/r91-ast-framing.json --summary local/runs/overnight-research-2026-09-25/blockmaps/r91-streams.jsonl
python local/runs/overnight-research-2026-09-25/probes/r92_ast_decode.py docs/research/overnight-2026-09-25/data/r92-ast-decode.json --previews local/runs/overnight-research-2026-09-25/previews
python local/runs/overnight-research-2026-09-25/probes/r93_speex_outlier.py docs/research/overnight-2026-09-25/data/r93-speex-outlier.json --previews local/runs/overnight-research-2026-09-25/previews
python local/runs/overnight-research-2026-09-25/probes/r94_abk_sx_hash.py docs/research/overnight-2026-09-25/data/r94-abk-sx-hash.json
python local/runs/overnight-research-2026-09-25/probes/r95_crowd_script.py docs/research/overnight-2026-09-25/data/r95-crowd-script.json
python local/runs/overnight-research-2026-09-25/probes/r95b_controlcenter.py docs/research/overnight-2026-09-25/data/r95b-controlcenter.json
python local/runs/overnight-research-2026-09-25/probes/gen_docs.py
```
Executed: r91 smoke × 3 (two grammar corrections), full × 1, representative maps × 2; r92 × 4 (offset fix, PCM term, planar reading,
preview construction); r93 × 1; r94 × 1 (+ two scratch explorations); r95 × 1, r95b × 2 (MOIR entry layout fix); gen_docs × 1.

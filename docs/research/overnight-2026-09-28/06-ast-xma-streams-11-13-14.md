# Handoff 06 (28 Sep) — X360 NHL 11 / 13 / 14 `.ast` EA-XMA streams: every stream record decodes to its declared count through the pinned helper

Research lane, 28 September 2026, published 05:44 MDT (system clock). Prompt package 6; this is the residual audio row from
[00](00-correction-checkpoint.md) §4. Claim `research-q6a-ast-xma-11-13-14`. Read-only on the loose X360 `audio/sfx/<bank>.abk` + `.ast`
pairs.

**Method.** The framing is unchanged R91: stub, then the loop body at the stub end or the next 0x40 boundary. The packet assembly is
unchanged R92, per sub-stream when a block holds several XMA sub-blocks. Decoder: the installed pinned helper (SHA-256 `dc3533ed…`,
stdin / stdout). Decoder output was never kept, and nothing was listened to.

Machine-readable: [`data/r140-ast-xma-streams-11-13-14.json`](data/r140-ast-xma-streams-11-13-14.json). Probe:
`local/runs/overnight-research-2026-09-28/probes/r140_ast_xma_streams_11_13_14.py` (`07177edb…`).

## Plain-language summary

Studio previews Xbox stream sounds only on the titles where research had a sample: NHL 09, 10, 12, 15 and Legacy. Research has now
decoded every stream sound in every bank of NHL 11, 13 and 14 with Studio's own decoder. Each comes out at exactly the length the bank
declares, including looping sounds and four- and six-channel ones. NHL 12, already admitted, was rerun as a control and behaves the same.
This is decoder evidence, not a listening check.

## 1. Results (every EA-XMA stream record; "exact" = every sub-stream of every part = the part's summed block samples, and for non-loops = the record's `num_samples`)

| Title | Mono, no loop | Stereo, no loop | Stereo loop (stub + body) | 4 ch, no loop | 4 ch loop | 6 ch, no loop | Framing refused / not exact |
|---|---|---|---|---|---|---|---|
| NHL 11 | 676 | 39 | 8 | 4 | 3 | — | 0 / 0 |
| NHL 13 | 3 185 | 73 | 14 | 7 | 5 | 1 | 0 / 0 |
| NHL 14 | 3 177 | 69 | 14 | 4 | 5 | 1 | 0 / 0 |
| NHL 12 (control, admitted) | 3 198 | 80 | 14 | 6 | 5 | — | 0 / 0 |

Four-channel streams carry two stereo sub-streams and six-channel streams three. Each sub-stream decodes to the part's count. The speaker
mapping is not established (as in handoff 05 §3).

## 2. For the developer

| # | Change | Regression inputs (bank, record → counts) |
|---|---|---|
| S1 | Add `nhl11`, `nhl13`, `nhl14` to `XMA_STREAM_TITLES` (non-looping, channel config 0 / 1) | NHL 11 `colors1.abk` #0 → 54 000 (mono); NHL 13 / 14 `ambientskatesandpunches.abk` #0 → 28 496 (stereo), #3 → 17 068 (mono) |
| S2 | Looping stereo stream records (stub + body, R92) on NHL 11 / 13 / 14, like NHL 15's channel-config-3 admission | NHL 11 `crowds.abk` #6 → stub 26 + body 113 266; NHL 13 `ambientskatesandpunches.abk` #207 → 32 + 815 616 |
| S3 | Four- / six-channel stream records as stereo pairs, with no speaker order claimed. The loop-pair (cc 3) admission currently limited to NHL 15 extends to 11 / 13 / 14 | NHL 11 `crowds.abk` #13 → 952 896 × 2; #14 → stub 24 + body 691 200 (× 2); NHL 13 `sfx.abk` #206 → 455 519 × 3 |

- Admission still requires decoded = declared at preview time.
- Recompiled stays refused until its archive route exists (handoff 04).
- The in-bank (ram) residuals are handoff 05.

## 3. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r140_ast_xma_streams_11_13_14.py <out.json>   (≈ 20 min for four titles)
```

## 4. Addendum 06:39 — NHL 10 / 15 / Legacy (answers developer receipt 53's "shapes research did not sweep"; [`data/r140b-ast-xma-streams-10-15-legacy.json`](data/r140b-ast-xma-streams-10-15-legacy.json))

Same probe (`--titles x360-nhl10,x360-nhl15,x360-legacy`). Every EA-XMA stream record is exact, with 0 framing refusals:

| Title | Mono | Stereo | Stereo loop | 4 ch | 4 ch loop | 6 ch |
|---|---|---|---|---|---|---|
| NHL 10 | 639 | 39 | 6 | 4 | 3 | — |
| NHL 15 | 3 166 | 69 | 14 | 4 | 5 | 1 |
| Legacy | 3 166 | 69 | 14 | 4 | 5 | 1 |

**Regression inputs** (only on the three titles in this addendum's table; NHL 12's loop shapes are in §1):
- stereo loop: Legacy / NHL 15 `ambientskatesandpunches.abk` #132; NHL 10 `crowds.abk` #198;
- 4-ch loop: Legacy / NHL 15 `crowds.abk` #0; NHL 10 `crowds.abk` #121;
- 4-ch: `crowds.abk` #83 (NHL 10: #144);
- 6-ch: `sfx.abk` #441.

With §1, every X360 NHL 10–15 / Legacy `.ast` EA-XMA stream shape is now swept. NHL 09 is R131.

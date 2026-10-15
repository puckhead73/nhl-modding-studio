# Handoff R71 — PS4 EA-MP3 (codec `0x0b`): the 1 152 embedded PCM samples join the decoded stream **continuously at sample 1 152** (candidate A) on 20 / 24 vectors under a linear-prediction junction test with a per-vector control, the decoded stream's first 161–880 samples are exactly silent, and the sized-frame grammar refuses every malformed variant tested (7 mutation classes × 24 vectors) once the SNS block count is checked — still a diagnostic preview, not verified game output

> **Correction note — 2026-09-23 (coordinator review items 1–2; [R75](07-correction-checkpoint-r69-r74-review.md)).** "Impossible" for hypothesis B is withdrawn: B is **not supported by this junction test** (the decoded stream carries no sample with |x| > 1 LSB before index 161–880 — a magnitude criterion, not exact-zero silence — and no consumer delay / state behaviour was tested). Candidate A has strong comparative support on the measured set (20 / 24), four vectors inconclusive; listening can establish audible plausibility, not the game's exact reconstruction rule. Mutation table per applicability: the dropped-middle-block test is **16 applicable / 16 refused / 8 not applicable** (2-block vectors), not 24; the three count checks are distinct — Σ declared block samples = `num_samples` (SNS level, the only whole-block integrity check), nine frames per full block (frame level), and block-count validation (not implemented: the SNS chain declares no block count). §3's table and §4's refusal list are read with this note.

Research lane, 22 September 2026 pm (prompt item 2; Q-R67-1; coordinator review F2). Claim `research-r71-ea-mp3-embedded-pcm`. Read-only
over the extracted PS4 banks on `H:`; temporary `.mp3` / `.wav` under the session scratchpad, removed by the probe. Nothing listened
to; nothing game-verified; no encoder; no writer. Same 24 vectors as r61 / r67 (SNS SHA-256 identical).

Machine-readable: [`data/r71-ea-mp3-embedded-pcm.json`](data/r71-ea-mp3-embedded-pcm.json) — per vector: the junction-cost values at
the candidate lags, the scan minimum, the control (another vector's PCM against the same decoded stream), the cross-correlation
result, first non-silent sample, tail measurement, every mutation's refusal reason, and the three rendering hashes (MPEG-only /
candidate A / candidate B) recorded separately. Probe `r71_ea_mp3_embedded_pcm.py` (≈ 2 min).

## Plain-language summary

The open question from this morning was what the 1 152 raw samples at the start of every EA-MP3 speech sample are for. This handoff
tests the join: if the raw samples are meant to be played *and then* the decoded MP3 audio continues from a certain point, the waveform
should flow smoothly across that point — and it should not flow smoothly when a *different* sound's raw samples are put in front of the
same decoded audio. A short linear predictor fitted on the end of the raw samples, applied across every possible join point from 0 to
2 400 samples, finds the smooth join at **exactly sample 1 152** on 20 of the 24 sounds (12 of them the single best point in the whole
scan; the control never joins as well there). Joining at 0 (raw samples *before* the whole decoded stream) is impossible: the decoded
stream begins with 161–880 samples of exact silence on every sound (a decoder warm-up), so the join would be silence, not audio. So the
raw samples stand in for the decoder's first, unusable frame — what handoff 03 called candidate A — with strong static support and
four sounds where the test is inconclusive. The listening card A-MP3-1 remains the only way to call the audible result *verified*.
Separately, the parser was attacked with seven kinds of damaged input; every one is refused by name, but a whole missing block is only
caught by checking the block count against the header, which is now part of the contract.

## 1. Junction test (T1) — the embedded PCM followed by `decoded[L:]`

Cost `J(L)`: normalised residual of an order-16 linear predictor fitted on the PCM's last 512 samples and applied across the splice
`PCM[-16:] + decoded[L:L+48]`, averaged with the mirror predictor (fitted on `decoded[L:L+512]` reversed, predicting backwards into the
PCM's last 48). Control: the same scan with the *next* vector's embedded PCM against this vector's decoded stream.

| Lag L (hypothesis) | median J, real | median J, control | vectors where real ≤ ½ control |
|---|---|---|---|
| 0 (B: pre-roll, decoded from 0) | 1.5 × 10¹¹ (decoded[0:48] is silent → undefined) | same | — |
| 529 (decoder delay) | 350 | 168 | 10 / 24 |
| 576 (encoder delay) | 232 | 179 | 10 / 24 |
| 1 105 (encoder + decoder delay) | 11.8 | 30.5 | 12 / 24 |
| **1 152 (A: PCM replaces decoded frame 0)** | **0.073** | **41.0** | **21 / 24** |
| 2 257 (A + delays) | 19.9 | 39.1 | 10 / 24 |

- Scan minimum at exactly L = 1 152 on **12 / 24**; the control's minima are scattered (1 014 … 2 323, never 1 152).
- **A-consistent** (J(1 152) ≤ max(2 × scan minimum, 0.5) *and* ≤ ½ the control at 1 152): **20 / 24**. Inconclusive: NHL 15 `332`
  (J 8.65, minimum 2.67 elsewhere), NHL 15 `5671` (11.6 vs 1.21), NHL 15 `35147` (1.12 vs control 2.2 — margin < 2 ×), NHL 17 `15935`
  (2.45 vs control 1.55). None of the four favours another candidate lag by the same criterion.
- Cross-correlation of the PCM against the decoded stream (T2) peaks at inconsistent lags (0 × 3, 199 … 2 982) with values 0.2–0.8 for
  real and control alike: the PCM is **not a copy** of any stretch of the decoded audio — consistent with A (it replaces the unusable
  frame 0) and with nothing else tested.

## 2. Decoder warm-up (T3) and tail (T4)

- First sample with |x| > 1 LSB in the decoded stream: **161 … 880** on 24 / 24 (sorted: 161, 163, 167, 170, 170, 176, 192, 201, 203,
  211, 215, 231, 243, 255, 264, 274, 277, 288, 298, 338, 861, 864, 864, 880). The decoded first frame is therefore not audio a preview
  should play from sample 0: hypothesis B (PCM then decoded from 0) would insert that silence between the PCM and the speech.
- Tail: the decoded stream carries no trailing near-silence beyond the declared trim on 21 / 24 (measured tail − trim ≤ 0; 0 or 52 on
  three). The trim (`frames × 1 152 − num_samples`, 204–1 097) therefore cuts into audible signal on most vectors, or the clips end
  abruptly; this measurement cannot separate the two and is **not evidence** for any hypothesis.

## 3. Robustness (T5) and reservoir continuity (T6)

The r67 grammar as a *refusing* parser (`contract_parse`): sizes must tile, every MPEG header valid (48 000 Hz mono), the `0x8000` flag
only on the first frame of the sample with `pcm_samples` = 1 152, `main_data_begin` ≤ the bit-reservoir bytes accumulated so far
(MPEG-1 mono: `frame_bytes − 4 − 17 − CRC`, capped at 511), nine frames per 10 368-sample block, and **Σ declared block samples =
SNR `num_samples`**.

| Mutation (per vector) | Refused | Reason recorded |
|---|---|---|
| last block truncated by 3 bytes | 24 / 24 | size exceeds payload |
| block 1 first size word pointing past the payload | 24 / 24 | size exceeds payload |
| block 1 first size word − 1 | 24 / 24 | frame does not tile its ea_frame |
| `0x8000` flag forced onto block 1 frame 0 | 24 / 24 | flag off the first frame |
| `pcm_samples` forced to 0 / 4 096 | 24 / 24 each | pcm_samples ≠ 1 152 |
| a middle block dropped | **16 / 16 applicable** (8 vectors have ≤ 2 blocks) | Σ block samples ≠ `num_samples` — **without that SNS-level check the frame grammar and the reservoir rule accepted all 16**: reservoir slack is usually full, so a missing block is invisible to them |
| unmodified | accepted 24 / 24 | reservoir consistent on every frame (T6) |

`mp3` vs `mp3float`: max |Δ| = 1 LSB on 24 / 24 (fixed / float consistency inside ffmpeg 8.1.1; not independence).

## 4. Contract input, rev. 2 (diagnostic preview; not Codex-accepted)

- Layers, verified separately per vector (hashes in the JSON): **(i)** MPEG-only PCM — frames × 1 152 samples, 48 000 Hz mono, SHA-256
  with ffmpeg 8.1.1 `mp3` (± 1 LSB with `mp3float`); **(ii)** candidate-A rendering `PCM(1 152) + decoded[1 152:]`, its own SHA-256;
  **(iii)** candidate-B rendering `PCM + decoded[0:]`, its own SHA-256 (kept only as the rejected control); tail trim to `num_samples`
  applied after (i) is verified.
- Refusals (all exercised above): non-tiling sizes, invalid header, wrong rate / mode, flag off the first frame, `pcm_samples` ≠ 1 152,
  `main_data_begin` beyond the reservoir, block count ≠ nine for a full block, Σ block samples ≠ `num_samples`.
- Presentation: **"diagnostic preview (start rendered by candidate A; not game-verified)"**. Candidate A is the *only* candidate with
  static support; it is still a candidate until the listening card A-MP3-1 or a runtime observation confirms it. Never present the
  output as verified game audio.
- Q-R67-1: **narrowed** to "A, pending audible confirmation"; B is rejected by §1–2. Replacement / encoding: unchanged, refused.

## 5. Reproduce

```
set R71_SCRATCH=<scratch dir>
python local/runs/overnight-research-2026-09-22-pm/probes/r71_ea_mp3_embedded_pcm.py docs/research/overnight-2026-09-22-pm/data/r71-ea-mp3-embedded-pcm.json --titles nhl15,nhl16,nhl17 --per-title 8   (≈ 2 min)
```
Executed: one 3-vector smoke run (NHL 15), two full 24-vector runs (the second after adding the Σ-samples check and the first-non-silent
measurement; the first run's JSON was overwritten). Temporary audio removed by the probe. Not executed: listening, encoders, writes.

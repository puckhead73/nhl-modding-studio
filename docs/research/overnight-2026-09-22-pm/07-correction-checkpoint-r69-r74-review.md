# Handoff R75 — Correction checkpoint for the coordinator review of R69–R74 (2026-09-23): the R70 headline count is corrected with a full old-to-new transition table (187 previously "decisive" levels are now not located, not 235), the R71 mutation table is regenerated per applicability with the three count checks separated and "impossible" replaced by "not supported by this junction test", and the R74 sock classes are kept separate everywhere (mirror 459 / 704, identical 153 / 124, mirror-like or other 120 / 119, flat 24 / 26)

Research lane, 23 September 2026 (first package of the continuation; applies
[research-20260922-pm-review.md](../../validation/research-20260922-pm-review.md) items 1–4). Claim `research-r75-corrections-r69-r74`.
**No raw evidence is changed; no probe was rerun**: every number below is read from the published JSONs (`r70-…`, `r71-…`,
`r74-…`) and the morning `r66-…` JSON. Dated notes are placed on handoffs 02 / 03 / 06 and the queue; the originals stay in place.

## Plain-language summary

Four wording and accounting corrections, published before any new work. The Xbox mip handoff said 235 earlier-counted levels had been
set aside; the real transition is that of the 546 levels the morning predicate called decisive, 359 are located under the strict rule
and 187 are not (96 ambiguous, 76 uninformative, 15 failing one control), while the 148 the morning called undecided stay unlocated —
and every level, old or new, was found at the same offset, so nothing moved, only the labels tightened. The speech handoff said the
"pre-roll" reading was impossible because the decoded audio starts silent; the correct statement is that it is not supported by the
junction test, and "silent" means no sample above one unit, not exact zero. Its mutation table is restated so that the dropped-block
test shows 16 applicable and 8 inapplicable sounds, and the three different count checks are named apart. The sock handoff's
classes are kept separate in every summary: two thirds of the pairs are clear mirrors, a sixth are identical because the design is
symmetric, and a sixth are mirror-like with detail differences or other — and none of that says which file the game puts on which leg.

## 1. R70 (handoff 03) — transition accounting, old predicate → strict classifier

Same 142 DXT1 / 3 / 5 descriptors, same found offsets on every level (the old and new probes agree on the offset for **694 / 694**
levels); only the labels differ.

| R66 label (morning predicate) | → located | → ambiguous | → uninformative | → controls_failed | total |
|---|---|---|---|---|---|
| decisive | **359** | 96 | 76 | 15 | **546** |
| undecided | 0 | 55 | 93 | 0 | **148** |
| total | 359 | 151 | 169 | 15 | 694 |

So: **187** levels the morning counted as evidence (its "decisive" class, all sizes) are not located under the strict rule; the
headline's "235" was wrong (it mixed the old-decisive subset with old-undecided levels). The 359 located results are unchanged and stay
separate from the 335 excluded cases (151 + 169 + 15). H1 agreement on located levels remains 359 / 359.

Retained wording (review item 3): a classifier score on generated output is **one diagnostic**, not a sufficient writer acceptance
oracle; the unsupported write gate stays intact and no mip-write enablement follows from R70.

## 2. R71 (handoff 02) — wording and the mutation table

- **Candidate B.** "Impossible" is withdrawn. Correct statement: hypothesis B (embedded PCM followed by the decoded stream from sample
  0) is **not supported by this junction test** — the cost at L = 0 is undefined-large because the decoded stream carries no sample with
  |x| > 1 LSB before index 161–880 (that criterion is *magnitude ≤ 1 LSB*, not exact-zero silence), and no consumer-side delay or state
  behaviour was tested. Candidate A has strong comparative support on the measured set (20 / 24 A-consistent; 4 inconclusive).
- **Listening (card A-MP3-1).** A listening pass can establish *audible plausibility* of a rendering; it does not by itself identify the
  game's exact reconstruction rule. Both distinctions are now on the card and in the queue.
- **Mutation table, per applicability** (from `r71-ea-mp3-embedded-pcm.json`; 24 vectors; 8 vectors have exactly 2 blocks):

| Mutation | applicable | refused | accepted | reason recorded |
|---|---|---|---|---|
| last block truncated by 3 bytes | 24 | 24 | 0 | size exceeds payload |
| block 1 first size word past the payload | 24 | 24 | 0 | size exceeds payload |
| block 1 first size word − 1 | 24 | 24 | 0 | frame does not tile its ea_frame |
| `0x8000` flag forced onto block 1 frame 0 | 24 | 24 | 0 | flag off the first frame |
| `pcm_samples` forced to 0 | 24 | 24 | 0 | pcm_samples ≠ 1 152 |
| `pcm_samples` forced to 4 096 | 24 | 24 | 0 | pcm_samples ≠ 1 152 |
| a middle block dropped | **16** (8 not applicable: 2-block vectors) | 16 | 0 | Σ declared block samples ≠ SNR `num_samples` |
| unmodified | 24 | 0 | 24 (accepted) | — |

- **Three different count checks, named apart:** (a) **Σ declared block samples = SNR `num_samples`** (SNS level; the check that
  catches a dropped or duplicated whole block); (b) **nine frames per full 10 368-sample block** (frame level; catches a block whose
  frame chain is short or long); (c) **block-count validation** — *not* implemented: the SNS block chain has no declared block count,
  so there is nothing to validate a count against; (a) is the only whole-block integrity check available statically. The handoff's
  earlier sentence folding these together is superseded.

## 3. R74 (handoff 06) — sock pair classes kept separate

| Title | pairs | clear mirror (mirror MAD ≤ 1.5, ≥ 4 × better than direct) | identical content (symmetric design) | mirror-like / other (direct 5–16, mirror 2–6, or symmetric with direct = mirror) | flat |
|---|---|---|---|---|---|
| X360 NHL 11 | 756 | 459 | 153 | 120 | 24 |
| X360 NHL 12 | 973 | 704 | 124 | 119 | 26 |

"Every pair is a mirror" is **not** claimed anywhere; the summary and the queue now carry the four classes. Both files remain required
components; their runtime assignment (which leg / which selector consumes which file) is **not inferred** (Q-R74-1, card K-SOCK-1).

## 4. Accepted consumption boundaries (restated from the review, unchanged)

H1 = candidate, no mip-write enablement; venue discovery read-only with every candidate, missing folder and typed dependency shown;
kit listings may use the title / platform grammar with the NHL 11 / 12 sock components separate, PS3 12 / 13 through archive-member
evidence and their own decode route; EA-MP3 as explicitly diagnostic parsing / preview with start interpretation and decoder / version
provenance visible; no encoding, no verified game-equivalent audio.

## Changed paths

Dated notes only: handoffs 02 (§1–4 wording; mutation table), 03 (headline count; §2 transition), 06 (O-2 classes), `DEVELOPER-QUEUE.md`
rows 1 / 2 / 4, `USER-CARDS.md` A-MP3-1 / K-SOCK-1, claims `research-r70` / `r71` / `r74` (`result` field). New: this handoff, claim
`research-r75-corrections-r69-r74`. Unchanged: every `data/*.json`, every probe.

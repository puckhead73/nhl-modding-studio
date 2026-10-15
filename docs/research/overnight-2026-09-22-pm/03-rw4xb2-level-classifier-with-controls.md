# Handoff R70 — Xbox 360 `RW4xb2` mip positions re-classified with a self-contained rule and four independent controls: **359 levels are located** (informative target, unique optimum with ≥ 2 × margin, misplacement / byte-order / decoded-content controls all pass) and **every one of them sits where the candidate rule H1 predicts** (own-tile 323 / 323, packed ≥ 4 × 4 blocks 29 / 29, packed smaller 7 / 7) — while 235 levels the morning's predicate had counted are now *uninformative* or *ambiguous* and carry no weight either way. H1 stays a candidate: 25 shapes, DXT1 / 3 / 5 only, nothing game-verified, no write contract

> **Correction note — 2026-09-23 (coordinator review item 3; [R75](07-correction-checkpoint-r69-r74-review.md)).** The headline's "235 levels the morning's predicate had counted are now uninformative or ambiguous" is wrong. Transition on the same 694 DXT levels (identical found offsets on 694 / 694): R66-decisive 546 → located 359, ambiguous 96, uninformative 76, controls_failed 15 (**187** no longer counted); R66-undecided 148 → ambiguous 55, uninformative 93. The 359 located results are unchanged and separate from the 335 excluded cases. A classifier score on generated output is one diagnostic, not a sufficient writer acceptance oracle; no mip-write enablement follows.

Research lane, 22 September 2026 pm (prompt item 1; coordinator review F1; supersedes the classification of the morning handoff 02).
Claim `research-r70-rw4xb2-level-classifier`. Read-only over the loose `rendering/**/*.rx2` trees of NHL 09 / 10 / 13 / Legacy (X360;
the same descriptor selection as r66: first 4 000 files, two descriptors per shape / mip-count class, 2D, ≥ 3 mips, **DXT1 / DXT3 / DXT5
only** — 35 / 39 / 35 / 33 descriptors; DXN 3 299 and `8_8_8_8` 542 descriptors skipped and counted, never scored). Nothing
game-verified; no writer; no visual review.

Machine-readable: [`data/r70-rw4xb2-level-classifier.json`](data/r70-rw4xb2-level-classifier.json) — every descriptor and level with
**unrounded** decision values (best score, the eight best other basins refined to 16 B with their scores, margin ratio / absolute, ties,
near-best fraction, target std and distinct colours, crowd median / p05 / p01), the control scores (± one page, ± one block, + 16 B,
+ one level tile, little-endian read), the decoded-content test (both X360 index-byte orders, the found offset, + one page, the runner-up),
the gate results, the class, and the H1 prediction; per-title summaries; the four synthetic controls. Probe
`r70_rw4xb2_level_classifier.py` (≈ 10.5 min for the four titles; `--synthetic` runs the in-memory controls).

## Plain-language summary

The review found that this morning's "decisive" label only compared the best match against the crowd of other positions. This handoff
replaces it with one rule applied to every level, agreement or not: the target must actually contain structure (flat or two-colour
content is *uninformative*); the best position must beat every other basin by at least two to one and never be tied (*ambiguous*
otherwise); and at the winning position three independent checks must pass — the same test one page, one block and one tile away must
be at least twice worse, reading the colour words in the wrong byte order must be at least twice worse, and the level's texels, fully
decoded from palette and indices, must match the fully decoded parent shrunk by half, and match it far better than one page away. On
synthetic textures the rule behaves as required: a constant texture and a checkerboard are refused as uninformative, a noise texture laid
out by H1 is located at H1, and the same texture with one level deliberately moved by a page is located at the *moved* place and flagged
as disagreeing. On the real trees 359 levels pass all gates and every single one sits where H1 says, including seven of the tiny levels
the morning could not judge; 235 levels that the old predicate had counted as evidence are now set aside as uninformative or ambiguous,
and 15 more fail one control by a small margin on high-contrast banner text (all of them also at H1, but not counted). Nothing here
makes H1 a write contract: it is a candidate rule that no located level contradicts on 25 shapes and three formats.

## 1. Classifier (one rule for every level; thresholds in the JSON)

| Gate | Test | Threshold |
|---|---|---|
| informative | block-mean std of the parent-derived target ≥ 2.0 (0–255 scale) **and** ≥ 4 distinct quantised block colours **and** crowd `median − p05` ≥ 2.0 | fails → **uninformative** |
| unambiguous | best refined score vs the best refined score of the eight strongest *other* coarse basins (each refined at 16 B): ratio ≥ 2.0, absolute margin ≥ 1.0, no basin within 5 % | fails → **ambiguous** |
| misplacement control | score at found ± 4 096 B, ± one block, + 16 B, + one level tile ≥ 2 × best | fails → **controls_failed** |
| byte-order control | the same gather with little-endian RGB565 words ≥ 2 × best | fails → controls_failed |
| decoded-content control | full DXT colour decode (palette + 2-bit indices; both X360 index-byte orders tried, the better one recorded) of the level at the found offset vs the 2 × 2 box reduction of the fully decoded parent: MAD at + one page and at the runner-up ≥ 2 × MAD at the found offset | fails → controls_failed |

Only **located** levels (all gates) are compared with H1. Levels are followed down the chain from the found offset as r66 did; a level
whose parent was not located is still classified on its own gates.

Synthetic controls (`--synthetic`, 128 × 128 DXT1 × 6, real mip chains encoded with a minimal DXT1 encoder): constant → uninformative on
every level; 8-texel checker → uninformative on every level; noise laid out by H1 → located at H1 on levels 1–2, ambiguous on the
16 × 16-texel level 3 and the 8 × 8 level 4 (noise has no large-scale structure, so small levels tie); noise with **level 2 shifted by one
page** → located at the *shifted* offset and `h1_agree = false`. Misplacement is therefore detected, and flat / repeated content is
never "found".

## 2. Results (four titles; `summary` per title in the JSON)

| Class | own-tile ≥ 4 × 4 blocks | packed ≥ 4 × 4 blocks | packed < 4 blocks in a dimension | total |
|---|---|---|---|---|
| **located** | **323** (DXT1 202, DXT3 95, DXT5 26) | **29** (15 / 12 / 2) | **7** (DXT1) | **359** |
| ambiguous | 28 | 66 | 57 | 151 |
| uninformative | 62 | 35 | 72 | 169 |
| controls_failed | 10 | 5 | 0 | 15 |

- **H1 agreement on located levels: 359 / 359** — no disagreement on any title, format or placement kind. (The morning's three
  "disagreements" are now: `clipboard_0_0_am` level 1 → uninformative; `poles_sm` level 2 → uninformative; the DXN records → not scored.)
- Control margins over the 359 located levels: misplacement ratio min 2.35 / median 4.35; little-endian ratio min 2.19 / median 16.6;
  decoded-content ratio (+ one page vs found) min 5.2 / median 20.7; basin margin ratio min 2.02 / median 4.27.
- Coverage: 110 / 142 descriptors have at least one located level; 25 distinct shapes (2 048 × 1 024, 512², 1 024², 512 × 256,
  1 024 × 512, 1 024 × 2 048, 128², 256², 256 × 1 024 … ); by level: 1 → 103, 2 → 98, 3 → 83, 4 → 59, 5 → 12, 6 → 3, 7 → 1.
- **Packed positions among located levels** (bytes past the H1 tail base; each equals the texel-offset rule): DXT1 16 × 16 → +256 (3),
  16 × 64 → +256 (4), 32 × 16 / 64 × 16 → +1 024 (8), **16 × 8 → +512 (3), 32 × 8 → +512 (4)** (the two < 4-block shapes, located with all
  gates); DXT3 16 × 16 → +512 (4), 32 × 16 / 64 × 16 → +4 096 (8); DXT5 16 × 16 → +512 (2).
- `DW12` = level-1 tile base on **103 / 103** records whose level 1 is located (own tile, or the tail when level 1 is already packed).
- The 15 controls_failed levels: 12 fail only the byte-order ratio (little-endian 1.5–1.9 × the big-endian score on high-contrast DXT3
  banner / year text where the endpoint-mean score itself is 14–16) and 3 fail the ± one-block ratio (1.9 ×); all 15 are at the H1 offset
  and all pass the decoded-content control (decoded MAD 4–7 vs 50–130 one page later). They are reported, not counted.
- **Index-byte order (new format observation):** with the decoded-content test the pairwise-swapped index bytes (`i1 i0 i3 i2`) give a
  clearly lower MAD than the PC order on 321 / 359 located levels (ratio < 0.5) and were preferred on all 359 — consistent with the
  16-bit word swap of X360 DXT blocks. This is an observation from the same data, not an independent verification.

## 3. What H1 is now

- A **candidate placement rule** for DXT1 / DXT3 / DXT5 2D mipped `RW4xb2` textures on NHL 09 / 10 / 13 / Legacy that **no located level
  contradicts** (359 / 359 on 25 shapes), with the misplacement-detecting classifier and controls the review asked for.
- **Not** a full-chain write contract: 320 levels are unresolved by this data (ambiguous / uninformative) — most of the packed tail and
  most levels below 4 × 4 blocks; 16-wide strips, DXN, CTX1, `8_8_8_8` and cubes are untested; whether the GPU tolerates anything else is
  not a data question. "A self-generated file scored against H1" remains insufficient on its own: the acceptance test for any writer is
  the *full* R70 classifier on the written file (all gates, synthetic misplacement included), plus the independent-controls list of R69.
- Donor-tail preservation is **not** a general fallback (R69 F1); an identity-preserving case must be demonstrated by the same test.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r70_rw4xb2_level_classifier.py docs/research/overnight-2026-09-22-pm/data/r70-rw4xb2-level-classifier.json --titles nhl09,nhl10,nhl13,legacy --max-files 4000 --per-shape 2 --synthetic   (≈ 10.5 min)
```
Executed: three synthetic-control runs (the first two exposed a flaw in the synthetic generator — flat blocks with no index variation
fail the decoded test by construction — fixed by encoding real mip chains), one aborted four-title run (output directory missing; the
first title's work was discarded and the run restarted), the final four-title run. Scratch JSONs in the session scratchpad, removed at
closeout. Inputs: the X360 loose `rendering` trees (paths in `r42_rw4xb2_slot_join.py`).

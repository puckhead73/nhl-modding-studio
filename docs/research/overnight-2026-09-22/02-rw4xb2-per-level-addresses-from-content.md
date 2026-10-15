# Handoff R66 — Xbox 360 `RW4xb2` mip levels located from **content**: a per-level address contract (H1) that the found positions confirm on every level of 4 × 4 blocks or larger — own 32 × 32-block tile per level while the smaller texel dimension exceeds 16, then one packed tail tile with the Xenos texel-offset placement — on NHL 09 / 10 / 13 / Legacy (386 / 387 own-tile and 76 / 78 packed decisive levels agree; the three exceptions are flat textures the test cannot score). The three smallest levels (≤ 8 × 8 texels) are **not** established by this method

> **Correction note — 2026-09-22 pm (coordinator review F1; research handoff [R69](../overnight-2026-09-22-pm/01-correction-checkpoint-f1-f3-r66-r68.md)).** The decisive predicate below (best < 0.5 · p05 and < 0.35 · median) tests neither a unique optimum nor a runner-up margin nor content informativeness; the recorded scores are rounded; the three disagreements were excluded after the fact while agreements were not re-examined; and DXN levels entered the contract check's grand totals although the text excludes them. `clipboard_0_0_am` (p05 63.75, median 255.0) is an *uninformative* level (constant parent target), not a flat-crowd artefact. **Withdrawn as stated:** the 386 / 387 and 76 / 78 totals as a classification result (they hold *under the R66 predicate* only); H1 as "the per-level address contract" (it is a **candidate with substantial partial static support** on levels ≥ 4 × 4 blocks); the §4 acceptance test as sufficient on its own (a self-generated file scored against the same layout needs independent controls: known misplacement, tied / flat content, wrong byte order, independently decoded per-level content); donor-tail preservation as a general resize fallback (refuse the unsupported mutation unless an identity-preserving case is demonstrated); and the `texlib.rs` debug-blue comment as anything but historical context. Endpoint means and box reduction are approximate evidence, not texture reconstruction. **Preserved unchanged:** every found offset, score and JSON, the base-level untiling validation, the `DW12` observation and the packed-position table. The reclassification with independent controls is research R70 (2026-09-22 pm campaign).

Research lane, 22 September 2026 (prompt priority 1; answers Q-R65-1 / Q-R60-1 as far as the data allows). Claim
`research-r66-rw4xb2-level-addresses`. Read-only over the loose `rendering/**/*.rx2` trees of NHL 09 / 10 / 13 / Legacy (X360; the first
4 000 files of each tree, two descriptors per shape / mip-count class, DXT1 / DXT3 / DXT5, 2D, ≥ 3 mips: 35 / 39 / 35 / 33 descriptors,
30 distinct shapes). Nothing game-verified; no writer; no visual review (deferred).

Machine-readable: [`data/r66-rw4xb2-level-addresses.json`](data/r66-rw4xb2-level-addresses.json) (every descriptor: slot, header size,
`DW12`, and per level the found offset, its score, the median / 5th-percentile candidate score, the runner-up and the score one page
later) and [`data/r66-contract-check.json`](data/r66-contract-check.json) (H1 agreement per title / format / placement kind, every
disagreement listed). Probes `r66_rw4xb2_level_addresses.py` (≈ 2–3.5 min per title at `--max-files 4000`) and `r66_contract_check.py`
(reads the JSON only).

## Plain-language summary

The coordinator's review asked for evidence of *where* each smaller copy of a texture sits, not just how much room the texture takes.
This handoff finds the copies by looking at what is in them: each smaller level must be a half-size version of the one above, so for
every candidate position in the texture's data the probe reads the blocks there through the console's tiled addressing and asks how
well they match the shrunken parent. The right position stands out sharply — typically 10–30 times better than the median candidate
and far better than the same position one page later — so a *misplaced* level is detectable, which is the test the review required. On
all four titles the positions found follow one rule: each level gets its own 32 × 32-block tile as long as its smaller side is over 16
texels; from the first level at or below 16 texels, all remaining levels share one final tile, placed at a texel offset equal to the
level's own size (to the right for square or tall levels, below for wide ones). That rule is confirmed for every level down to 4 × 4
blocks (16 × 16 texels). For the last three levels (8 × 8, 4 × 4, 2 × 2 texels) the blocks are too few for the test to tell positions
apart, so their placement is stated as the same rule's *prediction* and marked unverified. One mistake in yesterday's probe was found on
the way: it read each texture's data 4 KiB early (the file-table offsets are relative to the end of the header), which is why its
extents were meaningless — recorded in handoff 01 and handoff 17.

## 1. Method (what "found" means)

1. The slot is read at `hdr_size + off` (`hdr_size` = u32 BE at `0x44`, 0x1000 on every file seen).
2. Level 0 at offset 0 is untiled with the Xenos 2D tiled address (`XGAddress2DTiledOffset`, the public Xbox 360 XDK helper; the same
   function Studio's `texlib.rs` uses). Its correctness is checked on the data rather than assumed: verified bijective on 32 × 32 and
   64 × 32 block grids; on a structured texture (`adboard_trans_rgb`, 512 × 128 DXT1) the untiled block-mean map is a clean framed
   rectangle while the linear reading is scrambled; and level 1 found by the search below is a 2 × 2 reduction of it.
3. For level k ≥ 1, every candidate offset (512 B steps in slots > 256 KiB, 128 B otherwise, then refined at 16 B) is scored: the level's
   block grid is gathered through the tiled address from that offset, reduced to per-block mean endpoint colour, and compared (mean
   absolute difference, 0–255) with the 2 × 2 box reduction of the parent level as found. The parent is then the found level, so the
   chain is followed, not assumed.
4. A level counts as **decisive** only when its best score is below half the 5th-percentile candidate score *and* below 0.35 × the
   median. Non-decisive levels are listed as undecided and used neither for nor against. (Three "disagreements" below have score 0.000
   with a near-zero 5th percentile — flat or zero textures — and are metric artefacts, not evidence.)
5. Formats: DXT1 / DXT3 / DXT5 (RGB565 endpoints readable). DXN was walked but has no colour endpoints and is excluded from the
   contract check; `8_8_8_8` / CTX1 were not sampled.

Discrimination, measured: over the 462 decisive levels of ≥ 4 × 4 blocks, the winning score is typically 2–10 against medians of 20–80
and 5th percentiles of 6–45; the score one 4 KiB page after the found offset is 5–50 × worse.

## 2. Contract H1 (established for levels of ≥ 4 × 4 blocks)

For a 2D texture `w × h`, format block size `bs` (4 for DXT) and bytes per block `bpb` (8 DXT1 / CTX1, 16 DXT3 / DXT5 / DXN), level k
has texel dims `(max(1, w >> k), max(1, h >> k))` and block dims `bw_k = ceil(w_k / bs)`, `bh_k = ceil(h_k / bs)`:

- **Own tile.** While `min(w_k, h_k) > 16` texels: level k occupies `align32(bw_k) × align32(bh_k) × bpb` bytes at the running offset,
  tiled with the Xenos address over its own block grid (width `bw_k`); the running offset starts at 0 (base) and advances by that size.
- **Packed tail.** The first level with `min(w_k, h_k) ≤ 16` opens the tail: one `32 × 32 × bpb`-byte tile at the running offset. Every
  remaining level is placed inside it at block position `(0, h_k / bs)` when `w_k > h_k` (wide: below), else `(w_k / bs, 0)` (square or
  tall: to the right), i.e. `tail + XGAddress2DTiledOffset(px, py, 32, bpb) × bpb`.
- `DW12` (the descriptor's mip address, 4 KiB units) is the **tile base of level 1** — the level-1 own tile, or the tail tile when level 1
  is already packed (32 × 32 DXT1 × 6 mips: `DW12` = 8 192 = tail base, level 1 found at tail + 256): 141 / 142 records; the exception
  is the 16 × 256 strip already known as the one `DW12` miss per title (handoff 02 §1), whose base level is itself ≤ 16 wide — an
  **unresolved shape**, listed below.

### Evidence (decisive levels only; `data/r66-contract-check.json`)

| Title | own-tile levels ≥ 4 × 4 blocks: agree / disagree / undecided | packed levels ≥ 4 × 4 blocks | packed levels < 4 blocks in a dimension |
|---|---|---|---|
| NHL 09 | 86 / 1* / 12 | 17 / 0 / 16 | 5 / 13 / 15 |
| NHL 10 | 105 / 0 / 14 | 28 / 0 / 10 | 9 / 18 / 12 |
| NHL 13 | 100 / 0 / 5 | 16 / 1* / 16 | 6 / 12 / 15 |
| NHL Legacy | 95 / 0 / 5 | 15 / 1* / 15 | 5 / 13 / 13 |

\* `clipboard_0_0_am` (64², level 1 found at 0, score 0.000 / p05 63.8 — a constant texture) and `banner/poles_sm` (64², level 2, score
0.000 / p05 0.084 on both titles) — zero-score flat content; not evidence.

Packed positions confirmed (bytes past the tail base; every one equals the H1 texel-offset rule): DXT1 16 × 16 → +256 (21), 16 × 32 /
16 × 64 (tall) → +256 (8), 32 × 16 / 64 × 16 (wide) → +1 024 (19); DXT3 16 × 16 → +512 (18), 32 × 16 / 64 × 16 → +4 096 (20); DXT5
16 × 16 → +512 (8). Identical on all four titles.

## 3. Not established

- **Levels of ≤ 8 × 8 texels (< 4 blocks in a dimension, i.e. the last three levels of a full chain).** 2 × 2- and 4 × 2-block levels carry
  4–8 endpoint pairs; the search is not decisive (agree 25, disagree 56, undecided 55, with no separation between them). H1's
  prediction for them (`(w/bs, 0)` or `(0, h/bs)`, which reaches block 0 for anything under 4 texels) is stated as a **candidate**. A
  writer must treat those bytes as unverified: either derive them from an independent source (a Xenos hardware-behaviour reference or a
  runtime check) or keep them out of any correctness claim.
- **16-wide strips** (`ice/texlib_0`, 16 × 256 DXT3, `DW12` = 0): base level already ≤ 16 wide; the found level 1 sits at offset 0 with
  a weak score. One shipped family per title; unresolved.
- **DXN, CTX1, `8_8_8_8`, cube maps**: not tested here (DXN needs a BC5 decoder; cubes need a face-aware search).
- Nothing about the runtime: this is the shipped layout, measured; whether the GPU tolerates anything else is not a question data can answer.

## 4. Consequence for the developer (resize / chain writer; not Codex-accepted)

- H1 is the per-level address contract the review asked for, **down to 4 × 4-block levels**, with a misplacement-detecting acceptance
  test: run `r66_rw4xb2_level_addresses.py`'s search on the *written* file and require every level ≥ 4 × 4 blocks to be found at H1's
  offset with a decisive score. Presence of the right byte count is not that test.
- The old writer comment in `texlib.rs` ("players rendered debug-blue at gameplay distance") is consistent with a tail placed elsewhere
  than H1; audit `replace_with_mip_chain` against §2 rather than declaring it broken from the withdrawn R60 numbers.
- Allocation: the H1 chain length (`Σ own tiles + one tail tile`, 8 KiB-rounded) is a *derived* length; the shipped slot lengths of
  handoff 02 remain the allocation record. Where they differ, prefer the shipped slot for shipped shapes and say why.
- Refuse (or leave the donor tail untouched and say so) for the unestablished cases of §3.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-22/probes/r66_rw4xb2_level_addresses.py docs/research/overnight-2026-09-22/data/r66-rw4xb2-level-addresses.json --titles nhl09,nhl10,nhl13,legacy --max-files 4000 --per-shape 2 --endian be   (≈ 11 min)
python local/runs/overnight-research-2026-09-22/probes/r66_contract_check.py docs/research/overnight-2026-09-22/data/r66-rw4xb2-level-addresses.json docs/research/overnight-2026-09-22/data/r66-contract-check.json
```
Executed: two NHL 13 smoke runs (40 files; the first before the `hdr_size` fix, both in the scratchpad, removed), eight scratch analyses
(`x1`–`x7`: full DXT1 decode smoothness, boundary energy, byte-order variants, permutation families, distribution map, ASCII maps —
removed at closeout), the four-title run, the contract check. Inputs: the X360 loose `rendering` trees (paths in `r42_rw4xb2_slot_join.py`).

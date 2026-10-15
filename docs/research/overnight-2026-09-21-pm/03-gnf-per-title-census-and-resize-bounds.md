# Handoff R43 — PS4 `GNF` header census on all seven titles (complete): pitch rule on the full population; BC6H (`0x28`) modelled; NHL 15 ships 1 115 two-stream files; shipped maxima vs the field bounds

> **Correction note — 2026-09-22 (coordinator review F3; research handoff [R65](../overnight-2026-09-22/01-correction-checkpoint-f1-f3.md)).**
> The run is **complete on all seven titles** — the reproduction footer's "in progress / only NHL 20 done" wording is superseded.
> Multi-stream `.gnf` files are **NHL 15 (1 115) and NHL 19 (5 342)**, 6 457 in total, not NHL 15 alone. The census walked each file's
> **first** `GNF ` stream, so a pass is a first-stream rule check: a multi-stream file is **not** resize-qualified by it and a writer must
> preserve every stream or refuse the file by name. Shipped maxima (8 192 × 4 096, 14 levels) are observations, not runtime limits.
> BC6H byte-size agreement establishes the 16-byte block for size arithmetic only — not decoded-pixel or encoder correctness.

Research lane, 21 September 2026 pm, roadmap step 1.3 / 2.5 / 4.x PS4 (prompt step 2: "restore reproducible … GNF pitch probes; complete
missing per-title coverage incrementally with checkpointable per-title output; do not discard completed titles because a larger census
cannot finish"). Claim `research-r43-gnf-per-title`. Read-only over the extracted PS4 trees on `H:`; nothing game-verified; no writer.
**Complete as of 22 September 2026, 05:26 MDT**: all seven titles are censused (§1); the per-title JSONs are the durable record.

Machine-readable: `data/r43-gnf-<title>.json`, one per completed title (classes, checks, shipped maxima, formats, pitch by non-multiple
width, BC6H test, `d5`/`d6`, tail tags, shapes, families, failure lists incl. multi-stream files). Probe `r43_gnf_header_census_per_title.py`
(≈ 20–45 min per title on the slow H: drive; `--titles` order is honoured; a title's file is written only when its walk completes).

## Plain-language summary

The PS4 texture-header rules of the 2026-09-21 handoff 15 are now measured on the real NHL 20 install — not implied from NHL 19 or 21 —
and hold on all 135 774 files whose format the rule models (100 %), with the 145 exceptions being cube maps and a further 2 821 BC6H
files now modelled too (a 16-byte block; every one passes). The pitch rule is tested on every file, not a sample (138 740 / 138 740).
NHL 20's largest shipped texture is 8 192 × 4 096 with 14 levels (25 MB of data) — twice the NHL 15 maximum — so the field bounds of
handoff 02 §4 (16 384) are well above what EA shipped, and no NHL 20 file exceeds them. NHL 15 (first pass) revealed a container
variant this census must handle: 1 115 of its `.gnf` files are two `GNF ` streams back to back (a second, mostly identical-shape texture
after the first; the first stream's `streamSize` is exact), which a writer must treat as a multi-stream file rather than "streamSize ≠
file size" — and **NHL 19 ships 5 342 of them** (the largest population; NHL 16 / 17 / 18 / 20 / 21 ship none). All seven titles have
now landed, including the NHL 15 rerun with the multi-stream walk, so the table below is the complete census: every title passes the
size rule on 100 % of its predicted-format files and the pitch rule on 100 % of its readable files, and the shipped ceiling is
8 192 × 4 096 / 14 levels from NHL 18 on (4 096 × 4 096 / 13 before).

## 1. Coverage (complete: seven titles)

| Title (install) | walked | readable | predicted-format | **size rule pass** | cube / array | BC6H `0x28` (16 B block rule) | not `GNF ` | multi-stream files | pitch rule (all readable) | shipped maxima (w × h, levels, data) |
|---|---|---|---|---|---|---|---|---|---|---|
| NHL 20 (CUSA15439, real install) | 140 142 | 138 740 | 135 774 | **135 774 / 135 774** | 145 (type 11; mipped cubes = 8 × the 2D prediction, single-level 6 ×) | **2 821 / 2 821 pass** | 1 402 | 0 | **138 740 / 138 740** (20 456 non-multiple-of-32 widths → `roundup(w, 32) − 1` on every one) | 8 192 × 4 096, 14 levels, 25 165 824 B |
| NHL 15 (CUSA00561) | 49 309 | 48 309 | 48 287 | **48 287 / 48 287** | 22 | 0 | 1 000 | **1 115** (all tile the file; 1 083 repeat the first stream's shape) | 48 309 / 48 309 (978 non-multiple widths) | 4 096 × 4 096, 13 levels, 22 374 400 B |
| NHL 16 (CUSA01909) | 68 849 | 68 083 | 68 060 | **68 060 / 68 060** | 23 | 0 | 766 | 0 | 68 083 / 68 083 (3 816) | 4 096 × 4 096, 13 levels, 11 187 200 B |
| NHL 17 (CUSA04326) | 83 572 | 82 667 | 82 641 | **82 641 / 82 641** | 26 | 0 | 905 | 0 | 82 667 / 82 667 (9 342) | 4 096 × 4 096, 13 levels, 5 611 520 B |
| NHL 18 (CUSA07569) | 105 551 | 104 369 | 104 342 | **104 342 / 104 342** | 27 | 0 | 1 182 | 0 | 104 369 / 104 369 (11 340) | 8 192 × 4 096, 14 levels, 5 611 520 B |
| NHL 19 (CUSA11126) | 124 292 | 122 900 | 121 585 | **121 585 / 121 585** | 103 | **1 212 / 1 212** | 1 392 | **5 342** (all tile the file; 4 971 repeat the shape) | 122 900 / 122 900 (16 819) | 8 192 × 4 096, 14 levels, 25 165 824 B |
| NHL 21 (CUSA18182) | 149 613 | 148 066 | 145 136 | **145 136 / 145 136** | 208 | **2 722 / 2 722** | 1 547 | 0 | 148 066 / 148 066 (22 755) | 8 192 × 4 096, 14 levels, 22 374 400 B |

Cube maps grow with the generation (22 → 208 from NHL 15 to 21). Other NHL 20 facts: `streamSize` = file size, `d7` = data bytes, `contentsSize` `0xf8`, tiling 13, `base_level` 0, `d0` 0 on all 138 740;
`d5` non-zero exactly on the 145 cube maps (`d6` never); 16-byte tail: zeros 137 436, `USER` 1 304; formats BC1 68 976, BC7 46 292, BC3
9 940, BC5 7 753, BC6H 2 919 (2 821 2D + 98 among the cubes / others), BC4 2 830, BC2 21, 8_8_8_8 9.

## 2. Rules confirmed / refined

- **Pitch:** `roundup(w, 32) − 1` for every block format including BC6H (full population; the pairs `4 → 31`, `36 → 63`, `164 → 191`,
  `332 → 351`, `2 748 → 2 751` … are in the JSON per width).
- **BC6H (`0x28`):** data = Σ levels of 8 × 8-block micro-tiles × **16 B** — 2 821 / 2 821 on NHL 20, 2 722 / 2 722 on NHL 21, 1 212 / 1 212
  on NHL 19, none shipped on 15–18 (closes the Q-R31-1 block-size item; the swizzle is still the tiling-13 micro-tile, nothing else).
- **Cube maps:** `type` 11; data = 6 × (single level) or 8 × (mipped) the 2D prediction — recorded, not modelled; `d5` ≠ 0 marks them.
- **Multi-stream files (NHL 15: 1 115; NHL 19: 5 342; none on 16 / 17 / 18 / 20 / 21):** `streamSize` < file size and a second `GNF ` header at offset `streamSize`; the census evaluates the
  rules on the first stream and records the extra streams' shapes. A resize writer must refuse or explicitly handle the second stream
  (it is not padding); the developer's `check_resize_rule` ("streamSize = file size") would refuse these — correct, but the reason should
  say "multi-stream file" rather than "not reproducible".

## 3. Field bounds vs shipped (PS4; the developer's constants stand)

Field bounds (handoff 02 §4): width / height ≤ 16 384, levels ≤ 15 at that size, pitch never binding, `d7` / `streamSize` u32. Shipped: NHL 20
reaches 8 192 × 4 096 / 14 levels, as do NHL 18 / 19 / 21; NHL 15 / 16 / 17 stop at 4 096 × 4 096 / 13. A request of 16 384 × 16 384 is representable and unshipped; a request above
16 384 is refused by the field (the reviewed 16 400 case). Nothing here bounds what the console *loads* (card R31-GNF-1 rev. 2).

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r43_gnf_header_census_per_title.py docs/research/overnight-2026-09-21-pm/data --titles nhl20   (≈ 42 min; each title ≈ 20–45 min)
```
Executed: NHL 20 smoke (1 500 files) + NHL 15 first pass (full) + NHL 15 smoke of the multi-stream walk (3 000 files) + the full per-title
run, **completed on all seven titles** (JSONs `r43-gnf-nhl15/16/17/18/19/20/21.json`; corrected 2026-09-22, review F3 — the earlier
"in progress (20 done)" wording described the state at the time of writing). Each file's **first** `GNF ` stream is the one the rules were
checked against. Not executed: writes, launches.

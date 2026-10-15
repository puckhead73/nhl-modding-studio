# Handoff R60 — Xbox 360 `RW4xb2` data slots: the shipped slot is a padded 8 KiB allocation and is never smaller than its content; the mip-chain content extent is measured per shape (a packed-tail oracle) — the packed-tail *formula* is not derived

> **STATUS: INVALID / SUPERSEDED — 2026-09-22 pm ([research R69](../overnight-2026-09-22-pm/01-correction-checkpoint-f1-f3-r66-r68.md), coordinator review of 2026-09-22).** The extent tables in §1 / §3 are **not measurements of the intended slots** (every window was read `hdr_size` = 4 KiB early, see the defect note below) and, even where correctly placed, would be lower bounds inside an already-bounded slot. The raw JSON and the probe are retained unchanged as the historical record. **No developer allocation or layout assertion may cite this handoff as qualifying evidence.** Successor evidence: R66 as bounded by R69 F1, then R70.

> **Correction note — 2026-09-22 (coordinator review F1; research handoff [R65](../overnight-2026-09-22/01-correction-checkpoint-f1-f3.md)).**
> The measurement below is the offset of the last **non-zero** byte inside a slot the probe had already bounded by the file table, and the
> slot lengths are 8 KiB-aligned. Valid texture bytes can be zero, so this extent is a **lower bound** on the addressed region and
> `slot_lt_extent4k == 0` is a consequence of the measurement boundary, not evidence. **Withdrawn:** that this validates the resize rule,
> that keeping the slot never truncates, that the §3 maxima are what to allocate, and that aggregate extent agreement establishes the
> single-level (or any) layout. In §2 only the "under" counts are evidence against a model; "over" is neutral (a larger prediction may
> cover valid zero bytes or padding). **Preserved unchanged:** the slot-size and non-zero-extent distributions, the 8 KiB granularity, the
> cross-title identity of §3 and `data/r60-rw4xb2-slot-data-extent.json`. Mipped output must not be enabled or qualified from this handoff;
> it needs an independent per-level address contract and a test that distinguishes misplaced mips — see this campaign's successor handoff.
>
> **Measurement defect found 2026-09-22 (research R66, while building the successor probe).** `r60_rw4xb2_slot_data_extent.py` reads the
> pixel slot as `data[e['off'] : e['off'] + e['bytes']]` from the **file start**, but a pixel-data entry's offset is relative to the end
> of the RX2 header (`hdr_size`, u32 BE at `0x44`, 0x1000 on every file checked). Every window r60 measured therefore began `hdr_size`
> bytes **before** the slot (inside the header and the descriptors) and ended `hdr_size` bytes before the slot's end. The §1 / §3
> extents are consequently **not measurements of the slot** and must not be read as such — the JSON is retained as the record of what
> ran; the successor probe reads the slot at `hdr_size + off`. r42 / r44 read descriptors, which are header-resident, and are unaffected.

Research lane, 21 September 2026 pm, roadmap step 1.3 / 2.5 / 4.x (X360 resize bounds; the open item of handoff 02 §3, "the packed-mip
tail is not modelled"). Claim `research-r60-rw4xb2-slot-extent`. Read-only over the loose `rendering/**/*.rx2` trees of NHL 09 / 10 / 13 /
Legacy (X360; 11 105 / 9 256 / 14 086 / 16 260 files, 17 680 / 16 482 / 25 934 / 29 171 descriptors); nothing game-verified.

Machine-readable: [`data/r60-rw4xb2-slot-data-extent.json`](data/r60-rw4xb2-slot-data-extent.json) (per title: tightness counts, model
match counts, the 150 most frequent shapes with their slot and 4 KiB-rounded extent distributions, 60 mismatch examples). Probe
`r60_rw4xb2_slot_data_extent.py` (≈ 100–140 s per title; `--titles`, `--max`).

## Plain-language summary

For every Xbox 360 texture the file table says how many bytes its pixel slot holds. This handoff measured how far the *content* actually
reaches inside each slot (the offset of the last non-zero byte). The slot is always at least as large as the content, it is always a
multiple of 8 KiB, and it equals the content rounded up to 8 KiB on 52–63 % of descriptors; the rest carry one or two spare 4 KiB pages.
So the developer’s resize rule — a new chain must fit the existing slot, or the slot must be regrown — is safe as written, and the
per-shape slot oracle of handoff 02 stays the length a writer should allocate. The second question, the exact byte layout of the small
mip levels (the “packed tail”), is **bounded, not solved**: the base level and every sub-level wider or taller than 16 blocks occupy
their 32 × 32-block tile (handoff 02’s `DW12` rule), and the levels of 16 blocks or less occupy a measured number of 4 KiB pages after
that (6 pages for an 8-byte-block 16 × 16 tail, 10–12 for a 16-byte one, 4 for 16 × 8, …). None of the simple models — every sub-level
its own tile, or 4 KiB-aligned, with a single packed tail — reproduces those extents, so a writer that must place small mips
byte-for-byte as EA does still needs the Xenos tiled-address model (developer implementation, outside this lane). A writer that only
needs the game to *accept* the texture can keep the slot length from handoff 02 and fill the tail region once that model exists.

## 1. Tightness (every descriptor with a modelled format, 2D + cube)

| Title | slot vs 4 KiB-rounded content extent |
|---|---|
| NHL 09 | 11758 / 17680 equal, 5922 larger, 0 smaller; 17680 / 17680 are 8 KiB multiples |
| NHL 10 | 10054 / 16482 equal, 6428 larger, 0 smaller; 16482 / 16482 are 8 KiB multiples |
| NHL 13 | 14463 / 25934 equal, 11471 larger, 0 smaller; 25934 / 25934 are 8 KiB multiples |
| NHL Legacy | 16196 / 29171 equal, 12975 larger, 0 smaller; 29171 / 29171 are 8 KiB multiples |

`slot_lt_extent4k` = 0 everywhere: **no shipped slot is smaller than its content**, so keeping the slot length never truncates.

## 2. Chain-length models vs the content extent (eq / over / under)

| Model | NHL 09 | NHL 10 | NHL 13 | Legacy |
|---|---|---|---|---|
| `M_tile` — every level its 32 × 32-block tile | 806 eq / 16874 over / 0 under | 784 eq / 15698 over / 0 under | 1622 eq / 24080 over / 232 under | 2086 eq / 26840 over / 245 under |
| `M_tile_pk16_tile` — as above, levels ≤ 16 blocks packed once into one tile | 1065 eq / 230 over / 16385 under | 978 eq / 231 over / 15273 under | 2078 eq / 475 over / 23381 under | 2567 eq / 745 over / 25859 under |
| `M_4k_nopack` — sub-levels 4 KiB-aligned tiles | 806 eq / 16874 over / 0 under | 784 eq / 15698 over / 0 under | 1622 eq / 24080 over / 232 under | 2086 eq / 26840 over / 245 under |
| `M_4k_pk16` — 4 KiB-aligned, ≤ 16 blocks packed into one 4 KiB page | 812 eq / 224 over / 16644 under | 791 eq / 224 over / 15467 under | 1629 eq / 462 over / 23843 under | 2094 eq / 731 over / 26346 under |
| `M_4k_pk4` — 4 KiB-aligned, ≤ 4 blocks packed | 1331 eq / 2856 over / 13493 under | 1347 eq / 1698 over / 13437 under | 2516 eq / 7051 over / 16367 under | 3044 eq / 8201 over / 17926 under |

The “eq” column is essentially the single-level population (extent = base tile). No model matches the mipped population: the tail is
larger than one tile or one page and smaller than one tile per level. Recorded as rejected candidates.

## 3. Measured packed-tail extent (2D, mipped): pages of 4 KiB after the base tile and the tiles of every sub-level > 16 blocks

The key is bytes per block (4 = `8_8_8_8`, 8 = DXT1 / CTX1, 16 = DXT3 / DXT5 / DXN) and the first sub-level at or below 16 blocks (its
block dimensions, and how many levels remain from it). Counts are descriptors over the four titles. Smaller page counts belong to
textures whose smallest levels are zero — an extent is a *lower* bound on the region — so the **max** is the value to allocate.

| block bytes | first packed sub-level (blocks w × h, levels left) | tail pages observed (count) | max |
|---|---|---|---|
| 4 | 16 × 16, 5 | 1 (2), 3 (2768) | **3** |
| 8 | 1 × 1, 2 | 0 (103) | **0** |
| 8 | 2 × 16, 7 | 0 (20) | **0** |
| 8 | 4 × 4, 5 | 2 (85) | **2** |
| 8 | 4 × 8, 6 | 2 (209) | **2** |
| 8 | 4 × 16, 7 | 1 (10), 2 (892) | **2** |
| 8 | 8 × 8, 6 | 2 (1), 3 (16), 4 (239) | **4** |
| 8 | 8 × 16, 7 | 1 (10), 2 (31), 3 (153), 4 (2492) | **4** |
| 8 | 16 × 2, 7 | 0 (216) | **0** |
| 8 | 16 × 4, 7 | 1 (2), 2 (141) | **2** |
| 8 | 16 × 8, 7 | 1 (8), 2 (23), 3 (135), 4 (5178) | **4** |
| 8 | 16 × 16, 7 | 3 (10), 4 (39), 5 (148), 6 (31828) | **6** |
| 16 | 4 × 4, 5 | 2 (9), 4 (6) | **4** |
| 16 | 4 × 16, 7 | 4 (47) | **4** |
| 16 | 8 × 8, 6 | 5 (2), 6 (15), 7 (8), 8 (26) | **8** |
| 16 | 8 × 16, 7 | 7 (8), 8 (507) | **8** |
| 16 | 16 × 4, 7 | 2 (2), 3 (136), 4 (91) | **4** |
| 16 | 16 × 8, 6 | 7 (7252) | **7** |
| 16 | 16 × 8, 7 | 7 (705), 8 (60) | **8** |
| 16 | 16 × 16, 7 | 9 (6), 10 (26815), 11 (66), 12 (1502) | **12** |

Observations: (a) the page counts are identical across the four titles — one layout from 09 to Legacy; (b) the tail grows with the block
size (8 B 16 × 16 → 6 pages, 16 B → 10–12) and shrinks with the first packed level (16 × 8 → 4 / 7–8, 8 × 8 → 4 / 8, 16 × 4 → 2 / 3–4,
4 × 4 → 2 / 2–4); (c) very wide or tall tails (16 × 2, 2 × 16, 1-block levels) add **no** pages — their content fits in the slack of the
preceding tile; (d) 369 descriptors have an extent *below* base + large-level tiles — placeholder textures whose data is zero past the
first bytes, and the 32 × 1 / 1 × 32-block strips whose levels do not fill a 32 × 32 tile — recorded, not modelled.

## 4. Contract input for the developer (resize / chain writer)

- Slot length: keep handoff 02’s per-shape slot oracle — this handoff confirms it is never below the content and is 8 KiB-granular. For a
  shape that is not shipped, `8 KiB-roundup(base tile + Σ large-level tiles + max tail pages × 4 KiB)` from §3 is a **candidate**, not
  verified in-game.
- Tail placement: not derived here. Implement the Xenos tiled address for the sub-levels (developer implementation) and check it against
  the §3 extents and the mismatch examples in the JSON. Until then, refuse mipped textures whose shape is not in the oracle, or write
  single-level textures only — the single-level population matches `M_tile` exactly.
- Refusal conditions are unchanged from handoff 02 §5.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r60_rw4xb2_slot_data_extent.py docs/research/overnight-2026-09-21-pm/data/r60-rw4xb2-slot-data-extent.json --titles nhl09,nhl10,nhl13,legacy   (≈ 8 min)
```
Executed: two trial runs on NHL 13 (`--max 400` / `--max 1500`, into the scratchpad, deleted; the second after correcting the tile unit
from texels to blocks), then one final run; three shell analyses of the JSON (the §3 tabulation) and one 512-byte non-zero map of five
NHL 13 slots (not retained). Inputs: the X360 loose `rendering` trees (paths in `r42_rw4xb2_slot_join.py`); no per-file hashes (a 50 k-file
census).

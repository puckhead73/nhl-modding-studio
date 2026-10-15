# Handoff R11 — localization: the Huffman tree pinned for a canonical rebuild, the 175-dictionary order/alphabet census, the PS4 locale inventory, and the NHL 07 label search closed

Research lane, 20 September 2026 (Priority 1C of the overnight campaign). Claim `research-r11-localization`. Consumer: developer
**P1-D / LOC-W3 O-G** (canonical rebuild with new glyphs — "not impl" in the matrix), 1.3 locale rows ("PS4 locale members not
extracted"), the NHL 07 1.1/1.3 cells. Builds on handoffs [20](../overnight-2026-09-18/20-dictionary-growth-invariants.md) (B1–B8, the
C serializer proofs) and [05](../overnight-2026-09-19/05-phase1-targeted-gaps.md) (R5 bounded search). Nothing here is game-verified;
nothing written under `X:`/`H:`/`E:\nhl-studio-fixtures`/`anchors/`; no launch. **No writer is proposed**; §5 is a contract proposal.

Machine-readable: [`data/r11-loc-tree-census.json`](data/r11-loc-tree-census.json) (175 dictionaries: identity, rows, tree bytes / leaves /
max code length / Kraft sum, alphabet-vs-usage, coded-bit cost vs optimal and vs length-limited optimal, the eight rebuild policies'
results, record-order runs, duplicates, nulls), [`data/r11-nhl07-label-grep-large.json`](data/r11-nhl07-label-grep-large.json) (the 39
files > 8 MB that R5 skipped, all scanned, 0 hits). Probes: `loc_tree_rebuild.py`, `nhl07_label_grep_large.py`.

**Evidence vocabulary:** **observed** / **inferred** / **unknown** as in the earlier handoffs.

## Plain-language summary

To add a symbol the dictionary's compression tree does not have (an accented letter, a new punctuation mark) the whole string pool
must be rebuilt with a new tree, and until now nobody had pinned how EA writes that tree or whether a rebuilt one would look like
EA's. This handoff pins the tree's byte layout exactly (re-serialising every parsed tree reproduces the original bytes on all 175
dictionaries), shows that EA's tree always covers exactly the symbols the strings use (never a spare leaf), that it is a valid full
prefix tree of depth 10–12, and that it is *not* the textbook Huffman tree for its own strings (0.1–4.3 % more bits than optimal, and
none of eight standard constructions or nine weight transforms reproduces it). So a rebuilt dictionary will carry a *different* tree —
that is unavoidable, and whether the game accepts a tree it did not write is a runtime question, stated as such. The census also gives the
developer the full order/duplicate/null picture of all 175 files (the out-of-order files are concatenations of 3–36 sorted blocks, on
NHL 08–10 and the `lng_lg` template only), the complete PS4 locale inventory (7 titles × 7 locales + NHL 21 `lng_lg`, same row count in
every locale of a title, one duplicate hash in every PS4 file, 30–591 empty texts per non-English file), and closes the NHL 07 label
question's size gap: the 39 files larger than 8 MB contain neither the keys nor any candidate label text.

## 0. Sources (observed)

25 committed anchors (`anchors/{xbox360,ps3,ps4}/*/databases/*eng_us.db`; the 07/08/09 "ps3" anchors are Xbox files by identity, known),
100 private gen7 loose locale files (`E:\nhl-studio-fixtures\loc\<platform>-<title>\*.db`, manifest hashes), 50 PS4 locale files from the
extracted trees (`H:\…\CUSA*\databigs\Extracted\nhl\data\kettle\loc\*.db`; SHA-256 in the JSON). 175 files, 0 parse errors.

## 1. The tree, pinned (observed on 175 / 175)

- **Serialisation.** The pool begins with one 4-byte quad per internal node, in breadth-first order from the root (node 0): `left
  marker, left value, right marker, right value`. A leaf child is `00 <symbol>`; an internal child is `<node number> 00`, where the
  node number is the child's position in the breadth-first queue and **the right internal child is enqueued before the left** (so the
  root's quad reads e.g. `02 00 01 00`). Re-serialising the parsed tree under this rule reproduces the original tree bytes on
  **175 / 175** files (`node_number_rule_ok`). Consequence: internal nodes ≤ 255 (marker 0 is reserved for leaves), i.e. ≤ 256 leaves.
- **Alphabet = usage.** The leaf set equals the set of byte symbols that occur in the reachable keys and texts — 0 unused leaves and
  0 used-without-leaf on 175 / 175. A new symbol therefore always needs a new tree (handoff 20's O-G is confirmed as the only path).
- **Shape.** Kraft sum exactly 1 (full binary tree) on 175 / 175; leaves 96–171; tree 380–680 B; **maximum code length 10 (18 files),
  11 (122) or 12 (35)** — never more, although the unconstrained optimum for the same histograms would reach 17–21.
- **Not textbook Huffman.** Coded cost vs the optimal Huffman for the reachable strings' histogram: ratio 1.0012–1.0434 (mean 1.0088),
  0 / 175 equal. Against the optimal *length-limited* (L = 11, package-merge) code: 14 / 175 files have exactly EA's lengths (NHL 07,
  NHL 11 eng, a few gen7 locales, 3 PS4 files); the rest are slightly *worse* than the limited optimum. Reconstruction attempts:
  eight standard-Huffman tie-break policies (fifo / lifo / symbol-ascending / -descending × left-first / -second, histogram of used
  symbols) → **0 / 175 byte-equal, 0 / 175 code-length-equal**; nine weight transforms (saturate / wrap at 16 and 15 bits, √, log₂,
  ÷16, ÷256) → none reproduces the lengths on the five files tried. **Inference:** EA builds a depth-limited tree with a heuristic of
  its own (the depth bound and the "exactly the used alphabet" rule are the observable constraints); the exact algorithm is unknown and
  two distinct approaches failed — recorded as an obstacle, not pursued further.

## 2. Record order, duplicates, nulls (observed; extends handoff 20 B2/B4/B5 to 175 files)

- **Descents** only in NHL 08 / 09 / 10 (every locale measured) and the `lng_lg` template (Xbox 12–Legacy: 22 descents, PS3 12–Legacy:
  33, identical across titles). The files are concatenations of sorted runs: NHL 08 eng 3 runs (2 829 / 618 / 2 291), NHL 09 eng 3
  (666 / 8 169 / 613), NHL 09 rus 36, NHL 10 eng 8–9, NHL 10 fre 9–11, `lng_lg` 23 / 34. **Inference:** the authoring tool concatenated
  separately sorted blocks (per-source tables) without a global re-sort; whether the runtime lookup is a global binary search (which
  would then miss keys in later blocks), a per-block search or a hash map is **unknown** — the census says only that such files
  shipped. Handoff 20's rule (never reorder, refuse O-K on class B) stands; the run structure is the input a later runtime card needs
  (insert a new key into the *last* run in hash order and observe, vs. into the first).
- **Duplicate hash:** exactly one per PS4 file (all 55 = 50 locales + 5 anchors; the `TXT_STATS_RANK` pair), 0 on all 120 gen7 files.
- **Nulls:** 172 / 175 files have null texts (0 null keys anywhere); the three without are NHL 07 (anchor ×2, private eng). PS4
  non-English files carry many more nulls than English (NHL 18: 49–50 vs 35; NHL 21: 590–591 vs 37; `lng_lg` 36) — **inference:**
  untranslated strings are shipped as null and fall back at runtime (unobserved).
- `max_records == current_records` on 175 / 175 (no spare rows anywhere; growth = the LOC-W3 preserving serializer).

## 3. PS4 locale inventory (observed; closes the 1.3 "research unknown" cell at the file level)

| Title | Locales (`nhl/data/kettle/loc/<loc>.db` + `-meta.xml`) | Rows (every locale) | Tree bytes / leaves (eng → rus) | Notes |
|---|---|---|---|---|
| 15 | cze_cz, eng_us, fin_fi, fre_fr, ger_de, rus_ru, swe_se | 24 251 | 616 / 155 → 656 / 165 | anchor `eng_us.db` = the extracted eng_us (same SHA; also 16 / 17 / 18) |
| 16 | same 7 | 27 468 | 620 / 156 → 660 / 166 | |
| 17 | same 7 | 30 799 | 632 / 159 → 668 / 168 | |
| 18 | same 7 | 33 396 | 636 / 160 → 668 / 168 | |
| 19 | same 7 | 37 734 | 640 / 161 → 668 / 168 | |
| 20 | same 7 | 40 535 | 640 / 161 → 668 / 168 | real install; the `nhl20` anchor equals the extracted **NHL 19** eng_us (SHA), not this file |
| 21 | same 7 + **lng_lg** | 55 766 | 640 / 161 → 668 / 168; lng_lg 668 / 168 | |

Every locale of a title has the same row count (same key set — inferred from the count, spot-checked on the duplicate pair only), its
own tree and its own pool; 0 descents; the same code path Studio already uses for `eng_us` applies (handoff 20 B1 layout, LE). Which
locale the console loads, and whether a null text falls back to English, are runtime questions. The gen7 side (100 private files) is
unchanged from the D5 sweep: NHL 09–Legacy 6 locales + `lng_lg`; NHL 12–Legacy `lng_lg` is a fixed 12 431-row (PS3) / 12 363-row
(Xbox) template identical across those titles.

## 4. NHL 07 league labels — coverage gap closed (observed)

R5 (handoff 05) had bounded its "no text source" result to members ≤ 8 MB. The 39 files above that bound (32 archive members: 31
`rendering/stadium/stadium_*_container_0.rx2` + `gamedata/anim/anim.viv`; 7 loose files: the three `.vp6` movies, `nis.big`, `nis.viv`,
`anim.viv`, `03_pressstartanimating.big`; 642 MB in all, 33 RefPack-wrapped and decoded) were scanned with R5's patterns (the
`CHTM_LEAGUE_*` keys and 14 candidate label strings, ASCII and UTF-16, nested BIGF one level): **0 hits**. The remaining exclusions of the
combined search are now: nesting deeper than two levels, compression other than RefPack, and encodings other than ASCII / UTF-16 —
not size. The conclusion of handoff 05 stands with that narrower bound: the three European league names have no located text source;
display behaviour stays card R5-07-1.

## 5. Contract proposal for the canonical rebuild (O-G) — policy, not evidence

Inputs: the dictionary (class A or B per handoff 20), the plan (edits / inserts) whose strings need symbols outside the current
alphabet. Build: the new alphabet = symbols used by every reachable string after the plan (exactly, no spare leaves — matching EA);
construct a **full binary prefix tree with maximum code length ≤ 12** (the observed ceiling; use a length-limited construction such
as package-merge with L = 12 — it yields a valid tree at least as compact as EA's; any tree meeting the two constraints is structurally
valid, and byte-identity with an EA tree is **not** a reachable target); serialise it with the §1 rule; re-encode every string; lay the
pool out per handoff 20 §4-C. Validation: re-parse; every row decodes to the requested `(key, hash, text|null)`; `node_number_rule_ok`
on the produced tree; max depth ≤ 12; leaves = used alphabet; Kraft = 1; the C1/C2 accounting of handoff 20 (every byte difference
named). Refusals: alphabet > 256 symbols; any string whose new encoding exceeds the capacities; class B + O-K (unchanged). Runtime:
**card R11-LOC-1** — a rebuilt tree on the NHL 21 PS4 `eng_us.db` copy with one added glyph in one visible string: rendered glyph and
all other strings intact → the loader accepts a non-EA tree of depth ≤ 12 (this title); garbled/blank text → tree rejected or the
glyph is absent from the font (separate: check a string using only existing glyphs in the same rebuilt file first — that isolates
tree acceptance from font coverage). Dictionary encoding capacity, font glyph availability and rendering are three separate
questions; this handoff settles only the first.

## 6. Unknowns (explicit)

1. EA's exact tree construction (heuristic depth limit). 2. Loader acceptance of a rebuilt tree / of a grown file (R-A, R-B of
handoff 20) / of unsorted rows (the run structure). 3. Font glyph coverage per title and locale (not inspected). 4. Which PS4 locale
the console loads and null-text fallback. 5. NHL 07 labels beyond the stated exclusions.

## 7. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-20/probes/loc_tree_rebuild.py docs/research/overnight-2026-09-20/data/r11-loc-tree-census.json [--only anchors|private|ps4]   (≈ 4 min)
python local/runs/overnight-research-2026-09-20/probes/nhl07_label_grep_large.py docs/research/overnight-2026-09-20/data/r11-nhl07-label-grep-large.json   (≈ 3 min)
```

Executed (read-only): the census over 175 files; the large-member grep (39 files); scratchpad checks — tree-hash comparison across
anchors (22 distinct trees; the 07/08/09 pairs identical as expected), five candidate corpora and nine weight transforms on 3–5
files, the length-limited optimum on all files. **Not executed:** writes, Studio binaries, launches. Scratchpad `r11-anchors.json`,
`r11.log` deleted at closeout.

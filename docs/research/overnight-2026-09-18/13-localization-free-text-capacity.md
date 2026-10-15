# Handoff P1.3-X — what a free-text / key-insert dictionary writer has to deal with (measured capacity)

Research lane, overnight 2026-09-18. Claim `p1.3-localization-capacity`. Consumer: developer Phase 1.3 / 2.3
(the free-text writer LOC-W1 deferred). Probe: `probes/loc_capacity.py` (all ten Xbox 360 `nhl_eng_us.db` anchors
and the PS4 `eng_us.db` anchors). Evidence: **observed**.

| Dictionary | rows (current / max) | spare rows | pool bytes | tree bytes | symbols in tree | printable ASCII **absent** from the tree | highest referenced pool offset |
|---|---|---|---|---|---|---|---|
| Xbox NHL 07 | 3 952 / 3 952 | **0** | 119 144 | 380 | 96 | `* ; @ [ \ ] ^ \` { \| } ~` | 119 116 |
| Xbox NHL 08 | 5 738 / 5 738 | 0 | 175 136 | 416 | 105 | `* ; \ ^ { \| } ~` | 175 123 |
| Xbox NHL 09 | 9 448 / 9 448 | 0 | 362 272 | 660 | 166 | `^ \| ~` | 362 216 |
| Xbox NHL 10 | 12 505 / 12 505 | 0 | 467 792 | 560 | 141 | **`\` a b c e f g h i j k l m n o p q r t u v w x y z \| ~`** (all-caps game) | 467 742 |
| Xbox NHL 11 | 14 920 / 14 920 | 0 | 572 520 | 636 | 160 | `` ` j q v x z \| ~ `` | 572 464 |
| Xbox NHL 12 | 16 787 / 16 787 | 0 | 668 064 | 632 | 159 | `` ` h j q v x z \| ~ `` | 668 012 |
| Xbox NHL 13 / 14 / 15 / Legacy | 18 826 … 25 782 (= max) | 0 | 851 704 … 1 346 408 | 660 | 166 | `` ` \| ~ `` | pool end − ~70 |
| PS4 NHL 15 … 20 | 24 251 … 37 734 (= max) | 0 | 1 258 128 … 1 888 560 | 616–640 | 155–161 | `^ ~` (+ `` ` `` / `\|` on 15/16) | pool end − ~30 |

Consequences for the writer design:

1. **No spare record slots anywhere** (`max_records == current_records` on every dictionary). Adding a key
   means growing the record block by 16 bytes, which shifts the pool start (the pool begins at
   `record_block + max_records × 16`), so *every* pool pointer stays valid only because pointers are
   pool-relative — but the file length, `db_size`, the table's `max/current_records`, the EOF CRC and the
   header CRC all change, and on packed titles the EB member length changes (S2-A same-length rule breaks;
   needs the re-plan path).
2. **Pool is packed to the end** (highest referenced offset within ~70 bytes of the pool end on gen7b): new
   text must be appended (grows the file) or must fit in the space of the text it replaces (in-place
   re-encode only when the new bit-length ≤ old, padding bits are irrelevant since strings are
   length-prefixed by *decoded* length).
3. **Huffman tree reuse limits the alphabet**: any character absent from the tree cannot be encoded without
   rebuilding the tree (= re-encoding every string, a full pool rewrite). NHL 10's tree has **no lowercase
   letters except `d` and `s`** — free text there must be upper-case; NHL 11/12 lack `j q v x z` (12: also `h`);
   all titles lack `` ` | ~ `` and most lack `^`. A writer must validate the requested text against the
   title's tree and report the exact unsupported characters.
4. Hash uniqueness: `hashid` is `loc_hash(key)` (case-folded CRC); inserting a key must check for a hash
   collision against all existing rows (the game looks up by hash).

So the cheapest useful free-text operation is **in-place re-encode of an existing key's text with the existing
tree when the new bit length fits** (no size change anywhere — same-length member replacement keeps working);
the general case (longer text, new keys, missing glyphs) requires the growth path. Both are Studio work; no
further research needed except the runtime check of whether the game tolerates a grown dictionary member.

## 2. Coverage per source-verified family (R1 follow-on, 2026-09-18)

Machine-readable: [`data/localization-locales.json`](data/localization-locales.json) (which `<lang>_<region>`
dictionaries exist per title/platform and where), [`data/localization-capacity-ps3.json`](data/localization-capacity-ps3.json)
(27 PS3 dictionaries measured from the X: disc trees), [`data/localization-capacity-xbox360-locales.json`](data/localization-capacity-xbox360-locales.json)
(43 non-English Xbox dictionaries) and [`data/localization-capacity-ps4-extra.json`](data/localization-capacity-ps4-extra.json)
(NHL 21 five locales, NHL 15/19 base vs patch — extracted from the H: installs, measured, deleted). Probes:
`loc_locales.py`, `loc_capacity_files.py`, `loc_capacity_ps3.spec.py`. Evidence: **observed**.

**Locales (what a "translate every locale" operation has to touch):**

| Family | Locales | Where |
|---|---|---|
| Xbox NHL 07, 08 | `eng_us` only | loose `fe/loc/` (07/08 also inside `data1.big`) |
| Xbox NHL 09 | `eng_us fre_fr ger_de rus_ru cze_cz fin_fi swe_se` (7) | loose |
| Xbox NHL 10, 11 | `eng_us fre_fr` (2) | loose (11: `fe.big`) |
| Xbox NHL 12 – Legacy (and Recompiled) | 7 + `lng_lg` (8) | loose in the extraction trees; on disc in `cache.big` (12/13) / `cacheboot.big` (14+) |
| PS3 NHL 08 | `eng_us` | loose |
| PS3 NHL 09, 10, 11 | `eng_us fre_fr` | loose |
| PS3 NHL 12 – Legacy | 8 (same set as Xbox) | loose **and** `cache.big` (12/13) / `cacheboot.big` (14+) |
| PS4 NHL 15 – 20 | 7 (`cze eng fin fre ger rus swe`) | `inxp_00.big:nhl/data/kettle/loc/`; NHL 15 and NHL 19 ship **patched copies** in `patch_inxp*.big` |
| PS4 NHL 21 | 7 + `lng_lg` | `inxp_00.big` |

`lng_lg.db` is a 12 363-row, all-caps "language template" dictionary byte-identical across Xbox 12–Legacy
(pool 642 152 B, slack 153); it is not a player-facing locale.

**Capacity and alphabets on the previously unmeasured targets** (all: `spare_rows == 0`, pool slack ≤ 157 B):

| Target | rows | pool | tree symbols | printable ASCII absent | lowercase present |
|---|---|---|---|---|---|
| PS3 NHL 08 `eng_us` | 5 718 (Xbox: 5 738) | 173 624 | 104 | `* ; \ ^ { \| } ~` | all |
| PS3 NHL 09 `eng_us` / `fre_fr` | 9 434 (Xbox 9 448) | 360 256 / 390 040 | 166 / 163 | `^ \| ~` / `& = ^ \` \| ~` | all |
| PS3 NHL 10 `eng_us` / `fre_fr` | 12 559 (Xbox 12 505) | 469 712 / 509 936 | 152 / 150 | no `b c f g h j m p q u v x z` (PS3 keeps `a d e i k l n o r s t w y` — **differs from Xbox NHL 10, which keeps only `d s`**) | partial |
| PS3 NHL 11 `eng_us` / `fre_fr` | 14 630 (Xbox 14 920) | 566 144 / 617 648 | 158 / 157 | `` ` j q v x z { \| } ~ `` / `# & = \` f x z { \| } ~` | partial |
| PS3 NHL 12 `eng/fre/ger/rus` | 16 409 (Xbox 16 787) | 658 896 … 1 021 776 | 158–159 | eng `` ` h j q v x z \| ~ ``; ger `# \` b j q x z \| ~`; rus `# & \` b f g h j p q v x z \| ~` | partial |
| PS3 NHL 13 / 14 / 15 / Legacy, all four | 18 362 / 24 247 / 25 170 / 25 783 | up to 1 948 696 (rus) | 166 (rus 171) | `` ` \| ~ `` (fre 13: + `# =`) | all |
| Xbox NHL 12 non-English | 16 787 | 696 704 … 1 032 616 | 145–158 | per locale, e.g. fre `# & = \` f m x z \| ~`, cze `` ` f j q x \| ~ `` | partial |
| Xbox NHL 13 – Legacy non-English | = eng rows | up to 1 949 480 | 152–171 | `` ` \| ~ `` (13 fre `# =`, 13 fin `#`) | all |
| PS4 NHL 21 `eng/fre/cze` / `rus/lng` | 55 766 | 3 028 680 … 4 260 272 | 161 / 168 | `^ ~` | all |
| PS4 NHL 15 base → patch `eng_us` | 24 251 → **24 443** | 1 258 128 → 1 267 696 | 155 | `^ \` \| ~` | all |
| PS4 NHL 19 base → patch 56 `eng_us` | 37 734 → **38 001** | 1 888 560 → 1 899 232 | 161 | `^ ~` | all |

Findings that change the writer's assumptions:

1. **PS3 and Xbox dictionaries of the same title are different files** (row counts differ on every title
   08–Legacy: 5 718/5 738, 9 434/9 448, 12 559/12 505, 14 630/14 920, 16 409/16 787, 18 362/18 826,
   24 247/24 736, 25 170/25 169, 25 783/25 782) with different Huffman trees — so alphabet validation and
   capacity must be read from the target's own dictionary, never from the sibling platform's anchor. This
   makes explicit the caveat that the PS3 08/09 dictionary *anchors* are Xbox files.
2. **Alphabets are per locale, not per title**: French trees lack `& =` on 09–13, Russian trees carry 76
   non-ASCII symbols, Czech trees are the smallest (145–152 symbols). A per-locale write must validate
   against that locale's tree.
3. **EA's own patches grow the dictionary**: PS4 NHL 15 (+192 rows, +9.5 KB pool) and NHL 19 (+267 rows,
   +10.7 KB) ship a larger `eng_us.db` in a patch archive. This is direct evidence that the runtime accepts a
   dictionary member with a different row count and size than the base — the growth path is a supported
   game behaviour, not a speculative one (on PS4; the gen7 engines were not observed doing it).

**Fixed allocation vs growth (clarified per R1):** what is *fixed* is the shipped allocation — zero spare
records and a pool packed to within ≤ 157 bytes of its end on every measured dictionary (79 files across
three platforms). That constrains only the **in-place** operation (same-length member: re-encode an existing
key's text with the existing tree when the new bit length fits). It does **not** mean no safe growth
algorithm exists: the dictionary is a single-table TDB whose record block and pool are re-emittable from the
decoded rows (Studio's LOC-W1 re-point writer already rewrites the file), so "append N rows, rebuild the
tree if new glyphs are needed, re-encode the pool, recompute sizes/CRCs" is a well-defined transformation;
its only open question is runtime acceptance of a larger member on each family (proved for PS4 by EA's
patches; a runtime card for gen7/Recompiled, where the loose `db/`-style member is also the disc member in
`cache*/cacheboot.big` and the EB re-plan path applies).

Unmeasured after this pass: Xbox NHL 07 `fe/loc` inside `data1.big` (same bytes as loose per R0), PS4 NHL
16/17/18/20 non-English locales (same tree family expected), and any language beyond the eight listed.

# Handoff L-G — dictionary growth contract (LOC-W3): baseline-preserving invariants, operations, tests, gates

Research lane, 18 September 2026. **Revision 3** (coordinator review of revision 2,
[`loc-w3-revision2-review.md`](../../validation/loc-w3-revision2-review.md)) — applies the five document
corrections: class A is *nondecreasing* (equal-hash runs allowed), the preserving serializer appends after
the physical pool end with explicit offset/size/CRC accounting, the canonical rebuild is proved by decoded
equality plus individually accounted structural differences (no universal byte-identity), baseline-relative
tests exclude exactly the requested changes, and the null-pointer claim carries the NHL 07 exception. No new
measurement was taken: everything below reuses the 95-file census and the six-file invariants survey.
Consumer: developer LOC-W3, after LOC-W1 (`repoint`) and LOC-W2 (`retext`). Machine-readable:
[`data/localization-order-census.json`](data/localization-order-census.json) (95 files: hash-order descents
with row numbers, duplicate hashes, null text pointers, shared pointers, reserved-word values),
[`data/localization-invariants.json`](data/localization-invariants.json) (six files incl. PS4 NHL 21 install:
packing/pointer detail), plus the R1 coverage data. Probes: `loc_order_census.py`, `loc_invariants.py`.
Format reference: `crates/tdb-loc/src/lib.rs` and `tdb_core::append_record` (accurate; nothing here
contradicts them). No application code was changed; no game bytes stored.

Evidence levels used below: **measured** (read from the census/invariants files), **Studio** (behaviour of
the existing `tdb-loc`/`tdb-core` writers, read from their docs), **unknown** (loader behaviour — needs a run).

## Plain-language summary

A grown dictionary is the same file with more rows and a longer string pool. The contract says which
operations are allowed, what bytes may change for each, and how to check the result against the input rather
than against an idealised layout — because EA's own files are not perfectly regular (a few titles have rows
out of hash order, the PS4 titles carry one duplicate hash, nearly every file has empty texts) and because
Studio's own earlier writers deliberately produce files with shared string pointers. The writer must keep all
of that exactly as found, add only what was asked, account for every changed byte, and hand the game-side
questions (does the loader accept a bigger file? does it find a *new* key?) to two separate runtime tests.

## 1. Baseline facts (measured)

Census composition (95 files, `localization-order-census.json`): 25 anchors (10 Xbox 360, 10 PS3, 5 PS4:
NHL 15/16/17/18 and the NHL 19-base `nhl20` anchor), 27 PS3 loose locale files (08–Legacy), 43 Xbox 360 loose
locale files (09–Legacy incl. `lng_lg`). PS4 NHL 21 (install) is measured in `localization-invariants.json`
(order, duplicates, sharing) and the PS4 install/patch extras (NHL 15 base/patch, NHL 19 base/patch 1.56,
NHL 21 five locales) in the capacity survey only (rows/pool/tree — no order or null measurement). Statements
below are about these files; they are not assertions about unmeasured titles or locales.

| # | Fact | Where it holds / exceptions |
|---|---|---|
| B1 | One table, three fields, **16-byte records**: gen7 `key ptr @0, text ptr @4, hashid @8` (BE), PS4 `text ptr @0, hashid @4, key ptr @8` (LE); bytes 12–15 are `00 00 00 00` in every record of every file | all 95 |
| B2 | Rows are in **nondecreasing unsigned `hashid` order** (no descents; equal adjacent hashes occur, see B4) | Xbox/PS3 NHL 07, 11, 12, 13, 14, 15, Legacy (every measured locale except `lng_lg`), PS4 15–18, 19-base anchor, NHL 21 install: 0 descents (73 of 95 census files + NHL 21). **With descents:** Xbox/PS3 NHL 08 anchors (2 each), NHL 09 (anchors 2; eng 2, fre 3–8, ger 2, rus 35, cze 3, fin 5, swe 2), NHL 10 (7–10 per locale), and the `lng_lg` template (22, identical on Xbox 12–Legacy) — 22 census files. Descent row numbers are in the census JSON. |
| B3 | `hashid == loc_hash(key)` where `loc_hash` folds UTF-16 code units with `0xDF` | every gen7 row; PS4 rows whose keys contain non-ASCII (NBSP, ZWSP, en dash, curly quote — 4 on NHL 21) hash from the UTF-16 form of the UTF-8 key |
| B4 | **Duplicate hashes exist in shipped data**: `' TXT_STATS_RANK'` / `'TXT_STATS_RANK'` (adjacent rows, hash `0x59D432E3`), one pair per file | the five PS4 census anchors (all with 0 descents) and PS4 NHL 21 install (invariants file); 0 on all 90 gen7 census files |
| B5 | **Null text pointers (`0xFFFFFFFF`)** are present in 93 of the 95 census files: gen7 NHL 08–Legacy 13–34 per file, PS4 anchors 30–37 (`TEAMLINE1_*`-style keys with empty text). **Exception:** both NHL 07 anchor records (`xbox360-nhl07-anchor`, `ps3-nhl07-anchor`) have 0. Keys are never null | as stated; null support is required regardless of whether a given input contains one |
| B6 | No two rows share a key or text pointer in any **shipped** file | all 95 + NHL 21 — an observation about EA's authoring, **not** a precondition (see §2, Studio-touched inputs) |
| B7 | Pool = Huffman tree (380–668 B) then strings, byte-aligned and contiguous (0-byte gaps), laid out in record order — gen7 `[key][text]` per row, PS4 `[text][key]` per row; pool packed to within ≤ 157 B of its end (28–39 B on the PS4 extras); `max_records == current_records` | six files measured in detail; slack/rows on all R1 files |
| B8 | Layout: file header (`db_size @0x08`, header hash `@0x14`), one-table directory, table block (`prior_crc @+0`, `max_records @+0x14`, `current_records @+0x16`, `header_crc @+0x24`), record block, pool from `record_block + max_records × 16` to `file_len − 4`, **trailing EOF CRC** (last 4 bytes, over the table body). Pool pointers are pool-relative. Text length prefix 2 B BE (also on PS4), key prefix 1 B; capacities 100 B key / 4 000 B text decoded | tdb-core / tdb-loc, verified on anchors |

Loader behaviour implied by none of the above: the descents in B2 do **not** tell us the lookup algorithm
(binary search, hash map, linear) — only that files with a few out-of-order rows ship and work in those
titles. Treat "does the loader require sorted order" as **unknown**; the contract therefore never reorders
existing rows and does not insert into files whose order is already anomalous.

## 2. Operations (supported / refused), per input class

Input classes, decided at plan time from the bytes:

- **class A** = no descent (`hashid` nondecreasing over the record order; equal-hash runs such as the PS4
  `TXT_STATS_RANK` pair are valid members of class A and are kept intact);
- **class B** = at least one descent (NHL 08/09/10 in every measured locale, `lng_lg`);
- **Studio-touched** = output of `apply_repoint` (shared text pointers) or `apply_retext` (re-encoded strings
  with zeroed slack) — detected as such, never refused for being Studio output; classed A/B by the same rule.

| Op | What it does | Class A | Class B | Studio-touched input |
|---|---|---|---|---|
| **O-T text growth** (existing key, longer text, glyphs in tree) | appends the new string to the pool, re-points the row | supported | supported (row order untouched) | supported; if the old string is shared (LOC-W1), the caller chooses "this row only" (default — new string, other sharers keep the old one) or "all sharers" — the sharer set is reported before mutation |
| **O-K new key** (key + text, glyphs in tree) | inserts a record at the deterministic position (§3) and appends two strings | supported | **refused** with reason `baseline-order-anomalous` (insertion position undefined without knowing the lookup algorithm); the refusal names the descent rows | supported (class A only) |
| **O-G new glyph** (any op whose text needs a symbol the tree lacks) | canonical rebuild (§4-C) with a new tree | supported, flagged `tree-rebuilt` | refused for O-K (as above); supported for O-T | supported; shared pointers preserved as sharing (§4-C) |
| **O-N null text** | new row with empty text, or an existing row's text set to empty | pointer `0xFFFFFFFF` (as EA does, B5) | same | same |

Refusals (all classes): **insertion** (O-K) of a key that already exists exactly — the caller wanted O-T;
**insertion** of a key whose hash equals **any existing row's hash** (row-identity check over the full
multiset, incl. the shipped B4 duplicates — never a lookup helper that collapses duplicates; the shipped
duplicate itself is never a reason to refuse the file); key > 100 B / text > 4 000 B decoded; key with a
null byte; input fails `tdb-loc::verify`; the input's baseline descents cannot be reproduced (corrupt
order); requested placement is the same-length disc-image route (§6). Existing-key **text edits** (O-T,
O-N on an existing row) are never refused for the key existing — that is their purpose.

Existing-key **text reassignment** (LOC-W1) and **in-place re-text** (LOC-W2) remain their own operations;
LOC-W3 does not re-implement them and must accept their outputs as inputs.

## 3. Deterministic insertion policy (O-K, class A only)

Position of a new row = number of existing rows whose `hashid < new hash`. Because a new hash equal to any
existing hash is refused (§2), the position is unique and never falls inside an existing equal-hash run.
Existing rows keep their relative order (they are a subsequence of the output), so the output has no descent
and every existing equal-hash run is intact. Multiple inserts in one plan are sorted by hash first and placed
left to right; two *new* keys with equal hashes are refused.

Validation relative to the baseline (both classes, all ops): the input rows appear in the output in the same
order with the same `(key bytes, hash, text bytes-or-null)`, **except exactly** the rows the plan names — an
edited row keeps its key and hash and takes the requested text, and, for O-T "all sharers", the explicitly
selected current sharers take it too; the descent set of the output restricted to input rows equals the
input's descent set (class B: identical, since nothing is inserted); class A outputs have no descent and
their equal-hash runs are the input's runs unchanged. `loader-order-compatibility: unknown` is recorded on
every plan.

## 4. Two serializers, two proofs

**P — preserving serializer (default for O-T, O-K, O-N).** Output = the input with exactly these changes,
each reported as a byte range (k = inserted rows, s = appended string bytes):

| Region | Change |
|---|---|
| file header | `db_size @0x08` += 16k + s; header hash `@0x14` recomputed |
| directory | unchanged (single table; its block precedes the insertion point) |
| table block | `max_records @+0x14` and `current_records @+0x16` += k; `prior_crc @+0` and `header_crc @+0x24` recomputed (`prior_crc` covers the unchanged directory, so its value is unchanged in practice) |
| record block | k inserted 16-byte records at their §3 positions (B1 layout, reserved word zero); every other record byte-identical **in content** — records after an insertion point move by 16 × (inserts before them); a re-pointed row changes only its 4-byte text pointer |
| pool | the **entire original pool** — tree, every existing string, unreachable storage and the trailing slack — copied verbatim; its absolute start moves by 16k (pool start = `record_block + max_records × 16`), its pool-relative content and every existing pool pointer are unchanged; new strings appended **after the physical end of the original pool** (i.e. after the slack, at old `file_len − 4` relative to the old pool start), in the order the plan lists them (`[key][text]` gen7 / `[text][key]` PS4 for an inserted pair) |
| EOF CRC | relocated to the new last 4 bytes and recomputed over the new table body |

Nothing else. Shared pointers from LOC-W1 survive untouched; LOC-W2's zeroed slack survives untouched (the
new strings sit after it — B7's per-row placement is EA's authoring order, and no evidence says the loader
depends on it; LOC-W1 already relies on a row reading a string that is not "its" position).

**C — canonical rebuild serializer (only for O-G, or when explicitly requested).** Rebuild from the decoded
rows: new tree from the full symbol histogram (or the old tree when the alphabet is unchanged), then the pool
in B7 per-row order, byte-aligned, zero gaps, no slack; **shared pointers preserved as sharing** (a string
referenced by n rows is emitted once and all n rows point to it); null texts stay null; header words, field
descriptors, reserved record bytes and (when the tree is kept) the tree bytes are copied, never regenerated.
C changes the pool layout in general, so it is never used silently.

Proofs (both required; a no-op run of P proves only that P copies, never that C is right):

- **P1** no-op preservation: P with an empty plan → byte-identical to the input (every one of the 95 files;
  any Studio-touched fixture likewise).
- **C1** no-op semantic rebuild: C with an empty plan and the *existing* tree, on every shipped file and on
  the Studio-touched fixtures → (i) decodes to the same `(key, hash, text|null)` sequence, same row order,
  same null set, same sharing relation, `verify()` passes; (ii) opaque metadata preserved byte-for-byte:
  file header apart from `db_size`/hash, directory, table block apart from CRC words, field descriptors,
  every record's reserved word, and the tree bytes; (iii) every byte difference against the input falls into
  one of these **individually accounted** classes, each named with its range: pool layout/length (strings
  repacked in B7 order), a **specifically identified** discarded range that is either trailing slack (bytes
  after the last reachable string end) or unreachable string storage (a string no row points to — expected
  only on LOC-W1 outputs where a row was re-pointed away), the pool pointers of rows whose strings moved,
  `db_size`, and the four CRC words. No generic "the pool may differ" allowance: an unexplained byte is a
  defect. For shipped files (B6, B7 hold) the expected outcome is that the repacked pool equals the input
  pool apart from the trailing slack range, and a further difference must be explained before C is accepted.
- **C2** rebuild with a plan (O-G path, or C requested for O-T/O-K/O-N): decoded output equals the
  **requested result** — input rows with the plan's edits applied plus the plan's inserted rows at their §3
  positions, nulls per O-N, sharing per the caller's sharer choice; tree size and bit lengths may change when
  the tree is rebuilt (then the tree-byte difference is accounted); pointers, sizes and CRCs as in C1.

Changed-range validation for both serializers: the test derives the expected changed ranges from the plan
and the input on its own (record positions from §3, pool sizes from the encoded string lengths, the B8 offsets)
and compares the output against that expectation; the serializer's own accounting report is then checked
against the same independently derived set. A report produced solely by the serializer under test is not
the oracle.

## 5. Test matrix (synthetic fixtures for the repo; the 95 real files as an ignored private-fixture sweep)

| Case | Input | Expected |
|---|---|---|
| T1a | P, empty plan, every shipped dictionary | byte-identical (P1) |
| T1b | C, empty plan, existing tree, every shipped dictionary | semantically equal, every byte difference in an accounted class (C1); for shipped files only the trailing-slack range, pointers/sizes/CRCs |
| T2 | O-K one key, class A synthetic (with an equal-hash run present) and PS4 NHL 21 | row at position = count(hash <); the equal-hash run intact; +16 B in the record block; pool = original pool verbatim (slack included) + appended `[text][key]` (PS4) / `[key][text]` (gen7); all other bytes per §4-P; CRCs valid; expected ranges derived independently by the test |
| T3 | O-K keys hashing below min / above max | first / last position; no descent |
| T4 | O-K with hash equal to an existing row (incl. the PS4 `TXT_STATS_RANK` pair, and a synthetic class-A file with a three-row equal run) | refused naming the colliding record index/indices and key(s); parse and O-T on the same input succeed (the shipped duplicate never blocks the file) |
| T5 | O-K on Xbox NHL 10 (class B); O-T on the same file | O-K refused `baseline-order-anomalous` listing descent rows 183, 482, …; O-T succeeds with the record block byte-identical **except the edited row's 4-byte text pointer**, plus `db_size`, string append, EOF CRC relocation and CRC words per §4-P |
| T6 | O-T longer text, glyphs in tree | row text pointer → appended string; old string bytes untouched (still referenced by any sharer); accounting per §4-P |
| T7 | **repoint → growth**, two independent cases from the **same** `apply_repoint` baseline (rows A and B share B's text): (a) O-T on A "this row only"; (b) O-T on A "all sharers" | (a) A → new appended string, B unchanged and still pointing at the old string, sharer report emitted before mutation; (b) A and B both → one appended string, old string unreachable but preserved (P). Case (b) starts from the baseline, not from (a)'s output — after (a), B is no longer a sharer of A |
| T8 | **retext → growth**: `apply_retext` output (zeroed slack inside the pool) then O-K, under P and under C | P: slack bytes copied verbatim, new strings after the physical pool end; C: the zeroed slack is the identified discarded range, every string after it moves — the downstream pointer changes, pool length, `db_size` and CRCs are accounted (C1 rules) and decoded rows equal the requested result (C2) |
| T9 | O-N: new key with empty text; O-T setting a non-null existing text to empty; O-N on an already-null row | pointer `0xFFFFFFFF`; decode returns null; null count +1 for the new row and for the previously non-null row; the already-null edit is a no-op (count unchanged, P output byte-identical) |
| T10 | non-ASCII: key with NBSP / en dash (PS4), text with `é` on a tree that has it | hash computed over UTF-16 units of the UTF-8 key (matches the 4 shipped NHL 21 rows); encoding uses the tree leaf for the UTF-8 bytes |
| T11 | O-G: text with `~` (absent everywhere) | C path: `tree-rebuilt` flag, C2 passes with the tree-byte difference accounted, `new-glyph` runtime flag raised |
| T12 | capacity: 101-byte key / 4 001-byte text | refused |
| T13 | per-locale: same O-K on `eng_us` and `rus_ru` of one title | independent trees/alphabets; refusal on the locale whose tree lacks a glyph does not block the other (caller decides atomicity) |
| T14 | placement: grown file into an EB v0x400 raw member / a PS4 chunkzip member / a loose file | archive re-parses, member decodes to the grown bytes, other members' hashes unchanged; disc-image route refused (§6) |

## 6. Placement — with the uncertainties kept

| Family | Copies that exist | What a growth write can do today | Unknown |
|---|---|---|---|
| Xbox 07–10 | loose `fe/loc/*.db`; 07/08 also a `data1.big` (BIG4) member | write the loose file; BIG4 member needs an entry re-plan | which copy the console/Xenia loads |
| Xbox 11 | `boot.big` member (EB v0x400, raw); loose copies only in extraction trees | `Eb::replace_payload` (reflow) | whether an extraction tree is ever read (launch source not established) |
| Xbox 12–Legacy | `cache.big` (12/13) / `cacheboot.big` (14+) member; loose copies in extraction trees; disc image | reflow the archive member; **the disc-image route (`tdb-xiso` range patch) is same-length only — growth is refused there** | tree vs archive vs image precedence |
| Recompiled | `game\` archives (= Xbox Legacy); optional user `_compiled` tree | as Xbox Legacy archive | `_compiled` precedence (handoff 01 §5) |
| PS3 12–Legacy | loose on disc **and** `cache*/cacheboot.big` member (identical bytes) | write both, or one after precedence is known | precedence |
| PS4 15–21 | `inxp_00.big` chunkzip member (PS4 15 = EB 0x00100400); NHL 15 and 19 also carry copies in `patch_inxp*.big` | re-`chunkzip` + reflow the base member; **a patch copy is a possible override until precedence is observed** — a plan must name every copy and let the user pick, never assume | patch precedence; EB 0x00100400 write support |

## 7. Gates — separated

- **Packaging gate** (offline): §5 T14 — container re-parses, member round-trips, other members unchanged.
- **Parser gate** (offline): `tdb-loc::parse` + `verify` on the output; T1a/T1b on all 95 files.
- **Runtime gate R-A — "the loader accepts a larger dictionary and reads an appended string"**: grow the
  English dictionary by one key (O-K on a class-A title) *and* re-point an existing, known-displayed key to the
  appended text (LOC-W1 semantics on the grown file); boot to that screen. Shows: the larger file loads and the
  appended pool region is read through an **existing queried row**. Does **not** show that the executable
  finds a new key.
- **Runtime gate R-B — "a new key is looked up"**: needs the executable to *query* the new key. Available
  bindings: PS4 `stockleagues.name` (the league text key is data — set one league's key to the new key);
  gen7 `kOtt.artabbr`-derived keys (`NHLTeamName_<artabbr>`, handoff 23 — change a team's `artabbr` to a new
  suffix and add `NHLTeamName_<suffix>`); both make the game hash-lookup the new key. Shows new-key lookup and,
  on class-A files, that a row inserted at its sorted position is found. Order tolerance on class B stays
  unknown (no insertion is offered there).
- **Glyph gate** (runtime): a rebuilt tree with a new symbol may still lack a font glyph — only a run shows it.
- **Patch-evidence scope**: PS4 NHL 15 / 19 patched builds ship larger `eng_us.db` files (+192 / +267 rows) —
  evidence that *those patched executables* load *their own* larger files; not evidence for unpatched
  executables, gen7/Recompiled loaders, Studio-built trees, new glyphs, or new-key lookup.

## 8. Revision history

**Rev. 3 (this document), from the revision-2 review:** (1) class A = nondecreasing/no descent, equal-hash
runs kept intact, the new-key collision refusal kept as a separate policy (B2, §2, §3, T2, T4); (2) P appends
after the physical pool end before the relocated EOF CRC, whole pool incl. slack preserved, offsets/sizes/CRCs
tabulated (§4-P); (3) C1 no longer promises an empty byte difference — decoded equality, preserved opaque
metadata and individually accounted structural differences, C2 compares to the requested result, changed
ranges validated against an independently derived expectation (§4-C, T1b, T8, T11); (4) baseline validation
excludes exactly the requested edits and selected sharers; T5 allows the edited pointer and size/CRC updates;
T7 cases start independently from the shared baseline; T9 already-null edit idempotent; existing-key refusal
scoped to insertion (§2, §3, §5); (5) B5 states 93/95 with the two NHL 07 anchor exceptions and the census
composition is stated from the JSON (§1).

**Rev. 2, from the growth-contract clarification:** I2 vs steps 3/5 (ascending vs preserved order) → §2/§3:
insertion only into class-A files, existing order always preserved, class-B insertion refused. Step 4 vs T1 →
§4: preserving serializer vs canonical rebuild with separate proofs. "No shared pointers" as a precondition →
B6 is an observation; Studio-touched inputs are supported (T7/T8). Runtime card → R-A vs R-B distinguished
with concrete new-key bindings. Placement → patch copies as possible overrides, disc-image same-length limit
preserved, extraction trees not claimed ignored. Null texts → rev. 1's "PS4 only" corrected (now B5 as above).

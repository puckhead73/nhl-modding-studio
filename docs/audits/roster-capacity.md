# Audit — roster-save capacity ceilings (Spike S3, VERIFIED)

> **Status: VERIFIED** by decode of the committed anchors (all PS3 `savefiles/*ROSTER*/SYS-DATA`
> inner TDBs + PS4 `databases/nhl_game.db` and `savefiles/*ROSTER*/DATA`), 2026-07-13, via the
> verified `tdb-savedata` extractor + `tdb-core::TdbFile::parse`. Every parsed DB reported
> `crc_ok=true` and (for pristine files) `size_ok=true`. Resolves the capacity-ceiling question that
> gates **Phase 4.4 roster capacity normalization**
> ([phase-4-roster-ops-transactions.md §4.4](../plans/phase-4-roster-ops-transactions.md)).
>
> Companion: [anchor-capacity-and-editable-tables.md](anchor-capacity-and-editable-tables.md) (the
> three-ceilings model, main-DB totals) — this doc adds the **vanilla-save `max_records`** lens the
> normalization pass needs. See also [roster-pools.md](roster-pools.md) (S1),
> [towx-blocks.md](towx-blocks.md) (S2).

## Question

Phase 4.4 must **auto-grow a game-trimmed roster save's roster-bearing tables to the vanilla ceiling**
so edits never hit a premature `CapacityExhausted`. What is that ceiling — the per-(title, platform)
allocated size (`max_records`) of the vanilla save — for the seven roster-bearing tables
`ZBac` (players/index), `ubPc` (bio), `dSvy` (skater AI), `xmja` (skater equip), `mHuy` (goalie AI),
`fMVl` (goalie equip), `eGlu` (roster slots)? Sourced from real bytes, never hardcoded.

## Finding — the ceiling is `max_records`, and *where it lives differs by generation*

Reading `current_records` (population) and `max_records` (allocated slots) for the seven tables from
both the main game DB and the vanilla ROSTER save exposed a clean two-generation split:

- **GEN7 (PS3 = Xbox360).** The main `nhlng.db` is **packed** — `max_records == current_records` for
  all seven tables (100 %, confirming [anchor-capacity §3.1](anchor-capacity-and-editable-tables.md)).
  The vanilla **ROSTER save carries the same population but pre-allocated slack**:
  `current_records == main.current_records` yet **`max_records > current_records`**. So on GEN7 the
  **ROSTER save's `max_records` is the ceiling**, and it is *larger* than the packed main-DB size —
  the main DB does **not** expose it. (Xbox360 ships no ROSTER save in the anchor set, but its
  `nhlng.db` is byte-identical to PS3's, so the same inner-TDB ceiling applies; only the save
  *container* differs.)
- **GEN8 (PS4).** The main `nhl_game.db` **already carries the slack** — `max_records > current_records`
  ([anchor-capacity §3.4](anchor-capacity-and-editable-tables.md)) — and the vanilla ROSTER save's
  `max_records` is **identical to the main DB's** (verified on NHL 16, the one pristine PS4 ROSTER in
  the set). So on PS4 the ceiling is readable from **either** file's `max_records`.

### Cross-platform validation — the ceiling is a property of the *roster-content year*, not the platform

Two exact identities fall out of the numbers and strongly corroborate that these are real ceilings,
not artifacts:

- **PS3 NHL 15 ROSTER `max` ≡ PS4 NHL 15 main `max`** — `ZBac 8948 · ubPc 7034 · dSvy 6479 ·
  xmja 6479 · mHuy 855 · fMVl 855 · eGlu 10619` (identical across the whole row).
- **PS3 NHL Legacy ROSTER `max` ≡ PS4 NHL 16 main `max`** — `ZBac 9955 · ubPc 7988 · dSvy 7331 ·
  xmja 7331 · mHuy 957 · fMVl 957 · eGlu 10879` (NHL Legacy and NHL 16 are the same roster year).

The GEN7 main DB is merely *packed*; the ceiling it hides is the exact one PS4 exposes for the same
content. Both platform families agree byte-for-byte on the reservation size.

## Ceiling tables (the deliverable — from real bytes)

**`max_records` = the ceiling** to normalize a trimmed roster save up to. Population is the vanilla
`current_records` (shown for context / to compute headroom).

### GEN7 — PS3 = Xbox360 (ceiling = vanilla PS3 ROSTER save `max_records`)

| title | ZBac | ubPc | dSvy | xmja | mHuy | fMVl | eGlu |
|---|--:|--:|--:|--:|--:|--:|--:|
| NHL 10 | 5688 | 4533 | 3999 | 3999 | 534 | 534 | 6219 |
| NHL 11 | 7093 | 5910 | 5260 | 5260 | 650 | 650 | 7728 |
| NHL 12 | 8283 | 6898 | 6324 | 6324 | 874 | 874 | 7853 |
| NHL 13 | 8907 | 6946 | 6407 | 6407 | 839 | 839 | 10457 |
| NHL 14 | 8803 | 6873 | 6350 | 6350 | 823 | 823 | 10353 |
| NHL 15 | 8948 | 7034 | 6479 | 6479 | 855 | 855 | 10619 |
| NHL Legacy | 9955 | 7988 | 7331 | 7331 | 957 | 957 | 10879 |

GEN7 population (vanilla `current_records`, == main-DB packed size):

| title | ZBac | ubPc | dSvy | xmja | mHuy | fMVl | eGlu |
|---|--:|--:|--:|--:|--:|--:|--:|
| NHL 10 | 4188 | 3933 | 3499 | 3499 | 434 | 434 | 3719 |
| NHL 11 | 5593 | 5310 | 4760 | 4760 | 550 | 550 | 5228 |
| NHL 12 | 5779 | 5395 | 4821 | 4821 | 574 | 574 | 5349 |
| NHL 13 | 5874 | 5412 | 4875 | 4875 | 537 | 537 | 5446 |
| NHL 14 | 5827 | 5374 | 4851 | 4851 | 523 | 523 | 5353 |
| NHL 15 | 5942 | 5534 | 4978 | 4978 | 556 | 556 | 5511 |
| NHL Legacy | 6087 | 5632 | 5066 | 5066 | 566 | 566 | 5532 |

Headroom (ceiling − population) is substantial everywhere — e.g. NHL Legacy `eGlu` 5532 → 10879
(**+5347 slots, ~2×**), `ZBac` 6087 → 9955 (**+3868**). EA ships the roster save with roughly a
doubling of capacity above the shipped population.

### GEN8 — PS4 (ceiling = `nhl_game.db` `max_records`, == vanilla ROSTER `max` where present)

| title | ZBac | ubPc | dSvy | xmja | mHuy | fMVl | eGlu |
|---|--:|--:|--:|--:|--:|--:|--:|
| NHL 15 | 8948 | 7034 | 6479 | 6479 | 855 | 855 | 10619 |
| NHL 16 | 9955 | 7988 | 7331 | 7331 | 957 | 957 | 10879 |
| NHL 17 | 10935 | 9428 | 8236 | 8236 | 1192 | 1192 | 11714 |
| NHL 18 | 11825 | 10371 | 9143 | 9143 | 1228 | 1228 | 13502 |
| NHL 19 | 14913 | 12714 | 11078 | 11078 | 1636 | 1636 | 15567 |
| NHL 20 | 14913 | 12714 | 11078 | 11078 | 1636 | 1636 | 15567 |
| NHL 21 | 15615 | 12439 | 10841 | 10841 | 1598 | 1598 | 16576 |

PS4 population (`nhl_game.db` `current_records`):

| title | ZBac | ubPc | dSvy | xmja | mHuy | fMVl | eGlu |
|---|--:|--:|--:|--:|--:|--:|--:|
| NHL 15 | 5984 | 5555 | 4996 | 4996 | 559 | 559 | 5523 |
| NHL 16 | 6087 | 5632 | 5066 | 5066 | 566 | 566 | 5532 |
| NHL 17 | 6701 | 6128 | 5527 | 5527 | 601 | 601 | 6430 |
| NHL 18 | 7735 | 7236 | 6522 | 6522 | 714 | 714 | 7373 |
| NHL 19 | 8581 | 7136 | 6389 | 6389 | 747 | 747 | 8328 |
| NHL 20 | 8581 | 7136 | 6389 | 6389 | 747 | 747 | 8328 |
| NHL 21 | 9281 | 7247 | 6498 | 6498 | 749 | 749 | 9090 |

> NHL 20 is identical to NHL 19 (same roster generation in the anchor set). `mHuy`/`fMVl` (goalie AI /
> equip) always share a count, as do `dSvy`/`xmja` (skater AI / equip) — the companion pairs allocate
> in lock-step, consistent with the entity model.

## "Is growth actually needed, and by how much?"

The spike method asked to compare against a *trimmed* roster save. **Empirical result: no committed
anchor roster save is trimmed below its ceiling** — every vanilla ROSTER already carries
`max_records == ceiling`. Two observations qualify this:

- The pristine PS4 NHL 16 ROSTER is byte-consistent with its main DB (`max` and `cur` identical) —
  vanilla, untrimmed.
- The PS4 NHL 17/18/19 ROSTER anchors are `RosterCustom0` **edited/community** saves (`size_ok=false`,
  a re-serialized file), **not** pristine vanilla. Even so, their `max_records` still equal the main
  DB's — the ceiling survives editing. NHL 19's custom roster illustrates the ceiling being *used*:
  its `ZBac` population is **9801** vs. vanilla **8581**, i.e. a community roster consumed ~1220 of the
  ~6332 reserved slots. So the headroom is real and gets spent by exactly the roster expansion Phase 4
  targets.

**Conclusion for 4.4:** within the anchor set, normalization is a **no-op** (files already meet the
ceiling); the growth path is a **safety net for externally-sourced / game-rewritten trimmed saves**
whose `max_records` has been packed down to `current_records`. We have the ceiling; we do **not** have
a trimmed anchor to grow, so 4.4's growth must be exercised against a synthetically-packed fixture
(pack a roster table's `max` down to `cur`, then normalize back to the ceiling).

## Implications for Phase 4.4 (normalization pass)

1. **Ceiling source, per generation** (the load-bearing consequence):
   - **PS4:** the ceiling is `max_records` of the **attached main `nhl_game.db`** — runtime-derivable
     from the open workspace, **no hardcoded constants needed**. (Also equals the roster save's own
     `max`, which is why PS4 normalization is typically already satisfied.)
   - **GEN7:** the main `nhlng.db` is **packed and cannot supply the ceiling.** The ceiling lives only
     in the vanilla ROSTER save, so 4.4 must carry the **GEN7 ceiling table above** as a
     `(title → [ZBac,ubPc,dSvy,xmja,mHuy,fMVl,eGlu])` constant that **traces to this doc**
     (satisfies the Phase 4.7 "no hardcoded ceilings not traceable to roster-capacity.md" grep).
2. **Grow `max_records`, not `current_records`.** The ceiling is an *allocation* target (reserved
   slots), exactly the vanilla ROSTER shape (`cur < max`). Normalization should raise a packed table's
   `max_records` up to the ceiling **without** inventing population rows — leaving `current_records`
   untouched so later `create_player`/`sign_from_fa` inserts fill slack instead of triggering
   per-insert block growth. (`tdb-core::append_record`'s full-table-growth path already preserves other
   records + CRC scopes; the pass pre-applies that growth in bulk.)
3. **Idempotent + main-DB no-op.** A file already at the ceiling is unchanged; run the pass only on the
   attached **roster save**, never on `nhlng.db`/`nhl_game.db` (the GEN7 main is intentionally packed).
4. **Only the seven roster-bearing tables** are in scope; the rest of the roster save's subset
   projection is left as-is.

## Confidence & caveats (CLAUDE.md rule 3)

- **VERIFIED (high):** every ceiling above is a `max_records` read from a real anchor inner-TDB that
  parsed CRC-clean. The two cross-platform identities (NHL 15, Legacy≡16) are independent corroboration.
- **GEN7 = the proven titles' authority.** Ceilings for NHL 10–15 + Legacy come from committed PS3
  ROSTER saves. Xbox360 ceilings are inferred from the **byte-identical** `nhlng.db` (container differs,
  inner-TDB dims do not) — consistent with the established PS3≡Xbox360 DB fact, but no Xbox360 ROSTER
  was decoded here.
- **PS4 NHL 15 ROSTER** (`84f8102f - ROSTER/DATA`) did **not** extract (`unrecognised save payload` — a
  container variant `tdb-savedata` doesn't yet decode); its ceiling is taken from `nhl_game.db` `max`,
  which NHL 16 proves equals the vanilla ROSTER `max`. **PS4 NHL 20/21** ship no ROSTER anchor; ceilings
  are from `nhl_game.db` `max`. These PS4 titles are net-new (out of the proven set) — the numbers are
  real reads but must still pass a round-trip/in-game gate before PS4 normalization is asserted working.
- **No trimmed anchor exists** to demonstrate growth end-to-end (see above) — 4.4 verifies against a
  synthetic packed fixture.

## Reproduction

Throwaway reader (removed after capture, matching the S1/S2 convention): for each
`anchors/<platform>/<game>`, read `databases/{nhlng,nhl_game}.db` and extract
`savefiles/*ROSTER*/{SYS-DATA,DATA}` via `tdb_savedata::SaveContainer::extract_db` /
`ps4_extract_roster`, then `TdbFile::parse` both and print `current_records`/`max_records` for the
seven `table_canonical` ids. All vanilla DBs reported `crc_ok=true, size_ok=true`.

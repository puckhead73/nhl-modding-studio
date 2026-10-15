# Phase 3 (Players) completion ledger — 25 September 2026

Built from the [entity roadmap](../../plans/entity-editing-parity-roadmap.md) Phase 3 table, the
[Phase 3 coverage matrix](../../validation/phase-3-coverage-matrix-2026-09-22.md) (3.4 rows of 22 Sep;
3.1 / 3.2 rows appended by receipts 11–14), the operation matrix
`docs/audits/data/operation-matrix.csv` (486 rows: 18 roster operations × 27 anchors, read only —
not regenerated), and the code in the working tree at HEAD `8d60a6d` + uncommitted campaigns.
Every "done" is **developer-verified only**; nothing is Codex-accepted, desktop-exercised or
game-verified. Same vocabulary as the League / Team ledger: **user-facing complete (offline)** =
reader + editor / writer + API + UI + a meaningful automated case per family; **internal only**;
**blocked** (the missing contract or fixture named); **n/a** (evidence-backed); **untested** (the
code path exists for the family but no case runs it there); **research unknown**.

Families: **07** (NHL 07, both consoles, research-only target), **08–10**, **11–Legacy** (X360 / PS3),
**PS4 15–21**, **Recomp** (by identity with Legacy; no independent anchor).

## 3.1 Identity data

| Op | Coverage | State | Concrete missing work |
|---|---|---|---|
| Names, birth date, birthplace / nationality, position, handedness, height, weight — read + edit + save / reopen / undo | 27 / 27 anchors (receipt 11); all 12 identity columns present on every anchor; names UTF-8 everywhere | **user-facing complete (offline)** | runtime agreement with the game's own display (card V-55) |
| Per-title encodings (birth m / d index; weight base; height +54) | 27 / 27 pinned; **weight base fixed** for PS4 + NHL 08 (receipt 11); **since receipt 20 the base and the index come from the schema-family contract (`studio_domain::encoding`), the cell probes are diagnostics only**; title / schema conflict refuses the encoded writes | complete (offline) | — |
| String width shown before the write | `text_max_bytes` on every string field view; the editor refuses over-width text in UTF-8 bytes before the write (receipt 16) | complete (offline) | — |

## 3.2 Additional database entries

| Op | Coverage | State | Concrete missing work |
|---|---|---|---|
| Ratings read / edit / save / reopen / undo; scale per title | 27 / 27 skater + goalie (receipt 13); scale 50 / 36 / 0 **from the schema-family contract since receipt 20** (07 in game; 08–10 `Inferred`, provisional; 11–Legacy in game for 14 / 15 / Legacy; PS4 metadata) — a bulk edit cannot move it (proven red → green) | **complete (offline)**; 08–10 scale **provisional** | research / console: one known rating on NHL 09 / 10 to confirm the +50 reading; NHL 08 `g_positioning` = 50 (33 rows) |
| Style / archetype, potential range / accuracy | 27 / 27 style (position-split vocabulary); potential 14–Legacy + PS4 (absent 07–13) | complete (offline) | NHL 10–13 4-bit `p_style` vocabulary **research unknown** (raw shown, editable in range) |
| Overall: computed vs stored | tab + roster list era-gated (receipt 12); calibrated fit NHL 14–21; stored 13; none 07–12 | complete (offline) | the list does not label "stored" vs "calculated" (UI hint) |
| Traits (32 slots) | trait dictionary with confidence flags; activate / deactivate | complete (offline); labels **provisional** by confidence | research: weak-confidence trait labels |
| Contracts (cap hit, term, two-way, status), UFA year | cap-hit unit per **schema family**, each witnessed by 6–25 independent public contracts (receipt 20; receipt 17's three players were the start): ×$25 000 on NHL 07–10 (NHL 08 measured, 7 contracts), ×$5 000 on NHL 11 → Legacy and PS4 (real NHL 20 fixture included); `ubPc` has no `salary` column — `contractdollars` is the AAV | complete (offline) | — |
| Draft data (team, round, pick, year) | fields from NHL 11 (`draftyear` +1900); NHL 07 / 10 `yeardrafted` base unconfirmed → raw | complete (offline) 11–PS4; **partial** 07 / 10 | base year for `yeardrafted` |
| Career stats rows (add / delete season) | operation matrix: pass 08–Legacy (6 + 12), PS4 15–17; **refused-clean** NHL 07 (no career tables) and PS4 18–21 (tables absent) | complete (offline) where the schema has tables; n/a elsewhere (evidence: table absence) | — |
| Player create / clone / delete (cluster) | operation matrix: pass on 25 / 27; **NHL 07 refused** (no slot key) — refusal now in words (receipt 14) | complete (offline) 08–PS4; **n/a NHL 07** (schema) | — |
| Roster operations: captaincy, jersey, dressed / scratched, release / sign, trade slots, autocorrect jerseys, repair blocks | pass 27 / 27 | complete (offline) | `set-dressed` writes `ZBac.RREB` (player TYPE per memory S4) — **verify the field** before relying on it |
| Move player / add-remove membership | pass on 11 anchors (operation matrix); **full-block refusal swept 27 / 27** (receipt 15) — the refusal names the team, nothing moves | complete (offline) | — |
| Draft-pick trades | pass 11–Legacy + PS4; refused 07–10 (no table) | complete (offline); n/a 07–10 | — |
| Historical import (person → cluster) | **27 / 27** (receipt 14): every probe flows through `lower_create`; NHL 07 refused in words | complete (offline) | corpus-backed import proven on NHL 14 only (`import_corpus_e2e`); the synthetic sweep covers the encodings, not the corpus join per title |
| Bulk edit | `set_cells_int_rows` (one undo entry) used by the schedule import; **no player bulk-edit UI** | **internal only** | a player bulk-edit surface is a scope question (roadmap lists it) — not started |
| Formula profiles (career ratings) | `compute_career_ratings`, Formulas page; era calibration NHL 14 / 15 | complete (offline); pre-14 **uncalibrated by design** | — |
| Legends team builder | engine + API + status; UI pages present | complete (offline) as built (memory: API + UI landed after 13 Aug) | qualification per family not swept |
| Animation / stance / celebration assignment | fields exist on **PS4 only**: `stance` (16–21), `goalcelebration` (17–21), `mouthguardanim` (18–21), `animpack` (19–21), `introanimid` (21); no gen7 field; edited as raw ints through the generic field editor, no vocabulary | **partial**: editable raw, **research unknown** vocabularies; n/a on gen7 (no field) | research: value → animation name tables |
| Companion records / player indexes audit | `companion_presence`, schema-aware equipment requirement (NHL 07), orphan bio stubs skipped (NHL 09) | complete (offline) | — |

## 3.3 Localization (receipt 18, measured on 27 anchors)

| Text | Where it lives | State | Concrete missing work |
|---|---|---|---|
| First / last name, PS4 nickname, birth city | inline biography strings in the player row (widths 160 / 40 / 25 B by era); UTF-8; never a `$` key — **except** eight PS4 17–21 "Joe NHL" template rows whose birth city is `$BURNABY` (the dictionary resolves it) | complete (offline) — edited as strings with the byte-width pre-check (receipt 16) | optional: resolve a `$`-keyed birth city through the attached dictionary in the header (template rows only) |
| Birth state (NHL 08–10 `statename`) | 40 B string, **empty on every row**; the enum column is the source | n/a (evidence) | — |
| Audio-name references | `audioid` (14 bits NHL 07–13, 16 from NHL 14; NHL 07 `defaultaudioid`; NHL 21 `surnameid`) — numeric, consumed by the speech catalogue chips (receipt 07) | listing / preview (3.5) | replacement blocked (3.5) |
| Studio labels (positions, nationalities, field names) | `defs/` enum and metadata vocabularies, not game dictionaries | complete (offline) | NHL 10–13 style vocabulary research unknown (3.2) |
| PS4 16–21 team names | dictionary keys — Phase 1.3 routes | out of 3.3 | — |

## 3.4 Assets and appearance — see the 22 Sep matrix rows (portrait / head containment listing complete; reassignment and new assets **not impl**; head converter external)

## 3.5 Audio — name calls: gen7 diagnostic preview reachable from the player card (receipt 07); PS4 bank listing (receipt 04); replacement / cue creation **blocked** (no encoder / event contract)

## Oldest actionable Phase 3 gap, offline

~~Move / add-membership refusal on a packed roster~~ — closed by receipt 15. ~~String-width pre-check~~ —
closed by receipt 16. ~~Contract-dollar units~~ — closed by receipt 17. Remaining 3.2 rows are research /
scope questions; the developer moves to 3.3. Formerly next: the contract-dollar granularity measurement for NHL 08–10 / PS4 18–21 (a
research question unless a known contract can anchor it).

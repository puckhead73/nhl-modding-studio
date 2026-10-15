# QA ledger — Phase 0 inventory & schema freeze

> **Status:** established 2026-08-01 (Phase 0 of the
> [Full-game QA and editability plan](../plans/full-game-qa-editability.md)).
> **Ground truth:** the immutable `anchors/` tree (27 committed main databases + their metadata,
> localization/auxiliary databases, and save containers). **Nothing here is `game-verified`** — Phase 0
> is inventory and evidence plumbing only.

Phase 0 freezes *what exists* and wires the drift guards the later phases build on. Every artifact is
regenerated deterministically from bytes on disk by a committed tool; no numbers are hand-maintained.

## Regenerating

One command rebuilds all four machine artifacts under [`data/`](data/):

```
cargo run -p tdb-qa --bin qa-inventory
```

It only ever **reads** `anchors/`. Re-running produces a byte-identical result (verified by
`crates/tdb-qa/tests/inventory.rs`), so a diff after regeneration means an anchor or a schema changed —
review it, don't paper over it. The generator lives in [`crates/tdb-qa`](../../crates/tdb-qa) (depends
only on `tdb-core` + `tdb-defs`; the builders are pure functions the tests re-run in memory).

## Artifacts

| Artifact | What it freezes |
|---|---|
| [`data/anchors-manifest.csv`](data/anchors-manifest.csv) | every file under `anchors/` — role, size, **sha256** (552 files). The integrity/hash-drift guard. |
| [`data/schema/<platform>-<game>.json`](data/schema/) (×27) | per main-DB schema snapshot: endian, table ids/names, field ids/names, tdb kind + metadata type, bit offset/width, ranges, update flag, key, FK parent, current/max records (**capacity**). |
| [`data/schema-diff.csv`](data/schema-diff.csv) | field-level cross-title/cross-platform diff: table/field added·removed·**resized**·**retyped**·renamed vs a family baseline, plus PS3↔Xbox360 parity. |
| [`data/game-qa-matrix.csv`](data/game-qa-matrix.csv) | the ledger: one row per game × platform × feature family, with `status,automated_test,manual_case,last_verified,evidence,blocker`. |

The earlier hand-made `data/drift-matrix.csv` + [schema-drift-matrix.md](schema-drift-matrix.md) remain
as a *table-level* vs-Legacy narrative; `schema-diff.csv` is the Phase-0 **field-level** superset.

## Status vocabulary

Each matrix cell carries exactly one of: `not-applicable`, `missing-fixture`, `discovered`,
`read-pass`, `edit-roundtrip-pass`, `game-verified`, `blocked`. Phase 0 assigns only the first four
plus `blocked`; `edit-roundtrip-pass` and `game-verified` are earned by later phases from their own
gates.

### How Phase 0 derives status (conservative, evidence-tied)

| Family | Rule |
|---|---|
| `db-presentation` | `read-pass` for every real main-DB anchor — proven by the Phase-1 27-anchor read matrix `studio_domain::read_matrix::read_matrix_over_all_anchors`. Recomp has no anchor → `missing-fixture`. |
| `db-operations` | **`edit-roundtrip-pass`** for every real main-DB anchor — proven by two gates: the Phase-2 scalar mutation matrix (`mutation-matrix.csv`) and the Phase-3 relational operation matrix `studio_domain::operation_matrix::operation_matrix_over_all_anchors` (+ `operation-matrix.csv`), which drives every implemented roster operation through the `EditSession` seam, holds the invariant battery + save/reopen, reverses cleanly, and refuses schema-absent operations without a half-commit. The `blocker` now scopes only in-game acceptance (`game-verified`, manual). Recomp → `missing-fixture`. |
| `save-raw-db` | **`edit-roundtrip-pass`** — the loose-DB save is a byte-identical no-op round trip (Phase-2 mutation matrix + `save-matrix.csv`). |
| `save-inner-container` | **`edit-roundtrip-pass`** where a save fixture ships — Phase-4 container matrix `studio_domain::save_matrix::save_matrix_over_all_fixtures` (+ `save-matrix.csv`): detect-by-content → extract → edit → repack → reopen with valid checksums. NHL 10/11 `v1` and PS4 keep their experimental blocker; multi-part / v5-settings / NHL15-PS4 saves stay read-only. |
| `onboarding-install` | `discovered` when the backing fixture exists. |
| `save-outer-package` | `blocked` on Xbox (STFS packaging not implemented); `not-applicable` elsewhere (anchors expose inner payloads). |
| `onboarding-localization` | `discovered` when a localization DB ships; `missing-fixture` otherwise. |
| `assets-*` (portraits, logos, jerseys, equipment, heads, arenas, audio) | `missing-fixture` — **no asset fixtures live under `anchors/`**; asset codec fixtures live in crate test dirs, outside the anchor ground truth. |

`last_verified` is set where an automated gate currently proves the row: `2026-08-01` for the
Phase-1 `read-pass` cells, `2026-08-02` for the `edit-roundtrip-pass` cells (Phase-2 scalar codecs,
Phase-3 relational operations, and Phase-4 save-container round-trips); everything else is left blank
until a phase verifies it.

## Catalog → install / schema target

17 catalog entries; 27 supported game×platform cells + 1 research copy. Platform tokens match the
anchor tree (`ps3`, `xbox360`, `ps4`, `recomp`).

| Entry | Xbox 360 | PS3 | PS4 | Recomp | Generation | Main DB / schema target |
|---|:--:|:--:|:--:|:--:|---|---|
| NHL 07 | ✓ | *research* | — | — | gen6 | `NHL07NG.db` (+ `NHL07NG-meta.xml`) |
| NHL 08 | ✓ | ✓ | — | — | gen7 | `nhlng.db` |
| NHL 09 | ✓ | ✓ | — | — | gen7 | `nhlng.db` |
| NHL 10 | ✓ | ✓ | — | — | gen7 | `nhlng.db` (v1 save experimental) |
| NHL 11 | ✓ | ✓ | — | — | gen7 | `nhlng.db` (v1 save experimental) |
| NHL 12 | ✓ | ✓ | — | — | gen7 | `nhlng.db` |
| NHL 13 | ✓ | ✓ | — | — | gen7 | `nhlng.db` |
| NHL 14 | ✓ | ✓ | — | — | gen7 | `nhlng.db` |
| NHL 15 | ✓ | ✓ | ✓ | — | gen7 (PS3/Xbox) + gen8 (PS4) | `nhlng.db` / `nhl_game.db` |
| NHL 16 | — | — | ✓ | — | gen8 | `nhl_game.db` |
| NHL 17 | — | — | ✓ | — | gen8 | `nhl_game.db` |
| NHL 18 | — | — | ✓ | — | gen8 | `nhl_game.db` |
| NHL 19 | — | — | ✓ | — | gen8 | `nhl_game.db` (no localization DB shipped) |
| NHL 20 | — | — | ✓ | — | gen8 | `nhl_game.db` |
| NHL 21 | — | — | ✓ | — | gen8 | `nhl_game.db` (no localization DB shipped) |
| NHL Legacy | ✓ | ✓ | — | — | gen7 | `nhlng.db` (the gen7 baseline) |
| Legacy Recompiled | — | — | — | ✓ | recompiled-gen7 | **shares** `xbox360/nhllegacy/nhlng.db` (Xbox-derived; no independent anchor) |

**Research copy:** the PS3 `NHL07NG.db` anchor is present for research but PS3 is not a shipping NHL 07
platform (a `—` cell in [scope & compatibility](../01-scope-and-compatibility.md)). It gets a schema
snapshot and parses (`read-pass`), but every one of its matrix rows is flagged `research copy — not a
supported catalog platform` in `blocker`, and it is **not** one of the 27 supported cells.

**Legacy Recompiled:** Xbox-derived, so it tracks `xbox360/nhllegacy` until an independent Recomp
fixture proves otherwise. It has no own DB/save anchor; its DB rows are `missing-fixture` pointing at
the shared schema, and its save rows are `missing-fixture` (user-supplied).

## Fixture coverage & known gaps

- **27 / 27 main databases** present with self-contained metadata (PS3 ×10, Xbox360 ×10, PS4 ×7).
- **PS3 ≡ Xbox 360**: schema-diff reports **zero** `platform-parity` rows — the two platforms' gen7/gen6
  databases are field-for-field identical; they diverge only in the save container.
- **Localization**: shipped for all PS3/Xbox360 games; PS4 ships `eng_us.db` for 15/16/17/18/20 but
  **not nhl19 / nhl21** (metadata only) → `missing-fixture`.
- **Saves**: PS3 `BLUS…` containers, Xbox360 `<TYPE> <timestamp>`, PS4 `DATA/INDEX/NAME` triads — one
  set per catalog cell; Recomp saves are user-supplied (`missing-fixture`).
- **Archives & extracted assets**: **none under `anchors/`** → every `assets-*` family is
  `missing-fixture`. Asset codec fixtures live under crate test directories and are qualified in
  Phase 5, not here.
- **Blocked**: Xbox STFS outer packaging (`save-outer-package` on all 10 Xbox360 cells).

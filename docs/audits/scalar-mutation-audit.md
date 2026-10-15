# Scalar field mutation audit (Phase 2)

> **Status:** established 2026-08-02 (Phase 2 of the
> [Full-game QA and editability plan](../plans/full-game-qa-editability.md)).
> **Automated gate:** `studio_domain::mutation_matrix::mutation_matrix_over_all_anchors`
> (`crates/studio-domain/tests/mutation_matrix.rs`).
> **Machine-readable companion:** [`data/mutation-matrix.csv`](data/mutation-matrix.csv) — the
> representative field + baseline/edited value proven for every codec on every one of the 27 main-DB
> anchors, drift-guarded by the same test.

Phase 1 made the UI stop *showing* fields a schema can't back. Phase 2 proves the fields it does show
are **truly editable**: for every distinct on-disk value codec in every schema, an edit staged through
the exact seam the UI calls survives save → reopen byte-for-byte, undoes back to the original bytes,
redoes deterministically, and refuses illegal input without disturbing the file or the history.

## What "the same path the UI calls" means

Every proof runs through `EditSession::set_field(studio_table, record, studio_field, FieldValue)` —
the method `studio_api::Registry::set_field` (and therefore the Tauri command, and therefore the React
field editors) calls. It validates against metadata, encodes to raw bits, and writes inside the
Layer-3 **repairing transaction** (audited, undoable). So a green matrix row means the production edit
path — not a test-only shortcut — round-trips that codec.

Anchors are immutable: each session opens a **temporary copy** of the `.db` (`Project::load` over a
copied file, so `save()` is a raw-db pass-through and byte-equality is exact). `git status --short
anchors/` stays clean.

## The codec taxonomy

A field's *codec* is how its logical value maps to stored bits — independent of its UI label. The
distinct codecs, classified from `tdb_defs::FieldMeta`:

| Codec | Rule | Proves |
|---|---|---|
| `boolean` | 1-bit integer, range `0..=1` | single-bit set/clear |
| `unsigned` | integer, `range_low >= 0` | the general count / enum / rating / FK-index encoding |
| `signed-twos-complement` | negative `range_low` that fits `bit_depth` signed | two's-complement negatives (e.g. `-16384..16383` at 15 bits) |
| `neg1-sentinel` | `range_low == -1`, not two's-complement | the nullable / "none" encoding where raw all-ones = `-1` |
| `offset-binary` | the anchored `raw = value - range_low` sites | career plus/minus (`OAqO.Uomt`), where two's-complement would misread the value |
| `inline-string` | `FieldType::String`, byte-aligned width | fixed-width text with the NUL/​max-length policy |

Endianness (big-endian on PS3/Xbox 360, little-endian on PS4) is a whole-database property, so it is
covered by running the matrix over **both** anchor kinds rather than as a separate class. Enums,
ratings, dates, salary/year scales and team/arena FKs are *display* semantics layered over these
codecs — their raw read/write inverse is proven by the underlying `unsigned` / `signed` /
`neg1-sentinel` proof; their friendly formatting is a Phase 1 presentation concern.

## Representative selection (deterministic)

For each codec present in a schema, the harness scans tables and fields in **canonical-id-sorted
order** and picks the first editable (`update=True`, non-key) candidate whose full cycle round-trips
cleanly. A candidate whose edit would trip the transactional audit (a structural `eGlu`
slot/jersey/captaincy field, a companion-link pointer) **self-filters** — those are relational
operations proven in Phase 3, not scalar codecs — so the proof always lands on a genuine scalar. The
sorted scan makes the selection byte-identical across runs and hosts, which is what lets
`mutation-matrix.csv` be drift-guarded.

Coverage is honest about absence: `offset-binary` has no row on the PS4 anchors because those career
plus/minus tables are not in `nhl_game.db`, and `signed-twos-complement` first appears on NHL 11 — the
earlier schemas carry no two's-complement field the harness can bound. A codec absent from a schema
gets no row rather than a synthetic pass.

## The eight-step contract (per representative, per value)

Run for both range boundaries of every integer codec, and for the max-length + short + Unicode strings
of the string codec:

1. **Read baseline** raw bits and the decoded friendly value.
2. **Choose** a different valid value, including the range boundaries (and the `-1` sentinel for the
   neg1 codec).
3. **Stage** it through `EditSession::set_field` (validated + encoded + transactional).
4. **Confirm the pending edit**: the session is dirty, exactly one new undo entry exists, and the
   working bytes now decode to the chosen value.
5. **Serialize + reopen**: `save()`, re-parse, and confirm the raw bits *and* decoded value survive
   with a valid header CRC and size; a second no-op `save()` is byte-identical (deterministic output).
6. **Undo**: prove **byte equality** with the baseline — only the target field ever moved, and the
   re-saved main DB equals the anchor.
7. **Redo**: prove the saved output is byte-identical to the first edit (deterministic redo).
8. **Reject invalid**: an out-of-range value (one past the high bound; for strings one byte over the
   field width) is refused with **no** change to bytes or undo history.

## String policy

`set_inline_str` zero-fills the field then writes the UTF-8 bytes, so read-back is exact. The width
budget is a **byte** budget, not a character count:

- a **max-length** ASCII fill (exactly the field width, leaving no NUL terminator) round-trips —
  `read_inline_str` returns the full width;
- a **multi-byte UTF-8** value trimmed to the byte budget round-trips byte-for-byte;
- a string **one byte over** the width is rejected at entry (no truncation, no write).

## Negative gates (read-only + identity)

Per anchor the harness also proves the guardrails the plan requires:

- an `update=False` integer field is **refused** (`ValidationError::NotUpdatable`);
- a **key / identity** field is **refused as a scalar** — even one the schema marks `update=True`;
- neither refusal changes the working bytes or the undo history.

## Defect found & fixed — editable primary keys

The matrix surfaced a real gap: **NHL 07 ships primary keys marked `update=True`** (e.g. `ADlP.vfEq`),
and the scalar edit guardrail keyed only on the `update` flag — so a key could be edited in place,
which would orphan its foreign keys. Fixed by making *both* sides of the contract reject keys:

- **write path** — `EditSession`'s `prepare_field_edit` now refuses any `key` field regardless of its
  `update` flag (the structural operations that legitimately assign ids mint them through the Layer-1
  path, never this validated scalar seam, so they are unaffected);
- **presentation** — `FieldView.editable` is now `updatable && !key`, so the UI never renders an
  editor on a key; keys remain visible as read-only cells for disclosure.

The Phase 1 read matrix's editor-honesty assertion was tightened to match (`editable == updatable &&
!key`). Severity: **medium** (misleading/unsafe surface; no data-loss occurred because the anchors are
edited only on temp copies) — closed with the regression coverage above.

## QA-matrix promotion

`db-operations` for every main-DB anchor moves from `discovered` to **`edit-roundtrip-pass`**, citing
this gate + `mutation-matrix.csv`. The row's `blocker` scopes the claim precisely: **scalar / string /
enum / FK codecs are edit-roundtrip-proven (Phase 2); the exhaustive structural roster-operation matrix
is Phase 3.** No `game-verified` is assigned — that needs manual in-game evidence.

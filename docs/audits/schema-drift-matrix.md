# Schema Drift Matrix — all 7 PS3 games vs NHL Legacy

> Verified field-set drift of each game's `nhlng.db` against NHL Legacy's `nhlng.db`. Reproduces the
> staging TSV semantics (see [staging-tsv-verification.md](staging-tsv-verification.md)) and extends to
> **all seven games**, including **nhl14** (which had no staging TSV). Machine-readable:
> [`data/drift-matrix.csv`](data/drift-matrix.csv).
>
> **Superseded field-wise by Phase 0:** this doc is table-level (field-set membership + counts). The
> [`data/schema-diff.csv`](data/schema-diff.csv) artifact ([qa-ledger.md](qa-ledger.md)) carries the
> full **field-level** diff across the whole catalog — added/removed/**resized**/**retyped**/renamed
> fields, plus NHL 07 (gen6) and the PS4 line — including the bit-depth/encoding changes noted below as
> living in the per-field schema snapshots.

Categories (field-set membership only): `IDENTICAL` · `SUBSET (#missing,0)` · `SUPERSET (0,#extra)` ·
`RESHUFFLED (#missing,#extra)` · `<GAME>-ONLY` · `LEGACY-ONLY`.

## Tally (vs Legacy)

| Game | IDENTICAL | RESHUFFLED | SUBSET | SUPERSET | GAME-ONLY | LEGACY-ONLY | rows |
|---|---:|---:|---:|---:|---:|---:|---:|
| nhl10 | 38 | 17 | 14 | 3 | 14 | 62 | 148 |
| nhl11 | 56 | 11 | 14 | 3 | 47 | 50 | 181 |
| nhl12 | 71 | 8 | 8 | 3 | 43 | 44 | 177 |
| nhl13 | 93 | 6 | 2 | 3 | 32 | 30 | 166 |
| nhl14 | 132 | 0 | 0 | 2 | 0 | 0 | 134 |
| nhl15 | 132 | 0 | 0 | 2 | 0 | 0 | 134 |

## Reading the drift

- **NHL Legacy is essentially NHL14/15-vintage.** NHL14 and NHL15 are structurally **identical** to
  Legacy except `ubPc` (Player Bios) and `PxMM` each carry **2 extra fields** (`SUPERSET 0/2`). NHL14
  and NHL15 share the same 134 tables / 2969 fields.
- **Divergence grows going back in time.** NHL13 still shares 93 tables identically; NHL10 shares only
  38 and is missing 62 tables Legacy has (and has 14 of its own).
- This drift map is exactly what the `(title, platform)` schema registry must encode. Tables marked
  `IDENTICAL` can share a schema definition with Legacy; `RESHUFFLED`/`SUBSET`/`SUPERSET`/`*-ONLY`
  tables need per-game definitions.

> The drift is **field-set only**. Bit-depth / rating-scale changes (which exist even within
> `IDENTICAL` tables) are captured per-field in the schema corpus
> ([nhlng-schema.md](nhlng-schema.md)), not here.

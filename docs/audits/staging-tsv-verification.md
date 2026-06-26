# Staging TSV Verification — VERDICT

> **Result: all five staging table-audit TSVs are VERIFIED EXACT.** Every row, category, and count was
> reproduced independently from the anchor `nhlng.db` binaries. They are accurate and safe to ingest.

Per the clean-room "nothing is gospel until verified" rule, the staging files in `Staging/` were
treated as **claims** and checked against ground truth (the binary databases under `anchors/ps3/`),
using a from-scratch TDB v6 parser (see [../formats/tdb-v6-format.md](../formats/tdb-v6-format.md)).

## What the staging TSVs are

Each `Staging/<game>-table-audit.tsv` is a **table-level field-set diff of that game's `nhlng.db`
against NHL Legacy's `nhlng.db`**. Columns: `table_id` (the reversed "real id" convention), `category`,
`c1`, `c2`.

Decoded category semantics (confirmed by reproduction):

| Category | Meaning | c1 | c2 |
|---|---|---|---|
| `IDENTICAL` | same field set as Legacy (bit-depths/order irrelevant) | 0 | 0 |
| `SUBSET` | game is missing fields that Legacy has | # missing | 0 |
| `SUPERSET` | game has extra fields Legacy lacks | 0 | # extra |
| `RESHUFFLED` | field sets differ **both ways** (some missing **and** some extra) | # missing | # extra |
| `<GAME>-ONLY` | table exists only in this game | – | – |
| `LEGACY-ONLY` | table exists only in Legacy | – | – |

> Note: the audit is purely about **field-set membership**. It deliberately does **not** flag
> bit-depth changes or record-layout reshuffles — e.g. NHL10↔Legacy `MkeF` has a differing field
> depth but is still `IDENTICAL` because the field *set* matches. Depth/rating-scale drift is a
> separate concern captured in the full schema corpus ([nhlng-schema.md](nhlng-schema.md)).

## Verification result

| Staging file | Rows | Reproduced | Verdict |
|---|---:|---:|---|
| `nhl10-table-audit.tsv` | 148 | 148 | ✅ EXACT |
| `nhl11-table-audit.tsv` | 181 | 181 | ✅ EXACT |
| `nhl12-table-audit.tsv` | 177 | 177 | ✅ EXACT |
| `nhl13-table-audit.tsv` | 166 | 166 | ✅ EXACT |
| `nhl15-table-audit.tsv` | 134 | 134 | ✅ EXACT |

Zero discrepancies across all five files. The reproduction is driven by a parser independently
validated to extract field sets with 100% agreement vs the meta XML (see
[../formats/tdb-v6-format.md](../formats/tdb-v6-format.md#validation)).

## Ingestion

- The five TSVs are **accurate** and may be ingested. The canonical, machine-readable form is
  `data/drift-matrix.csv`, which covers **all seven games** (it additionally includes **nhl14**,
  which had no staging TSV — nhl14 is `IDENTICAL` to Legacy except `ubPc`/`PxMM` `SUPERSET 0/2`).
- The raw `Staging/` files are left in place; the verified, extended form is the committed
  `docs/audits/` corpus.

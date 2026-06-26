# Audits

Verified findings from the pre-engine audit of the `anchors/` data and the NHLViewNG EXE. Everything
here was independently reproduced from ground truth (the anchor binaries); nothing was taken from a
staging file or reference doc without verification.

## Documents

| Doc | What it establishes |
|---|---|
| [staging-tsv-verification.md](staging-tsv-verification.md) | **Verdict: all 5 staging TSVs verified EXACT** against the binaries |
| [schema-drift-matrix.md](schema-drift-matrix.md) | Field-set drift of all 7 games vs Legacy (extends the TSVs to nhl14) |
| [nhlng-schema.md](nhlng-schema.md) | Verified field-level schema corpus (7 games) overview |
| [anchors-manifest.md](anchors-manifest.md) | Inventory of every anchors file (sha256) |
| [nhlviewng-internals.md](nhlviewng-internals.md) | Static EXE audit; cross-references the functional report |

Binary format specs produced by the audit live under [../formats/](../formats/): the verified
[TDB v6 format](../formats/tdb-v6-format.md), [PS3 save container](../formats/ps3-save-container.md),
and [metadata XML](../formats/metadata-xml.md).

## Machine-readable data ([`data/`](data/))

| File | Contents |
|---|---|
| `schema/<game>-schema.json` (×7) | per-game field-level schema (id, depth, + meta enrichment) |
| `drift-matrix.csv` | all 7 games × tables, drift category + counts vs Legacy |
| `anchors-manifest.csv` | every anchors file: path, platform, game, role, bytes, sha256 |
| `savefile-inventory.csv` | every PS3 save: type, title, container magic, layout, inner-TDB flag |

## Method

A from-scratch, read-only TDB v6 parser (the namespace reversal, header, directory, field-count at
`+0x1c` LE, and 16-byte descriptors) was validated to **100% agreement** with the meta XML on table
sets, field sets, and bit-depths before any finding was recorded. Analysis tooling was throwaway
(scratchpad, not committed) per the audit plan; only verified docs + data are committed.

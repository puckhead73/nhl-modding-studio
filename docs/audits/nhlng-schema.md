# Per-Game Schema Corpus (`nhlng.db`)

> Verified field-level schema extracted directly from the anchor `nhlng.db` binaries for all 7 PS3
> games, enriched with `nhlng-meta.xml` where present. Machine-readable:
> [`data/schema/<game>-schema.json`](data/schema/).

## Coverage

| Game | Tables | Fields | Meta XML | Notes |
|---|---:|---:|:---:|---|
| nhl10 | 86 | 1769 | ✗ | binary-only (no meta ships); ids only, names borrowed from nhl11 where matched |
| nhl11 | 131 | 2514 | ✓ | `DBOFIELDTYPE_*` type names |
| nhl12 | 133 | 2593 | ✓ | |
| nhl13 | 136 | 2843 | ✓ | |
| nhl14 | 134 | 2969 | ✓ | `FIELDTYPE_*` |
| nhl15 | 134 | 2969 | ✓ | identical structure to nhl14 |
| nhllegacy | 134 | 2965 | ✓ | the drift baseline |

## JSON shape

```json
{ "game":"nhl14", "platform":"ps3", "db":"nhlng.db", "version":6,
  "table_count":134, "meta_present":true,
  "tables": {
    "ubPc": { "raw":"cPbu", "name":"playerbiotable", "field_count":55,
      "fields":[ { "id":"dXSB", "raw":"BSXd", "depth":8,
                   "name":"Team ID", "type":"INTEGER",
                   "rangelow":"0", "rangehigh":"...", "key":false, "update":true,
                   "fk":"<parent table>"? } , ... ] } } }
```

- `id`/`raw` = the field id in "real" (reversed) and raw conventions.
- `depth` = bit width, taken from the binary (verified vs meta).
- `name`/`type`/`range*`/`key`/`update`/`fk` come from `nhlng-meta.xml` (absent for nhl10).

## Provenance & trust

- Field id + bit-depth are from the **binary** (the arbiter), validated 100% vs the meta XML for
  nhl11–legacy (see [../formats/tdb-v6-format.md](../formats/tdb-v6-format.md#validation)).
- Type/range/name/key/update/FK are from the **meta XML** (authoritative, present nhl11–legacy).
- The descriptor's record-layout bytes (offset/type packing) are **not** yet decoded; only `depth` is
  used from the binary descriptor. This is the one open structural item.

## Use

This corpus is the verified input to the engine's `FieldMetaIndex` / `(title, platform)` schema
registry (see [../EDIT-MODEL.md](../EDIT-MODEL.md) and [../DESIGN.md](../DESIGN.md)). It is **not** the
final engine format — it is the verified source the registry is built from.

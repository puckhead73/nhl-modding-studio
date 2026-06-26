# Metadata / Definition XML Formats (verified)

> **Verification status: VERIFIED.** The per-game `nhlng-meta.xml` was confirmed to match the binary
> `nhlng.db` exactly (table set + field set + bit-depth, 100%). This is the **authoritative FieldMeta
> source** the engine ingests (per [../EDIT-MODEL.md](../EDIT-MODEL.md)).

Two distinct XML families ship with the games / reference data. They use the **raw** id convention
(see [tdb-v6-format.md](tdb-v6-format.md#the-id-reversal-rule-read-first)).

## 1. `nhlng-meta.xml` — the schema / FieldMeta backbone (authoritative)

Present in `anchors/ps3/<game>/databases/` for **nhl11→legacy** (nhl10 ships **none**).

```xml
<database name="nhlng_game" shortname="emag" version="6">
  <indices> <index .../> </indices>
  <table name="<human>" shortname="<raw4>" maxupdate maxinsert maxdelete savegroups="...">
    <fields>
      <field name="<human>" shortname="<raw4>" type="FIELDTYPE_INTEGER|FIELDTYPE_STRING"
             depth="<bits>" null="True|False" rangelow="N" rangehigh="N"
             key="True?" update="True|False?" fkparenttable="<table name>?" />
    </fields>
  </table>
</database>
```

Per-field this yields the full `FieldMeta` `{ id, name, type, bit_depth(=depth), range_low, range_high,
updatable(=update), key, fk_parent, save_groups(table-level) }`. Field counts derive directly.

**Year drift to handle in ingestion:**
- nhl11 uses `DBOFIELDTYPE_*` type names + extra DML shortnames (`insertshortname`, …); nhl14+ use
  `FIELDTYPE_*`. Same hierarchy, different type-name prefix.
- **nhl10 has no meta** → its schema (table+field+depth) comes from the binary only; borrow
  names/types from nhl11 by matching ids, and mark the rest unnamed.

**Trust the binary, not the XML, on disagreement.** The Reference-Data `nhlng-meta (DB Converter).xml`
is a near-identical snapshot that differs from the anchors copy by a handful of fields — the anchor
binary is the arbiter.

## 2. UI / editor definition XMLs (names + control types, NOT schema)

In `…/Reference Data/Database Definitions/`: `NHL 14 (EA DB Editor).xml`,
`NHL 14-Legacy (DB Patcher).xml`, `season-config-nhl14-legacy.xml`. Root `<xml>` with `<View>` (grid
layouts), `<Table>`/`<Field>` mappings:

```xml
<Table><Abbreviation>ubPc</Abbreviation><Name>Player Bios</Name></Table>
<Field><Abbreviation>dXSB</Abbreviation><Name>Team ID</Name><ControlType>TextBox</ControlType></Field>
```

These use the **reversed "real id"** convention (`ubPc`, `dXSB`) and provide **human display names,
`ControlType` (TextBox/Dropdown), and grid column order** — but no type/depth/range. Use them as a
secondary enrichment layer (UI hints) merged on top of the `nhlng-meta.xml` schema.

> **Disputed claim resolved:** the functional report stated the EA DB Editor XML encodes the eGlu
> 19-line-group layout. Reconnaissance found it encodes `View`/grid column order and name/control-type
> mappings, **not** an explicit line-group enumeration. The line-group flags live in the `eGlu` table's
> field values, not in this XML. Treat the line-group layout as data-driven, to confirm against
> records.

## 3. `nhl_eng_us-meta.xml` — localization schema (supplementary)

Tiny (one `LanguageStrings` table, `hashid`/`stringid`/`sourcetext`); describes the
`nhl_eng_us.db` localization database. Ingest separately if/when localization editing is built.

## Ingestion summary

Primary source = per-game `nhlng-meta.xml` → `FieldMetaIndex` keyed by `(game, table, field)`.
Enrich with the EA DB Editor XML for human names + control types. For nhl10, synthesize from the
binary + nhl11 id matches. The verified per-game corpus is `../audits/data/schema/<game>-schema.json`.

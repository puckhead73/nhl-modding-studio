# TDB Metadata XML

**Evidence:** measured: every title's metadata file was checked against its database (table set, field set and bit depth match 100% on every anchor that ships one).

Each title ships an XML file beside its main database that describes every table and field: `nhlng-meta.xml` on Xbox 360 and PlayStation 3, `nhl_game-meta.xml` on PlayStation 4, and a tiny `nhl_eng_us-meta.xml` or `eng_us-meta.xml` for the dictionary. The binary database carries only ids and widths; the XML adds names, types, ranges, keys and foreign keys. It is the backbone of any metadata-driven editor.

## Shape

```xml
<database name="nhlng_game" shortname="emag" version="6">
  <table name="exhibitionplayerbiotable" shortname="cPbu"
         maxupdate="…" maxinsert="…" maxdelete="…" savegroups="…">
    <fields>
      <field name="game_id" shortname="zIBw" type="FIELDTYPE_INTEGER"
             depth="14" null="False" rangelow="0" rangehigh="16383"
             key="True" update="False" fkparenttable="…"/>
    </fields>
  </table>
</database>
```

Per field this yields the readable name, the type (integer or string), the bit depth, the declared range, whether the field is a key, whether the game's own save logic updates it, and for some fields the parent table it references. Table-level `savegroups` says which save types carry the table.

The `shortname` values use the **raw** id convention, byte-reversed relative to the names most modders know; see [TDB Id Reversal and Naming](TDB-Id-Reversal-and-Naming).

## Per-title differences

| Title | Note |
|---|---|
| NHL 10 | ships **no** metadata file. Its schema is read from the binary and names are borrowed from NHL 11 by matching ids; unmatched fields stay unnamed. |
| NHL 11 | type names use a `DBOFIELDTYPE_` prefix and tables carry extra DML short names (`insertshortname` and similar). NHL 14 onwards uses `FIELDTYPE_`. |
| PS4 | ids in the XML are un-reversed, matching the little-endian binary. The secondary databases (`entry_draft.db` and others) ship no XML of their own; they are strict subsets of the main schema and validate against `nhl_game-meta.xml`. |
| Patches | PS4 patch archives carry no metadata; the base title's file applies. |
| NHL 07 | has its own `NHL07NG-meta.xml` for its distinct gen6 schema. |

The Xbox 360 and PlayStation 3 metadata of the same title are byte-identical wherever both exist.

**Trust the binary on disagreement.** A near-identical snapshot of the NHL 14 metadata distributed with a community database converter differs from the shipped file by a handful of fields; the shipped database is the arbiter.

## Declared ranges are not validity

A declared range is what the field can *encode*, not what the game accepts. Birth months are declared 0 to 15 on most titles though only twelve values are dates; see [Field Encodings](Field-Encodings). An editor has to apply calendar and domain rules above the declared range.

## Editor definition files

Community tools also distribute an "EA DB Editor" XML keyed by the reversed ids, which supplies display names, control types (text box, dropdown) and grid column order, but no types, depths or ranges. It is a presentation layer on top of the schema file, not a schema. Contrary to an earlier reading, it does not encode the roster line-slot layout; that lives in the roster table's own field values (see [Roster Model](Roster-Model)).

Last reviewed: 2026-10-20

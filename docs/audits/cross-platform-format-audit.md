# Cross-Platform Format Audit (Xbox360 + PS4)

> **Verification status: VERIFIED.** The TDB v6 read+write core extends across the whole matrix.
> Checked the newly-added Xbox360 (nhl10–15+legacy) and PS4 (nhl19/20/21; nhl18 saves only) anchors
> against the verified spec ([../formats/tdb-v6-format.md](../formats/tdb-v6-format.md)).

## Headline

| Platform | Databases | Save container |
|---|---|---|
| **Xbox360** | **Byte-identical to PS3** (same sha256, all 7 games) — covered for free | **Net-new** (different format) |
| **PS4** (19–21) | **LE TDB v6** — same format/CRC, endian-aware; **new schemas + new file set** | **Net-new** (Sony save dir) |

## Xbox360 databases = PS3 databases

The Xbox360 `nhlng.db` (and siblings) are **byte-for-byte identical to the PS3 copies** for all seven
games (verified by sha256). EA shipped the same database content on both big-endian consoles; the
platform difference lives entirely in the **save container**, not the database. So all our verified
PS3 schema work (corpus, drift, relationships, enums) applies to Xbox360 unchanged.

## PS4 databases = little-endian TDB v6

PS4 (x86) uses the **little-endian** TDB v6 variant — the spec's endian byte `[4]` is `0` (vs `1` on
PS3/Xbox), and the **4-char ids are stored un-reversed** (e.g. `dSvy`, `GJCv` appear directly), exactly
as the spec's "reverse only on big-endian" rule predicts. Confirmed against the anchors:

- Every PS4 `.db` parses; **`db_size` and the header CRC (CRC-32/MPEG-2, stored LE) verify**.
- `nhl_game.db` (the main DB): **111 tables (nhl19/20), 115 (nhl21)** — a *different, newer* schema than
  PS3's `nhlng.db`. Cross-checked nhl19 `nhl_game.db` vs `nhl_game-meta.xml`: **111/111 tables, 100%
  field-set match**, meta using the same un-reversed ids.
- **Different DB file set** than PS3: `nhl_game.db`, `eng_us.db` (localization; table `GJCv`, same as
  PS3's), `entry_draft.db`, `expansion_draft.db`, `initial_experience.db`, `threes_campaign.db`,
  `proam.db` (nhl20), + `nhl_game-meta.xml` / `eng_us-meta.xml`.

So the verified TDB core (header, directory, descriptors, record decoding, all CRC scopes, Huffman) is
**endianness-parametric and covers PS4 reading + writing** — only the per-game schema corpus is new.

## Save containers — net-new (the remaining reverse work)

Neither platform uses PS3's `PS3<Type>File` framing:

- **Xbox360**: single save file per type with its own ASCII magic — e.g. `Dynasty`, `BeAPros` (vs PS3
  `PS3DynastyFile`/`PS3BeAProFile`). Likely a CON/STFS-extracted blob wrapping the TDB; header/checksum
  layout not yet reversed.
- **PS4**: each save is a **Sony save directory** — `DATA` / `INDEX` / `NAME` / `sce_sys/` — with the
  `DATA` blob carrying JSON-ish metadata (`{"IsBackup"…}`) plus the payload. A different, more involved
  container (PS4 save-data structure); not yet reversed.

Both are deferrable: they don't block DB editing on the proven titles. They become relevant for
save read/write on those platforms.

## Coverage status after this audit

| | DB read | DB write (CRC) | Schema corpus | Save container |
|---|---|---|---|---|
| PS3 nhl10–15+legacy | ✅ | ✅ | ✅ | ✅ (v2/v4) / v1 read-only |
| Xbox360 nhl10–15+legacy | ✅ (=PS3) | ✅ | ✅ (=PS3) | ⬜ net-new |
| PS4 nhl19/20/21 | ✅ | ✅ | ⬜ to extract | ⬜ net-new (Sony dir) |
| PS4 nhl18 | — (no DBs added) | — | — | ⬜ (saves only) |
| NHL16; NHL17 | ⬜ no anchors | | | |

## PS4 schema corpus — extracted & verified

The PS4 field-level corpus is generated for nhl19/20/21: **17 schema files** under
`data/schema/ps4/<game>-<db>.json`, covering `nhl_game.db` (111 tables / 3156 fields; nhl21 = 115 /
3265), `eng_us.db`, and the secondary DBs (`entry_draft`, `expansion_draft`, `initial_experience`,
`threes_campaign`, `proam`). Every field matched its meta — `nhl_game.db`/`eng_us.db` against their own
meta XML, and the secondary DBs **100% via borrowed `nhl_game-meta.xml`** (they are strict subsets of
the main schema, ids stable). The **LE record decoder** was validated against the spec's own worked
vector (`read(31,5)==1`) and `position` decodes to the valid 0–4 range.

> **Structural note (PS4):** the base `nhl_game.db` appears to hold **default/template** values —
> `jersey` is uniformly 19, `overall` uniformly 50 across rows. So unlike PS3 (`nhlng.db` *was* the
> roster), the **live edited roster on PS4 likely lives in the save data**, with `nhl_game.db` as the
> base/template. Confirm when the PS4 save container is reversed.

## Next steps

1. **Reverse the two save containers** (Xbox360 magic-blob; PS4 Sony save dir) — needed for save
   read/write, and (per the note above) likely where the PS4 live roster actually lives.
2. NHL16 (any platform) and NHL17 (PS4) still have **no anchors**; NHL18 PS4 is saves-only.

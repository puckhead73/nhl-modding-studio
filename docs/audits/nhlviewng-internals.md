# NHLViewNG Internals Audit (static analysis)

> Static analysis of `nhlviewng_unpacked.exe` (6,308,864 bytes, PE32 i386 Delphi/VCL, built
> 2022-11-12), authorized by the user as an explicit clean-room exception. Read-only; the EXE was
> never executed. This **confirms and extends** the prior
> [nhlviewng-functional-report.md](nhlviewng-functional-report.md).

## PE structure

- PE32 / i386 GUI, **not packed** (.text entropy ~6.5). `.rsrc` ≈ 695 KB (VCL `.dfm` forms, dialogs,
  icons). ~102,750 ASCII + ~3,100 UTF-16LE strings. No authenticode signature.
- Framework: Delphi VCL + `Data.DBX`, `System.Rtti`. Internal product version string `6.0.0.0`.

## Database / TDB handling (matches our verified TDB v6 spec)

- Classes `TtdbDatabaseHeader`, `TtdbMetaTable`, `TtdbMetaField`, `TtdbMetaIndex` — the in-memory
  relational meta-model (consistent with our reversed header → directory → field-descriptor layout in
  [../formats/tdb-v6-format.md](../formats/tdb-v6-format.md)).
- Integrity: `CheckCRC`, `DBHeadHash`, `CheckMagicForCompressed` → confirms the head/table hashes are
  CRC-family (we verified their location; algorithm still to reverse for writes).
- Compression: `Compress`/`Decompress`, `CompressedVarchar`, `CompressionSizeTrigger`, and Huffman
  (`THuffmanTreeInternalNode`, `uAbstractTDBHuffman`). **`CompressionSizeTrigger` directly explains
  our container finding** that NHL14+ large save payloads are compressed while small ones stay
  plaintext (see [../formats/ps3-save-container.md](../formats/ps3-save-container.md#plaintext-vs-compressed-payload-key-boundary)).
- Game coverage via RTTI: `TNHLElement*NG2008..2016` (9 years) → NHLViewNG spans **NHL 09–16**.

## Container / crypto

- PS3: `uPS3Pfd`, `TPS3ProtectedFile`, `TPfdKey` (PFD decrypt/encrypt). Xbox360: STFS / `CON`. PS4:
  `actFileOpenPS4`. `.bak` backups, `AutoSave`, `SYS-DATA`, `DATA000`.
- Crypto: `TAESAbstract`, PKCS7, IV/CTR with replay detection — used for console save de/encryption.
  (Our anchors are already-decrypted exports, so this path was not needed for the read audit.)

## Edit / validation model (matches EDIT-MODEL.md)

- Actions: `actDatabaseCreatePlayer`, `actDatabaseBulkAttributeAdjust`, `actDatabaseFind`,
  `actDatabaseErrorLog`; trade/move, `LoadRoster`/`LoadFreeAgentsRoster`/`LoadHiddenRoster`, jersey,
  contracts, custom face. 50+ `Plr*` player attributes.
- Validation: rich `E*Error` hierarchy, constraint/range checks, `actDatabaseErrorLog`. Consistent
  with the metadata-validated, entity-first model in [../EDIT-MODEL.md](../EDIT-MODEL.md).

## Cross-reference to the functional report

| Functional-report claim | Audit verdict |
|---|---|
| One binary spans NHL 09–16 (RTTI `TNHLDatabaseNG2008..2016`) | ✅ confirmed (`TNHLElement*NG2008..2016`) |
| TDB format version 6 | ✅ confirmed (`6.0.0.0`; our binaries are `version=6`) |
| Container framing: zlib/compress/CRC/checksum/endian | ✅ confirmed; **+ Huffman + size-triggered compression** (extends report) |
| PS3 PFD de/encrypt, PS4, Xbox360 STFS, `.bak`, AutoSave | ✅ confirmed |
| Metadata-driven editing (`TtdbMeta*`), validation hierarchy | ✅ confirmed |
| EA DB Editor XML encodes the eGlu 19-line-group layout | ⚠️ **disputed** — that XML encodes grid/column order + name/control-type, not an explicit line-group list (see [../formats/metadata-xml.md](../formats/metadata-xml.md)). Line groups are `eGlu` field values. |

## Scope-relevant gaps

- NHLViewNG covers **09–16**; our target is **10–21**. It writes **v1 (NHL10/11)** and **PS4**
  containers — capabilities our engine must build. Years **17–21** are beyond NHLViewNG entirely
  (net-new; ROADMAP Phase 10).
- Not reversible from strings alone (deferred): exact CRC/Huffman codec tables, AES key derivation,
  STFS layout. These are engine-phase work, not needed for the current read audit.

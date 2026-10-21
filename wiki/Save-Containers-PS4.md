# Save Containers for PS4

**Evidence:** measured on the PS4 reference saves (NHL 15 through 21): the record stream, both payload codecs and the trailers are decoded, and an unedited repack of the NHL 17, 18, 19 and 21 roster saves is byte-identical to the game's file. NHL 16's repack is equivalent but not identical. In-game acceptance of an edited save and the meaning of the record-header tag are open.

A PlayStation 4 save is a folder exported from the console or an emulator: `NAME` (the slot name, such as `RosterCustom0`), `INDEX`, `sce_sys/param.sfo`, `sce_sys/icon0.png` and **`DATA`**, which holds everything editable. The inner database is a **little-endian** TDB v6 (see [TDB Database Format](TDB-Database-Format)).

## `DATA` is a record stream

`DATA` is a sequence of records, each `[u32 little-endian length] [12-byte header] [payload]`. The header begins with constant bytes, carries a per-record id and ends with a 7-byte tag shared by all records of one save; its role is not established and a writer preserves it verbatim. A roster save holds four records: a JSON `{"IsBackup":false}`, the game version string, JSON metadata with a creation date and a `CRCValue` string, and the main payload. The `CRCValue` is not recomputed on write; whether the game checks it is open.

## Payload codecs

| Titles | Main payload |
|---|---|
| NHL 15 | a `chunkzip` stream with a padded native header (version word 2, 128 KiB window) decoding to the TDB |
| NHL 16 | a bare LZ4 frame |
| NHL 17 to 21 | a `01` codec byte, an LZ4 frame, then a little-endian u32 holding the decompressed length |

Every LZ4 frame uses independent 64 KiB blocks with a content checksum and no block checksums. The blocks were produced by LZ4's fast compressor **as it stood before version 1.9.0**; current liblz4 lets a match start one byte closer to the block end and so produces a few blocks 1 to 5 bytes shorter. Reproducing EA's bytes needs the older match-finding rule, which Studio ports. NHL 16 was compressed by an older build still; its blocks differ in both directions and only an equivalent repack is possible.

## What is inside

The NHL 19 roster save inflates to a 54-table TDB, a roster subset of the 111-table main database; the arena table decodes to the 31 NHL cities. Mode saves on NHL 15 and 16 hold their TDB directly. On NHL 17 and later the mode record's payload is **another** `01` + LZ4 frame (nested); NHL 17's inflates to the mode TDB, NHL 18 to 21's to a small section table rather than a TDB. Studio does not read the nested form.

## Trailers after the TDB

The decompressed roster payload can carry bytes after the TDB's declared size:

| Save | Bytes after the TDB | Content |
|---|---|---|
| NHL 17 roster | 20 | a constant, zeros, **a u64 equal to the TDB size**, then `02 00 01 00` |
| NHL 18 and 19 roster | 18 | the same with a `01 00` tail |
| NHL 18, 20, 21 3-on-3 | 15 to 17 k | JSON (`{"DefaultPlayers":…`) |
| NHL 15 and 16 mode saves | 160 to 370 k | mostly zero padding |
| NHL 15 and 16 roster | 0 | |

The embedded size equals the TDB size on all three roster anchors. An editor must split the trailer off at the TDB's declared size, edit only the TDB (so every CRC including the end-of-file one is computed at the TDB's own end) and re-append the trailer. When an edit changes the TDB's length, the u64 is rewritten only when the trailer has exactly this form; any other trailer refuses a size change. The tail bytes' meaning is open.

## Companion files

Updating `INDEX` and `sce_sys/param.sfo` after an edit has not been required by any observation but is not established either. No console-side encryption was observed on these exported saves; real-console write-back is a separate question.

Last reviewed: 2026-10-20

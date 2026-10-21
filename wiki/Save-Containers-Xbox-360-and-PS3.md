# Save Containers for Xbox 360 and PS3

**Evidence:** measured on the committed save fixtures of every title (121 saves across platforms and types). Both checksums are reproduced on every v2, v3 and v4 save; an unedited repack of every roster save is byte-identical, including the zlib variant since the compressor settings were matched. In-game acceptance of an edited save is open for every title; the NHL 10 and 11 tag is partly open.

PlayStation 3 and Xbox 360 saves wrap a TDB in a small container with two CRC-32 tags. The two platforms use the **same container**; only the magic string and a marker word differ. The container does not encrypt anything: console-sealed PS3 saves must first be exported in plain form (an emulator does this), and Xbox 360 saves must be taken out of their outer package. Both outer layers are outside what this page covers.

## Save directories and types

A PS3 save is a folder named `<product code><n> - <TYPE>` holding `PARAM.SFO`, `ICON0.PNG` and the payload, either one `SYS-DATA` file or several `DATA0000…` parts. An Xbox 360 save is a single file named after its type.

| Type | PS3 magic | Xbox 360 magic | Contents |
|---|---|---|---|
| ROSTER | `PS3RosterFile` | `RosterFile` | the roster TDB; the primary editing target |
| SETTINGS / PROFILE | `PS3UserProfileFile` | `UserProfileFile` | a 132-byte TDB plus about 1 MB of profile data |
| BEAGM / DYNASTY | `PS3DynastyFile` | `Dynasty` | franchise mode state |
| BEAPRO | `PS3BeAProFile` | `BeAPros` | career mode state |
| SEASON | `PS3SeasonFile` | `SeasonFile` | season mode state |
| HOCKEYCARD | `PS3HockeyCardFile` | `HockeyCards` | card images; no TDB |

Mode saves carry their mode's tables (33 to 43 of them) in one TDB and **no roster**; the roster lives only in the ROSTER save and the main database. BEAGM, BEAPRO and SEASON become multi-part `DATA0000…` files from NHL 13 on PS3; multi-part reassembly is **not** implemented by Studio and those saves are read-only.

## The three header variants

The variant is decided by the **body marker**, never by the version word, which varies by title *and* save type (words 1, 3, 5 and 6 all occur on 44-byte raw headers and on zlib bodies).

| Variant | Header | Titles (roster saves) | Body |
|---|---|---|---|
| RawV3 | 32 bytes | NHL 07, 08, 09 | raw TDB; NHL 08 and 09 carry a gen7 TDB inside a v3 container; NHL 07 and 08 roster saves are change lists against the game database rather than full rosters |
| RawV1 | 32 bytes | NHL 10, 11 | raw TDB at 0x20 |
| RawV2 | 44 bytes | NHL 12, 13 | raw TDB at 0x2C, written tight |
| ZlibV4 | 48 bytes | NHL 14, 15, Legacy | zlib stream at 0x30, zero-padded |

**Common header fields:** 16-byte NUL-padded magic at 0; `checksum_a` at 0x10; the version word at 0x14; flags at 0x18. RawV1 adds `tag_1c` at 0x1C. RawV2 and ZlibV4 add two words at 0x1C and 0x20 (the latter a platform marker: 1 on Xbox 360 and some PS3 saves, 2 on other PS3 saves, so not reliable), `uncompressed_size` at 0x24 and `checksum_b` at 0x28. ZlibV4 adds a **little-endian** `compressed_size` at 0x2C, the only non-big-endian header field.

## Checksums

| Tag | Algorithm | Covers | Stored |
|---|---|---|---|
| `checksum_a` (0x10) | CRC-32/IEEE, reflected, polynomial 0xEDB88320, final XOR 0xFFFFFFFF | `file[0x1C..EOF]` including padding, the stored `checksum_b` and any mode sections | big-endian |
| `checksum_b` (0x28) | CRC-32/BZIP2, polynomial 0x04C11DB7, non-reflected, final XOR 0xFFFFFFFF | `file[0x2C .. 0x2C + uncompressed_size]`, the payload region only | big-endian |

On a roster save the payload region is the whole rest of the file, so the two rules coincide. Mode saves carry further sections after the payload (a block list beginning `00000002 00000001 <len> <tag>`) covered by `checksum_a` alone. Recompute `checksum_b` first, then `checksum_a`.

**RawV1 `tag_1c`** (NHL 10 and 11) is partly reversed: on BEAPRO and SETTINGS saves it is CRC-32/MPEG-2 over `file[0x20..EOF−4]`; on ROSTER, BEAGM and SEASON saves it matches no CRC of the file, and the PS3 and Xbox 360 roster saves carry the same tag over different bodies. Writers preserve it verbatim; whether the game checks it is open.

## The zlib body

ZlibV4 bodies are standard zlib (RFC 1950) at level 6, window 15 and memory level 9. With those parameters a repack of every v4 roster anchor is byte-identical to the game's file. The `uncompressed_size` word equals the inflated payload on roster saves and exactly **half** of it on every NHL 14, 15 and Legacy mode save, for reasons not established; a writer keeps a mode save's declared region.

Even a ROSTER payload can carry a few bytes after the inner TDB's declared size (Legacy: four non-zero bytes). A writer must preserve everything after the database, not just the database.

## Settings saves and version 5

PS3 SETTINGS saves on NHL 10, 11, 14 and 15 carry version word 5 and are refused by the Studio's container reader; they are outside the editing scope. There is no "version 5 format": the word is per title and per type.

## Game-specific roster saves

- **NHL 07 and NHL 08** roster saves are **change lists**: per table, insert, delete and update companions keyed to the game's database. An empty change list shows the database unchanged; an edit is written as a delete-and-insert pair where the table has both companions or as an update row otherwise. The writer reproduces the game's own saved files byte for byte on NHL 07.
- **NHL 09 onwards** roster saves hold a full TDB subset of the main database, record-index-aligned, so an edit made to the database can be mirrored into the save by the same table, record and field.

Last reviewed: 2026-10-20

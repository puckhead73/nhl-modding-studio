# Archives and Compression

**Evidence:** measured on the installed archives of every title (9 BIG4 archives with their `.bh` sidecars, 118 EB archives across Xbox 360, PS3 and PS4) and on 50 `chunklzx` and 240 `chunkzip` members decoded byte-identically to loose files. Whether the games' loaders honour a modified archive's size word or an appended entry is open.

The games pack most of their assets, and from NHL 11 on their databases too, into `.big` archives. There are three archive directory formats, two chunked compression wrappers and one older stream compressor. All of them are plain containers: no encryption anywhere.

## Archive directory formats

### BIG4 with Viv4 `.bh` sidecar (Xbox 360 NHL 07 to 10, PS3 NHL 08 to 10)

- Header: magic `BIG4`, a little-endian u32 size equal to the file length, a big-endian count and index size.
- Index: names with offsets and sizes, followed by an 8-byte trailer (`L277` on NHL 07 and 08, `L286` on 09 and 10, plus `15 05 00 00`); NHL 07 and 08 then carry an opaque text string before the first entry.
- Entries are **64-byte aligned**, in index order, with zero gaps (one unexplained non-zero gap byte in NHL 09's `boot.big`, preserved).
- Beside every archive sits a **`.bh` file** (`Viv4` magic) repeating each member's offset, stored size, decoded size (for RefPack members; 0 for raw) and a djb2 hash of the path, with a 16-byte trailer holding the archive length. A size change must patch **both** files, and on NHL 07 also the copies of the `.bh` files packed inside `boot.big`.

### EB 0x00000400, 16-byte rows (Xbox 360 NHL 11 to Legacy, PS3 NHL 12 to Legacy, Recompiled)

- Header: magic `EB\0\3`, file count, version tag 0x00000400, name-table offset, slot widths and a u64 archive size.
- Rows: `[offset in 16-byte units][zero][stored size][djb2 of the path]`.
- Payloads are 16-byte aligned, table order equals offset order, gaps are zero, no shared spans; `archive_size` equals the file length (aligned up to 16 after the last payload).

### EB 0x00100400, 16-byte rows (PS4 NHL 15)

Same row layout but the table is **sorted by the hash word**, not by offset. On 3 of the 17 archives the declared size is **smaller** than the file: the entries beyond the declared size start exactly there and are packed to the end of the file, and the archives contain large zero holes inside the declared region.

### EB 0x00110400, 20-byte rows (PS4 NHL 16 to 21)

- Rows add a fourth word: `[offset/16][zero][stored size][sort key][djb2 of the path]`. The sort key is path-derived and its formula is **open** (siblings differing in a trailing character share it); a same-path replacement preserves it verbatim.
- Name table: fixed-width records of `u16 directory index` plus a NUL-terminated basename; directory table: NUL-terminated paths. Full path = directory + `/` + basename.
- **Deduplication is the norm**: identical payloads are stored once and referenced from many paths (NHL 20: 27 872 shared rows across the install). Most league-logo members on NHL 17 to 21 share bytes with their French twin and with other league ids. Replacing one logo therefore means appending a private copy and re-pointing only the chosen path.
- One shipped archive (NHL 16 `game_90.big`) also has a declared size below its length, with three appended entries. Dead zero space with a correct size word is ordinary in shipped archives.

The 17 GB stadium archive on NHL 21 requires a streaming reader; the front tables are small.

## Payload codecs

| Codec | Where | Shape |
|---|---|---|
| **RefPack** | front-end art packages (`BIGF`) on Xbox 360 and PS3 NHL 07 to Legacy; some `.bh` copies | EA's classic LZ77 stream with a declared decoded length. Studio's encoder produces streams 0.5 to 4% longer than EA's on every one of 38 stock members, so a re-encoded member usually, but not always, changes placement. |
| **chunkzip** | PS3 archives, PS4 archives, NHL 15 PS4 roster save | magic `chunkzip`; chunks of 128 KiB compressed with standard zlib at level 9; a chunk that does not shrink is stored uncompressed. EA's members on PS3 and PS4 (240 of 240) are reproduced byte for byte by this recipe. Two header variants (plain and padded). |
| **chunklzx** | Xbox 360 EB archives (NHL 11 to Legacy): rink logos, ice, jersey and puck texlibs, attribute databases | magic `chunklzx`; big-endian header with `fullsize`, a 0x40000 chunk size and chunk count; data from offset 40. Each chunk header sits at an offset congruent to 8 modulo 16 (`((end − 8 + 15) & ~15) + 8`) and holds `zsize` and a flag (3 observed). The payload is a sequence of sub-frames `[u16 csize][bytes]` of 32 768 output bytes each, a last frame `[0xFF][u16 usize][u16 csize][bytes]`, then 5 zero bytes. The concatenated sub-frames of one chunk form **one LZX stream with window bits 17**, an Intel E8 header bit and 16-bit realignment at each frame end; the header's window word is the chunk size, not the LZX window. Decoded output equals `fullsize` exactly. PS3 never uses it. No encoder exists in Studio; writing a `chunklzx` member means substituting `chunkzip`, whose runtime acceptance is open. |

## Inner packages (`BIGF`)

League logos and other front-end art sit in small `BIGF` packages inside the archives with a fixed member layout: 64-byte alignment, a 16-byte tail not counted in the package's own size word, and an opaque 8-byte tag. Rebuilding such a package from scratch changes bytes it should not; shifting the members after a replaced texture is fully determined.

## Replacing a member

What a safe repack must do depends on the family:

1. **Same length, same path:** overwrite the payload bytes in place. Works on every family; the `.bh` sidecar and the PS4 hash words need no change.
2. **Different length on BIG4:** re-plan the layout at 64-byte alignment, rewrite the index and the `.bh`, update the size word.
3. **Different length on EB:** place the new payload (in the member's slot if the gap bytes are verified zero and it fits, otherwise appended), update the row, keep the hash words, and update or preserve the size word according to that version's rule. For 20-byte-row archives, give a shared payload a private copy first.
4. **Disc images** (ISO) stay same-length only.

Gap bytes are never free capacity unless read and verified zero for that slot.

## What is not an archive

Loose extracted trees (what Studio produces with "Extract game containers") are the archives' contents written out as files. On Xbox 360 the loader may read loose files from a `cache:` location, on the Recompilation from a user folder convention, and on PS4 the base and patch archives both hold copies of the databases; which copy a game loads when both exist is **open** for every platform and is exactly the kind of question only an in-game test settles.

Last reviewed: 2026-10-20

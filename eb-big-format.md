# The EA "EB" `.big` archive format

As used by NHL 19 and NHL 21 on PS4. Everything here is ground-truthed against real archives from
`CUSA18182` (NHL 21) and `CUSA11126` (NHL 19), and against a pristine dump of NHL 21 kept at
`references/Unedited 21 Files/`.

Sections are marked **verified** (reproduced from bytes), **observed** (consistently true across every
archive we have, mechanism unconfirmed) or **unresolved**.

**All multi-byte integers are big-endian.**

---

## 1. Layout at a glance

```
0x00                header (0x20 bytes)
0x30                file table        FileCount x 16 or 20 bytes
NameTableOffset     name table        FileCount x FileRecordLength
(aligned to 0x10)   folder table      FolderCount x FolderRecordLength
...                 file data         addressed by the file table, in BlockSize units
EOF
```

The three tables are walked in **lockstep**: row *i* of the file table and record *i* of the name
table describe the same file. There is no key linking them; order is the only association.

---

## 2. Header — verified

Fixed 0x20 bytes at offset 0.

| Offset | Type | Field | Notes |
|---|---|---|---|
| `0x00` | char[2] | Magic | `"EB"` (`45 42`) |
| `0x02` | u16 | Version | `3` in every NHL 19/21 archive |
| `0x04` | u32 | FileCount | |
| `0x08` | u32 | Flags | see below |
| `0x0C` | u32 | NameTableOffset | |
| `0x10` | u32 | NameTableSize | |
| `0x14` | u8 | FileRecordLength | bytes per name-table record |
| `0x15` | u8 | FolderRecordLength | bytes per folder-table record |
| `0x16` | u16 | FolderCount | |
| `0x18` | **u64** | ArchiveSize | must equal the real file length |
| `0x20`–`0x2F` | — | reserved / padding | |
| `0x30` | | file table begins | |

`ArchiveSize` being a u64 matters: `game_90.big` is 17 GB, well past a u32.

### Flags

| Bit | Name | Effect |
|---|---|---|
| `0x00000200` | LargeBlockSize | block size becomes `0x40`; otherwise `0x10` |
| `0x00010000` | ExtraFileField | each file-table row carries a 5th u32 |

Every NHL 21 archive has `Flags = 0x00110400` → block size **0x10**, extra field **present**, so rows
are **20 bytes**. Bits `0x100000` and `0x400` are set in every archive and their meaning is unknown.

### Real headers (NHL 21)

| Archive | Files | Folders | FileRecLen | FolderRecLen |
|---|---|---|---|---|
| `boot.big` | 7,365 | 1,533 | 61 | 83 |
| `game_60.big` | 10,375 | 2 | 14 | 42 |
| `game_90.big` | 92,322 | 92 | 86 | 63 |
| `inxp_00.big` | 2,950 | 152 | 70 | 105 |
| `inxp_50.big` | 12,336 | 34 | 66 | 50 |
| `overlays_20.big` | 6 | 2 | 36 | 49 |

Record lengths are per-archive: they are sized to the longest path each archive happens to contain.

---

## 3. File table — verified

Starts at `0x30`. Rows are 20 bytes when `ExtraFileField` is set, otherwise 16.

| Offset in row | Type | Field | Notes |
|---|---|---|---|
| `+0` | u32 | **OFFSET** | in **blocks**, not bytes — multiply by `BlockSize` |
| `+4` | u32 | **ZSIZE** | compressed size; `0` in practice throughout NHL 21 |
| `+8` | u32 | **SIZE** | the entry's span on disk |
| `+12` | u32 | CRC | **not a data checksum** — see below |
| `+16` | u32 | EXTRA | present only with the flag; purpose unknown |

### The span rule

An entry occupies `ZSIZE` bytes when `ZSIZE != 0`, otherwise `SIZE` bytes. Since NHL 21 stores `0` in
`ZSIZE` everywhere, **`SIZE` is the on-disk span**, and the payload it points at is usually
*chunkzip-compressed* — so `SIZE` is emphatically not the decompressed length.

### The `CRC` field is not a CRC — verified

Rows 2, 3, 4 and 5 of `game_90.big` all store `0x0000718C` despite covering different data of
different lengths (2,281 / 2,497 / 2,574 / 2,378 bytes). Rows 0 and 1 both store `0x000002B3`. It is
constant within groups, so it is a group or category id, not a checksum. No CRC-32 variant (forward
`0x04C11DB7`, reflected `0xEDB88320`, Adler-32) reproduces it.

**Consequence: a writer does not need to recompute it.** This was suspected as a cause of corrupted
edits and ruled out.

`EXTRA` increments by a constant 1,185,921 between consecutive rows, which looks like an ordering key.
It does not change when an entry is rewritten in place or relocated.

---

## 4. Name and folder tables — verified

At `NameTableOffset`, one record per file, each `FileRecordLength` bytes:

```
u16   folder index -> into the folder table
char  name[FileRecordLength - 2]   fixed width, NUL padded
```

The folder table follows the last name record, **rounded up to a `0x10` boundary**, and holds
`FolderCount` fixed-width strings of `FolderRecordLength` bytes each.

A file's full path is `folders[folderIndex] + "/" + name`.

---

## 5. Payload and `chunkzip` — verified

Almost every entry's payload begins with the 8-byte ASCII tag `chunkzip`. In `game_90.big`, 92,318 of
92,322 entries are chunkzip; 4 are stored raw with no tag.

```
"chunkzip"                    8 bytes
[probe u32]                   if <= 2 this is a TYPE 0 header, and fullSize follows
fullSize u32                  decompressed length
chunkSize u32                 decompressed bytes per chunk
[chunkCount u32 + 16 reserved]   TYPE 0 only; otherwise derived as ceil(fullSize / chunkSize)
then, per chunk:
  [TYPE 0] realign: round up to 8, then advance by 8 until (pos mod 0x10) == 8
  compressedSize u32
  [TYPE 0] rawFlag u32        4 means the chunk is stored uncompressed
  <compressedSize bytes>      raw DEFLATE, no zlib wrapper
```

Two header shapes exist and are told apart by probing the first u32 after the tag. The TYPE 0
realignment applies before **every** chunk, not just the first.

---

## 6. Deduplication — verified, and the biggest trap

**Multiple file-table rows can point at the same data block.** Identical payloads are stored once.

Measured on a clean `game_90.big`:

| | |
|---|---|
| entries | 92,322 |
| distinct data blocks | 72,937 |
| shared blocks | 4,386 |
| entries sharing a block | 23,781 (**25.8%**) |
| largest single group | **3,313 entries** on one block |

Kit textures are among the worst: one pant normal map is shared by ~190 entries spanning every team
and both brands; kit fonts are shared across a team's `home`/`away`/`three` variants.

### Why it matters

Writing an entry **in place** rewrites the block its siblings are still reading, and updates only that
one row's `SIZE`. The siblings are left pointing at a payload that is the wrong content and often the
wrong length.

The failure does not look like a corrupted texture. It looks like *the whole renderer breaking*,
because the corrupted siblings belong to unrelated teams that the game also loads. Three separate
"the game softlocks" incidents in this project traced back to exactly this, one of them undetected for
weeks.

**Rule: never write in place into a shared block. Relocate, so the entry gets its own copy.**

Detect it by grouping rows by OFFSET; a group larger than one is shared. Identical `(start, end)`
pairs are normal deduplication, *not* damage — only a **partial** overlap (two entries claiming
different but intersecting ranges) means a broken write.

---

## 7. Editing: what is safe

### In-place write — safe when the block is exclusive

Possible only when the new payload is **≤ the existing span**, and the entry shares its block with
nothing. Write the bytes, zero-fill the leftover slack, update `SIZE` at row `+8`. The file length
never changes.

Because payloads are compressed, whether a replacement fits depends on how well the new image
compresses. It varies per texture and can only be found out by trying.

### Relocating write — structurally correct, but see §8

Append at the next `BlockSize`-aligned offset past EOF, then update three fields:

1. row `+0` — new offset **divided by BlockSize**
2. row `+8` — new span
3. header `0x18` — new archive size

Verified byte-exactly on a test archive: a relocation changes **9 bytes** of metadata and nothing
else. The name table, folder table and every other entry's data are untouched.

### Recomputing checksums

Nothing to do. The row's `CRC` is not a checksum and there is no archive-wide digest.

---

## 8. Limits and ceilings

| Limit | Value | Notes |
|---|---|---|
| Addressable size | `0xFFFFFFFF × 16` = **68.7 GB** | offsets are u32 block indices |
| With LargeBlockSize | `0xFFFFFFFF × 64` = 275 GB | unused in NHL 21 |
| Archive size field | u64 | not a constraint |
| Files per archive | u32 | not a constraint; largest real archive is 92,322 |
| Folders per archive | **u16 → 65,535** | largest real value is 1,533 |
| Entry span | u32 → 4 GB | largest real entry is ~5.6 MB |
| Path length | `FileRecordLength - 2` | per archive; 84 chars in `game_90` |

### Block alignment — observed

Every one of the 16 shipped NHL 21 archives has a file length that is an exact multiple of 16, its
declared `BlockSize`. 64-byte alignment is *not* consistent, so 16 is the rule.

A relocating write leaves the file unaligned unless it pads. Whether the game requires alignment is
**unresolved** — see below.

---

## 9. Unresolved: growing an archive can break the game

**Observed:** after every entry in `game_90.big` was restored to content-identical-with-stock (verified
entry by entry against the clean dump, 0 differences), the game still would not get past the
language-select screen. Copying the pristine archive over — so the file was *byte*-identical, not
merely content-identical — fixed it immediately. Three subsequent in-place edits that left the file
length unchanged were fine.

The only difference in the broken state was structural residue from relocation: 61 MB appended, ~36
entries living at appended offsets.

**Ruled out:**

- Relocation writes are minimal and correct — 9 bytes, verified byte-exactly.
- The row `CRC` is not a checksum, so a stale value is harmless.
- Nothing outside the archives records their size: not `sce_discmap.plt`, not `Project.gp4`, not
  `nhl/data/*/config/*`.
- Simple misalignment is **not** sufficient on its own: `inxp_00.big` (length % 16 = 4) and
  `inxp_10.big` (% 16 = 8) are both grown and unaligned, and the game boots fine with them.

**Still open:** why `game_90.big` specifically is intolerant. Candidates not yet tested: a size or
offset threshold particular to the largest archive; a limit on how many entries may live past the
original EOF; or block alignment mattering only for this archive.

**Working rule until resolved: do not let `game_90.big` grow.** Prefer replacements that fit their
existing span; treat a "relocated" result as a signal to stop and reset from the clean dump.

This interacts badly with §6: entries in shared blocks *must* relocate to avoid corrupting siblings,
so the most heavily deduplicated textures — pants and fonts especially — currently cannot be edited
safely at all.

---

## 10. Tooling

In `Nhl19BigStudio.Cli`:

| Command | Purpose |
|---|---|
| `list <big> [n]` | header, per-entry offset/size/codec, chunk tally |
| `search <big> <query>` | find entries by path (caps at 200 results) |
| `verify <big>` | bounds, partial overlaps, header/file agreement |
| `diff <clean> <working> [filter]` | per-entry content comparison against a pristine archive |
| `extract <big> <entry> <out>` | decode one entry |
| `replace <big> <entry> <file>` | write one entry, with an undo record |
| `undo <undo-file>` | revert one write |
| `undo-export <undo-file> <out>` | recover the original content a record captured |

`diff` is the highest-value one. It compares raw spans first — an untouched entry is byte-identical on
disk — and only decodes when they differ, so a 92,322-entry archive compares in a couple of minutes.
It found 36 corrupted entries in one pass that hours of per-texture investigation had missed.

**When something breaks, run `verify` and then `diff` before suspecting any individual asset.**

# TDB String Pools and Localization Database

**Evidence:** measured. The Huffman decoder reproduces every dictionary's text and the documented test vectors; pool rebuild on write is automated-tested. The dictionary census (row counts, pools, alphabets per title and platform) was read from every anchor and from the installed games. Growth behaviour in game is open.

Team names, city names, league labels and most other text the games display do not live in the main database. They live in a **localization dictionary** (`nhl_eng_us.db`, or `eng_us.db` on PlayStation 4), one per language, as a single hash-keyed table whose strings sit in a Huffman-coded pool. The main database refers to them by key. The same pool mechanism is used by any TDB table whose allocation type is 66.

## The dictionary table

One table (`vCJG` in real-id convention, `GJCv` on PS4), three fields: `hashid`, `stringid` and `sourcetext`. The game looks strings up by `hashid`, a case-folded CRC of the key text, so keys must be unique by hash, not merely by spelling.

Rows are packed: on every dictionary measured, Xbox 360, PlayStation 3 and PlayStation 4 alike, `max_records` equals `current_records`. There are **no spare rows anywhere**. Adding a key means growing the record block, which moves the pool and changes the file length, the size word, the row counts and every CRC; inside a packed archive it also changes the member length.

## The type-66 pool

The extra-data section of a type-66 table holds the Huffman **tree first, then the packed strings**.

**Tree on disk.** A breadth-first sequence of 2-byte child descriptors `[index, value]`. For each internal node the file gives the left child then the right child; `index == 0` marks a leaf carrying byte `value`; a nonzero index marks an internal node. One quirk matters for anyone reimplementing it: when newly discovered internal children are queued for later expansion, the **right child is queued before the left**.

**Codes.** Depth-first from the root, left appends bit 0, right appends bit 1. EA's trees top out around 16 bits deep.

**Per string.** A big-endian length prefix (1 byte for kind 13, 2 bytes for kind 14) holding the **decoded character count**, then the bitstream, read most-significant bit first. There is no end-of-string symbol; the decoder stops after exactly the prefixed number of characters. Worked vector: with codes A=`0`, B=`10`, C=`11`, the text `ABCAB` is the bits `0 10 11 0 10`, packed as the single byte 0x5A.

**Alphabet.** Only bytes present in the tree can be encoded. Rebuilding a tree means re-encoding every string in the pool.

## Writing text

A single varchar change ripples through every later string's offset, so a writer rebuilds the whole pool: keep the tree bytes verbatim, re-encode every populated row's text with the existing tree, deduplicate identical strings, write new offsets into the records and recompute the CRCs. Two constraints follow:

1. **No new glyphs.** A character absent from the tree cannot be written without a full re-tree.
2. **Pool fit.** The pool may shrink (the tail is zero-padded) but growing past the original section moves every later table.

The cheapest useful operation is therefore an in-place re-encode of an existing key's text when the new bit length fits.

## What the dictionaries hold

| Dictionary | Rows | Pool (bytes) | Characters absent from the tree |
|---|---|---|---|
| X360 NHL 07 | 3 952 | 119 144 | `* ; @ [ \ ] ^ \` { \| } ~` |
| X360 NHL 08 | 5 738 | 175 136 | `* ; \ ^ { \| } ~` |
| X360 NHL 09 | 9 448 | 362 272 | `^ \| ~` |
| X360 NHL 10 | 12 505 | 467 792 | every lowercase letter except `d` and `s`, plus `\ \| ~` (an all-caps game) |
| X360 NHL 11 | 14 920 | 572 520 | `` ` j q v x z \| ~ `` |
| X360 NHL 12 | 16 787 | 668 064 | `` ` h j q v x z \| ~ `` |
| X360 NHL 13 to Legacy | 18 826 to 25 782 | 851 704 to 1 346 408 | `` ` \| ~ `` |
| PS4 NHL 15 to 20 | 24 251 to 37 734 | 1 258 128 to 1 888 560 | `^ ~` (NHL 15 and 16 also `` ` `` and `\|`) |
| PS4 NHL 21 | 55 766 | 3 028 680 and up | `^ ~` |

Pools are packed to within about 70 bytes of their end on the gen7b titles and about 30 on PS4.

**PS3 dictionaries are different files from the Xbox 360 ones on every title.** Row counts differ (NHL 08: 5 718 on PS3 against 5 738 on Xbox 360; NHL 10: 12 559 against 12 505; Legacy: 25 783 against 25 782) and the Huffman trees differ, so alphabet checks and capacity must always be read from the target's own file, never from the sibling platform. PS3 NHL 10 keeps a different partial lowercase set from Xbox 360 NHL 10.

EA's own PS4 patches ship **grown** dictionaries (NHL 15: 24 251 to 24 443 rows; NHL 19 base to patch 56: 37 734 to 38 001), which shows a patched build loads a larger file than the base shipped. Whether a game accepts a dictionary grown by a tool is **open**.

The `lng_lg` dictionary on Xbox 360 NHL 12 to Legacy and PS4 NHL 21 is a 12 363-row all-caps language template, byte-identical across the Xbox titles, not a player-facing locale.

## Fonts

A string that encodes is not necessarily a string the game can draw. Glyph coverage of the game's fonts is a separate limit, measured per title from the shipped TrueType character maps; the Studio checks requested text against both the tree and the font before accepting it. The shipped fonts cover 322 to 339 code points on the Xbox 360 and PS3 titles and 619 to 1 138 on PS4; the gen7 fonts lack eight Polish and Hungarian capitals and the CJK characters of the language-selection words. Details are on [Team Identity, Logos and Kits](Team-Identity-Logos-and-Kits).

Last reviewed: 2026-10-20

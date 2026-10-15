# PS4 Save Container Format (verified — read/extract)

> **Verification status: VERIFIED (read/extract).** Reversed and confirmed against the PS4 anchors
> (nhl19). The inner roster TDB decompresses cleanly, its header CRC verifies, and its records decode
> to real data (NHL city names). Structural repack is tested; game acceptance remains unverified. PS4 is **little-endian
> TDB v6** ([tdb-v6-format.md](tdb-v6-format.md)).

## Save directory (Sony save-data export)

Each save is a directory (e.g. `RosterCustom0/`, `BeAGM2/`, `Season0/`, `Profile0/`):

| Entry | Meaning |
|---|---|
| `NAME` | the save slot name (ASCII, e.g. `RosterCustom0`) |
| `INDEX` | file index — begins `__SaveLoadFileInfo` (~320 B) |
| `sce_sys/param.sfo` | PS4 SFO metadata (~1596 B) |
| `sce_sys/icon0.png` | save icon |
| **`DATA`** | the payload (the editable data lives here) |

## `DATA` = length-prefixed record stream

`DATA` is a sequence of records, each:

```
[u32 LE length] [12-byte header] [payload (length bytes)]
```

The 12-byte header is `00 70 00 00 00 <id> <7-byte shared tag>` — byte `0x70` is constant, one byte is
a per-record id/sequence, and the trailing 7 bytes are shared across a save's records (a content
hash/timestamp — exact role TBD; needed for write-back). Observed records in a roster save:

| Rec | Payload |
|---|---|
| 0 | JSON `{"IsBackup":false}` |
| 1 | game version string (`1.60`) + padding |
| 2 | JSON metadata `{"FileCreateDate":…}` |
| 3 | **the main payload** (the roster) — see below |

## Main payload: chunkzip or `[1-byte codec]? [LZ4 frame]` → LE TDB

NHL15 uses the same record stream with a bare `chunkzip` roster payload. The committed
`84f8102f - ROSTER/DATA` anchor is 739,913 bytes; its roster record starts at offset 52,
has a native padded header (version word 2, 128 KiB window, 20 chunks), and decodes to
2,510,592 bytes of little-endian TDB with a valid header CRC. The codec now reads this
variant and repacks edited content using native chunk framing, preserving other records
and all record headers. An unknown chunkzip framing is refused for repack.

The October 3 overnight QA also passed the production mirrored database/roster edit,
save and reopen test, then opened the written roster independently to check its edits.
This establishes structural write-back, **not emulator or console acceptance**; header
tag semantics and companion-file update requirements remain unverified in game.

The big record's payload is an **LZ4 Frame** (magic `04 22 4D 18`), optionally preceded by a `01`
codec-id byte. **NHL17–19+ use `[01][frame]`; NHL16 uses a bare `[frame]`** (no codec byte) — verified
against both anchors. The 12-byte record header also varies by title (NHL19 starts `00 70 00…`, NHL16
`07 00 00…`), so the codec preserves it verbatim and never interprets it.
Decompressing the frame yields a **little-endian TDB v6** at offset 0:

- nhl19 `RosterCustom0`: LZ4 1,734,365 B → **7,557,036 B** inner TDB, **54 tables** (a roster subset,
  not the full 111-table `nhl_game.db`), inner **header CRC verified OK**.
- Records decode correctly — `StEO` (arenas) → "Anaheim, CA", "Winnipeg, MB", "Calgary, AB", … (the
  31 NHL cities), confirming the LE read path end-to-end.

> **This is the PS4 roster save's database.** *(Corrected 15 Oct 2026, P0-6: an earlier line
> called the base `nhl_game.db` a default/template only.)* Studio edits `nhl_game.db` as the main
> database, in the role `nhlng.db` plays on PS3/Xbox 360. NHL21-E1 changed it inside `inxp_00.big`
> and the user accepted the result in game. The roster save is a separate, editable file.

## Data after the TDB (the roster trailer)

The decompressed roster payload can carry bytes after the TDB's declared size (`db_size` at TDB
`+0x08`). Studio splits them off at that size when it opens the save, edits only the TDB (so every
TDB CRC, including EOF, is computed at the TDB's own end) and re-appends them when it saves
(`studio_domain::Project`, `Source::Ps4Save`).

| Save (anchor) | Bytes after the TDB | Content |
|---|---|---|
| NHL 17 `RosterCustom0` | 20 | `D9 2E B7 58` · `00 00 00 00` · **`u64` LE = TDB size** · `02 00 01 00` |
| NHL 18 / 19 `RosterCustom0` | 18 | `D9 2E B7 58` · `00 00 00 00` · **`u64` LE = TDB size** · `01 00` |
| NHL 18 / 20 / 21 `ThreeOnThree0` | 15–17 k | JSON (`{"DefaultPlayers":…`) |
| NHL 15 / 16 BeAGM, NHL 16 Season | 160–370 k | mostly zero padding |
| NHL 15 / 16 ROSTER | 0 | — |

The embedded size equals the TDB size on all three roster anchors (measured 13–14 Oct 2026).
When an edit changes the TDB length (for example capacity normalization), Studio rewrites that
`u64` **only** when the trailer has exactly this form and the stored size equals the old length.
The tail bytes (`02 00 01 00` / `01 00`, meaning unknown) are kept verbatim. Every other trailer
form refuses a size change. Tests: `studio-domain` `session.rs`
`ps4_roster_edit_keeps_the_trailer_outside_the_crc_scope` and
`ps4_roster_growth_rewrites_the_trailer_size` (`48a9e85`). Runtime acceptance of a grown save is
**unverified**.

## Verified vs deferred

- ✅ **Verified:** the directory layout, the record stream framing, the LZ4 payload, the inner LE TDB
  (header CRC + real-data decode).
- ✅ **Structurally verified:** chunkzip and LZ4 recompression, record-stream rebuild, preserved
  non-roster records, and production NHL15 edit/save/reopen.
- ⏳ **Deferred (runtime write-back qualification):** the 12-byte record
  header tag/hash semantics; updating `INDEX` / `sce_sys/param.sfo`. No PFD-style console encryption
  was observed (these are emulator/decrypted exports — confirm for real-console write-back).

## Tooling note

LZ4 Frame decode/encode is a standard codec (the engine's Rust uses the `lz4_flex` / `lz4` crate). The
audit used Python `lz4.frame` (scratchpad only; not a repo dependency).

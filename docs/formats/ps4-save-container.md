# PS4 Save Container Format (verified — read/extract)

> **Verification status: VERIFIED (read/extract).** Reversed and confirmed against the PS4 anchors
> (nhl19). The inner roster TDB decompresses cleanly, its header CRC verifies, and its records decode
> to real data (NHL city names). Write-back (recompress + rebuild) is deferred. PS4 is **little-endian
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

## Main payload = `[1-byte codec]? [LZ4 frame]` → LE TDB

The big record's payload is an **LZ4 Frame** (magic `04 22 4D 18`), optionally preceded by a `01`
codec-id byte. **NHL17–19+ use `[01][frame]`; NHL16 uses a bare `[frame]`** (no codec byte) — verified
against both anchors. The 12-byte record header also varies by title (NHL19 starts `00 70 00…`, NHL16
`07 00 00…`), so the codec preserves it verbatim and never interprets it.
Decompressing the frame yields a **little-endian TDB v6** at offset 0:

- nhl19 `RosterCustom0`: LZ4 1,734,365 B → **7,557,036 B** inner TDB, **54 tables** (a roster subset,
  not the full 111-table `nhl_game.db`), inner **header CRC verified OK**.
- Records decode correctly — `StEO` (arenas) → "Anaheim, CA", "Winnipeg, MB", "Calgary, AB", … (the
  31 NHL cities), confirming the LE read path end-to-end.

> **This is where the live PS4 roster lives** — recall the base `nhl_game.db` holds default/template
> values; the edited roster is the LE TDB inside this LZ4 record.

## Verified vs deferred

- ✅ **Verified:** the directory layout, the record stream framing, the LZ4 payload, the inner LE TDB
  (header CRC + real-data decode).
- ⏳ **Deferred (write-back):** LZ4 **re-compress** + rebuild the record stream; the 12-byte record
  header tag/hash semantics; updating `INDEX` / `sce_sys/param.sfo`. No PFD-style console encryption
  was observed (these are emulator/decrypted exports — confirm for real-console write-back).

## Tooling note

LZ4 Frame decode/encode is a standard codec (the engine's Rust uses the `lz4_flex` / `lz4` crate). The
audit used Python `lz4.frame` (scratchpad only; not a repo dependency).

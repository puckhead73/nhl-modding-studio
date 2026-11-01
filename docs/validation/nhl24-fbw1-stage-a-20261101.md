# FB-W1 stage A: NHL 24 database write through `game-mods`, 1 November 2026

**Summary.** A database edit written by Studio's own tooling loads in NHL 24. The edit went to
the `game-mods` overlay as a new archive file plus an edited copy of one index file, and the
index file was not re-signed. The user saw the edited name in game. The only edit was one
same-size rename, so database growth and the batch file list are still untested. Those are the
next probe.

Package: [nhl24-roster-writer-package-2026-11-01.md](../plans/nhl24-roster-writer-package-2026-11-01.md).
Target: NHL 24 PS4 CUSA37933 01.60, under shadPS4 (`nhl24_fix.exe`), install `F:\Games\NHL24`.

## 1. Game-verified fact

On NHL 24 PS4 01.60 under shadPS4, the following loads in game:
- a `game-mods\Patch\Gen4b\globals.toc` whose one chunk-table entry for the database batch is
  repointed, with the 0x08–0x22B header and signature bytes unchanged;
- a new `game-mods\Patch\Gen4b\superbundlelayout\nhl_installpackage_00\cas_02.cas` holding
  the batch as stored (codec 0) blocks.

With these in place, the edited `nhl_game.db` is the one the game uses. The user confirmed in
game, 1 Nov 2026, that `ubPc` row 1570 (Connor McDavid) showed the last name "FBW1PROBE".
Probe 2 (§5) extends this to a resized database.

These are the second and third game-verified facts in the repository. They cover only this
target and this route.

## 2. Findings from existing files (read-only, not separate probes)

| Finding | Level | Basis |
|---|---|---|
| shadPS4 overlays `.toc` and `.cas` files from `game-mods` | runtime-observed, then game-verified by §1 | the existing NHL 27 deke mod; shad_log "Files found in game mods folder" |
| `.toc` signature not enforced by shadPS4 | runtime-observed, then game-verified by §1 | the deke mod TOCs differ from stock in 12 payload bytes each, with the signature region identical; §1 |
| Codec-0 stored blocks accepted | game-verified by §1 | the probe batch was all stored blocks |
| The database batch has exactly one TOC reference: `Patch/Gen4b/globals.toc` file offset `0x1CF0`, stock `(0x00010001, 0x05DD0950, 0x1FD410)` | implemented | `build_probe.py` |
| `globalsfull.toc` holds no entry for the batch | implemented | same |
| The batch decodes to 14,839,434 bytes in 57 Kraken blocks; `nhl_game.db` at 1,112,652 is byte-identical to `local/arena-library/nhl24-resolved` | implemented | same |

Whether a real PS4 console checks the signature is untested and out of scope: the target is
the emulator.

## 3. Probe construction

- **Edit:** `ubPc.QbMR` (inline string) of row 1570, changed from "McDavid" to "FBW1PROBE",
  with CRCs recomputed. 13 bytes changed; the database stayed the same size. The source and
  staged files both round-trip, and restoring the name gives back the original bytes exactly.
- **Batch:** re-framed as 57 stored blocks with the original decoded block sizes (14,839,890
  bytes).
- **Index:** the one entry was rewritten to `(0x00010002, 0, 14,839,890)`, meaning Patch layer,
  catalogue 0, `cas_02`, offset 0.
- **Installed hashes:**
  - `globals.toc`: SHA-256 `ce6e6f5a67a3a49d0a06bdbb0553e28aa666b7fdd94febd850e598a5663a3bd3`
  - `cas_02.cas`: SHA-256 `c36fb1a225f04c6a51af7c783c031e8c4a47aa29b80aa9355d52d0bd05a78d8d`
- **Variant B** (a copied `cas_01.cas` with the batch appended) was built but not needed.

## 4. Cleanup

- Both probe files and the created `nhl_installpackage_00` folder were removed after the test.
  `game-mods` again holds exactly its prior six files.
- Scratch (session scratchpad `fbw1/`): the probe databases, `build_probe.py`,
  `build_probe2.py`, the manifest dump and the staged variants. These are kept as the reference
  for porting in stages B and C, then deleted.
- The temporary examples `crates/tdb-core/examples/fbw1_probe_{scan,edit,grow}.rs` were
  deleted at the close of stage A.
- No process was left running.

## 5. Probe 2: a resized database (game-verified, 1 Nov 2026)

**Result.** The game loads a database of a different length, together with a rewritten
collector file list. The user's screenshot of the Edmonton Oilers roster screen shows
"C. FBW1GROWTH" in the list and in the player header. OVR, salary and contract read normally,
and the rest of the roster is intact.

**Edit.**
- `ubPc` capacity grown from 13,300 to 14,300 slots with `TdbFile::grow_table_max` (stock
  usage is 9,067 records).
- `nhl_game.db` went from 13,726,652 to 13,906,652 bytes, and every later table shifted.
- Row 1570's last name was set to "FBW1GROWTH".
- Every existing record in every table is otherwise unchanged (asserted).

**Collector file list** (manifest chunk `47a230bb2cfe55e5a3bf938bde520b43`, 356,680 bytes
decoded):
- The header is u32 words. `+12` is the entry count (3,225) and `+16` the entry start (64).
  `+36`/`+40` give the count and offset of a packed-batch size table: 4 records of 24 bytes,
  each a u64 decoded size followed by a native chunk GUID.
- Entries are 56 bytes: u64 `name_at`, `compressed_start`, `compressed_end`, `offset` and
  `size`, then the chunk GUID.
- The batch holds 5 files in contiguous order: `cannedmessage.csv`, `nhl_gm_simulation.data`,
  `nhl_game-meta.xml`, `nhl_game.db` and `playercolors.csv`. No other manifest references the
  batch.
- `compressed_start`/`compressed_end` are stored-stream offsets of the first and last block
  covering the file; for the last file, the end equals the stored length. The writer
  recomputes them for 64 KiB stored blocks. Probe 1 left them stale and still loaded, so the
  game may slice the decoded batch; that is not relied on.
- The probe rewrote the five entries and the batch size in the size table (14,839,434 →
  15,019,434). 33 bytes changed and the length was unchanged.

**Install.**
- One new `cas_02.cas` (15,377,970 bytes; SHA-256 `eac93e85…3c3d4e`) holding the batch at 0
  and the re-framed manifest after it.
- `globals.toc` (SHA-256 `5e4b1514…35abf2`) with two 12-byte entries repointed, for the batch
  at `0x1CF0` and the manifest. The header and signature were unchanged.
- Both files were removed after the test, and `game-mods` holds its original six files again.

**Not yet tested:** adding populated records, such as new players with their companion rows.
A bigger database and shifted tables are now proven, and `ubPc` already has 4,233 spare slots.

## 6. Next

Stages B and C: port the probe builders into a Rust writer in `tdb-frostbite`. It needs the
position-aware TOC index, the collector manifest rewrite, stored-block framing, the
`game-mods` ownership manifest, restore, and an emulator-running refusal, with synthetic and
scratch-overlay tests. Then stage D, Studio integration.

# Save Editor write-back quality pass (19 October 2026)

**Summary.** A tester reported that a PS3 save written by Studio would not load in the game. This pass ran every committed anchor save through the Save Editor — open, save unchanged, edit a rostered player (number and surname), save, reopen, put the original values back, save again, and move a player to another team and back — and compared each written file with the anchor byte for byte. The pass found that Studio did not reproduce the games' own files in several ways and fixed them: PS3/Xbox 360 NHL 14, NHL 15 and Legacy saves were re-compressed with different zlib settings than the games use; PS4 NHL 17–21 saves were re-compressed with a different LZ4 build and lost a size word the game writes after the compressed data; the container checksum that mode saves (franchise/season/Be a Pro/profile) carry was computed over the wrong range, and those saves were not even recognised because the reader trusted an unreliable version number. After the fixes, **every roster save Studio can open on every game and platform (NHL 07–21, PS3, Xbox 360, PS4) is reproduced byte for byte** on an unchanged save and after reverting an edit — with one exception, PS4 NHL 16, whose LZ4 build could not be identified (its saves are equivalent but not identical). Nothing here is game-verified; the one remaining known risk for the reported PS3 problem is the NHL 10/11 `tag_1c` word, which is still only partly reversed.

Evidence level: **implemented and automated-tested** against the committed anchors. No cell is game-verified.

## 1. Harness

`crates/studio-api/tests/save_roundtrip_matrix.rs` (`every_anchor_save_round_trips_through_save_editor`), 44 cases over temporary copies of the anchors, through `Registry::save_editor_identify / open / save` — the same path the UI uses. Per case:

| Phase | Check |
|---|---|
| no-op | open, save without an edit → file must equal the anchor |
| edit | renumber + rename one rostered player, save → structural diff vs the anchor (container words, checksum validity, which inner tables changed); reopen and read the edit back |
| revert | put the original number and surname back, save → file must equal the anchor |
| move | move a player to another team, save, reopen, move back, save → reported (informational: slot placement may legitimately differ) |

Mode saves (franchise, season, 3-on-3, Be a Pro) run the no-op phase only (their rosters are shown read-only from the bundled database). `SAVE_MATRIX_ONLY=<substring>` restricts the cases; `SAVE_MATRIX_REPORT=<file>` writes the per-case report.

## 2. Findings and fixes

| # | Finding (anchors) | Fix | Where |
|---|---|---|---|
| F1 | **v4 zlib (NHL 14/15/Legacy, PS3 + Xbox 360):** Studio deflated with flate2's default (memLevel 8), so every save came back with a different stream, different `compressed_size` and different checksums although the inner database was identical. The games' streams are real zlib **level 6, window 15, memLevel 9, default strategy** — a 1 400-combination sweep matched exactly that on all five v4 roster anchors. | Deflate through `deflateInit2_` (libz-sys) with those parameters. Every v4 roster anchor now repacks byte-identically. | `tdb-savedata/src/pack.rs` |
| F2 | **PS4 LZ4 (NHL 17–21):** the writer used `lz4_flex` with its own framing; the games write `FLG 0x64 / BD 0x40` frames (independent 64 KiB blocks, content checksum, no block checksums, no content size) **followed by a little-endian u32 decompressed length** that the writer dropped. Blocks are produced by LZ4's fast compressor as it stood before 1.9.0 (liblz4 1.10 differs on a few blocks by one byte of match-start slack). | `tdb_savedata::lz4_legacy`: a port of the pre-1.9 `LZ4_compress_generic` (byU16 path) plus the frame writer; the size word is preserved and rewritten for the new length. NHL 17/18/19/21 roster and 3-on-3 saves repack byte-identically. | `tdb-savedata/src/lz4_legacy.rs`, `ps4.rs` |
| F3 | **Container variant chosen by version word:** the word varies by title *and* save type (NHL 12 SEASON/BEAGM word 1 over a 44-byte raw header, NHL 13 DYNASTY word 3, NHL 14/15/Legacy DYNASTY/SEASON words 5/6 over zlib, NHL 14+ Xbox PROFILE word 5 raw). Every one of those saves was refused ("could not be read"). | The body marker (`DB` at 0x1C / 0x20 / 0x2C, zlib at 0x30) settles the variant; the word is consulted only when no marker is present (keeps the gen6 dynasty / HockeyCard classification as before). | `tdb-savedata/src/header.rs` |
| F4 | **`checksum_b` scope:** it covers `[0x2C .. 0x2C + uncompressed_size]`, not to EOF. Identical on rosters; on every mode save (NHL 12–Legacy, raw and zlib) the stored value matches only that range. | `checksum_b_region_end`; recompute uses it. | `tdb-savedata/src/checksum.rs` |
| F5 | **Mode-save layout:** after the payload region a `00000002 00000001 <len> <tag> …` section list follows (raw v2 and zlib v4 alike), covered by `checksum_a` only. The v4 writer zero-filled to the old file length, destroying it; the v2 writer declared the trailer inside `uncompressed_size`. A v4 mode save's `uncompressed_size` is exactly **half** its inflated payload (meaning unknown). | v4: region = `0x2C + payload` on rosters (allocation follows the data), the declared region on mode saves; sections appended verbatim; inflate no longer rejects "declared ≠ inflated". v2: `uncompressed_size` = database length. | `pack.rs`, `extract.rs` |
| F6 | `studio-domain` `save_identify` and `studio-api` `save_editor` tests still expected NHL 20 PS4 to have no definitions (stale since the genuine anchor landed on 15 Oct). | Expectations updated (NHL 20 3-on-3 identifies, opens with the mode caution). | tests only |

Not fixed, documented:

- **PS4 NHL 16** LZ4 build: 12 of 52 roster blocks differ in both directions from this port, from the pre-r129 search variant and from liblz4; the repack is equivalent (identical inner database, every other record preserved), not identical.
- **PS4 NHL 17+ mode saves** nest a second `[01][LZ4]` frame; NHL 17's inflates to the mode TDB, NHL 18–21's to a section table that is not a TDB. Identification fails with "missing 'DB' magic". NHL 15/16 mode saves hold the TDB directly and open read-only.
- **PS4 roster `CRCValue`** (54-character string in the JSON record) is never recomputed; unknown whether the game checks it.
- **NHL 10/11 `tag_1c`** (v1 containers): CRC-32/MPEG-2 over `file[0x20 .. EOF-4]` on the NHL 10 BEAPRO (PS3 and Xbox) and SETTINGS anchors; the ROSTER, BEAGM and SEASON tags match no contiguous CRC/hash under any standard 32-bit parameter set, and the PS3 and Xbox NHL 10 (and NHL 11) ROSTER anchors carry the **same** tag over bodies that differ in ~6 000 bytes — so for those types it is not a hash of the stored body. Preserved verbatim as before; **this remains the most likely cause of a PS3 NHL 10/11 save the game refuses**, and the only way to settle it is a game-written pair (same roster saved twice with one change) or a controlled in-game load of an edited save.

## 3. Results (final run, Windows, debug profile)

| Group | Cases | no-op | revert | Notes |
|---|---|---|---|---|
| PS3 roster saves NHL 08–Legacy (v3, v1, v2, v4; NHL 08 change list) | 9 | 9 identical | 9 identical | NHL 14/15/Legacy were not identical before F1 |
| Xbox 360 roster saves NHL 07–Legacy | 10 | 10 identical | 10 identical | NHL 14/15/Legacy as above; NHL 07/08 change lists |
| PS4 roster saves NHL 15–19 + 3-on-3 NHL 18/21 | 7 | 6 identical, NHL 16 equivalent | 4 identical, NHL 16 equivalent | NHL 17/18/19/21 not identical before F2 |
| PS3 / Xbox 360 mode saves (single-file NHL 09–Legacy) | 13 | 13 identical | — | 7 of them unreadable before F3 |
| PS4 mode saves | 5 | NHL 15 identical, NHL 16 equivalent, NHL 17/19/21 unidentified (expected) | — | nested LZ4, see §2 |

Edit phase: on every roster save the edit changed only the expected tables (bio + name pool / companion rows, or the change-list companions on NHL 07/08) and read back after reopen; container checksums valid. Move-and-back: identical on raw containers whose slot order is restored; on NHL 07/08 change lists and PS4 NHL 19 the file differs only by slot placement (informational).

Regression: `tdb-savedata` 9 unit + 10 integration (new: v4 byte-identity on all six roster anchors; variant-by-marker on seven mode saves with byte-identical repack; PS4 LZ4 byte-identity on NHL 17/18/19/21; `lz4_legacy` decode round-trip); `studio-domain` `save_identify` (2) and `save_matrix` (1, CSV regenerated — §4); `studio-api` `save_editor` (12); the matrix itself (1, 44 cases, 524 s).

## 4. Generated evidence

`docs/audits/data/save-matrix.csv` was regenerated with `QA_WRITE=1 cargo test -p studio-domain --test save_matrix` because its subject — container classification and codec round-trip — is exactly what changed: 31 rows move from `unsupported`/`v1`/`v3`/`v4` `read-only` to the correct variant with a codec `roundtrip` (every PS3 SETTINGS, every PS3 NHL 12 mode save, every Xbox 360 PROFILE and mode save from NHL 11 to Legacy). No capability CSV was touched.

## 5. Files

- `Cargo.lock` (libz-sys, twox-hash, lz4 resolved).
- `crates/tdb-savedata/`: `Cargo.toml` (flate2 `zlib` feature, `libz-sys`, `twox-hash`; `lz4` as a dev-dependency for the probes), `src/lib.rs`, `header.rs`, `checksum.rs`, `extract.rs`, `pack.rs`, `ps4.rs`, new `lz4_legacy.rs`, `tests/container.rs`, new `examples/lz4_blocks_match.rs` and `examples/ps4_inner_probe.rs` (read-only probes over the anchors).
- `crates/studio-api/tests/save_roundtrip_matrix.rs` (new), `tests/save_editor.rs` (NHL 20 expectation).
- `crates/studio-domain/tests/save_identify.rs` (NHL 20 expectation).
- `docs/formats/ps3-save-container.md`, `tdb-v6-format-spec.md` (§3/§4), `ps4-save-container.md`; `docs/audits/data/save-matrix.csv`.

Temporary files: the harness and tests use `std::env::temp_dir()` folders removed on drop; CRC/zlib search scripts live in the session scratchpad only.

## 6. Still to verify in game

1. A PS3 NHL 14/15/Legacy roster edited in Save Editor (now byte-exact apart from the edit) — the tester's original report, if it was one of these titles.
2. A PS3 NHL 10 or 11 roster edited in Save Editor (stale `tag_1c`).
3. A PS4 NHL 17–21 roster edited in Save Editor (`CRCValue` not recomputed).
4. Any mode save re-saved unchanged (sections now preserved on v4; `checksum_b` scope corrected).

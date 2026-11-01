# FB-W1 stages B and C: NHL 24 database overlay writer, 1 November 2026

**Summary.** Studio now has a Rust writer that puts an edited NHL 24 database into
`game-mods`, at any size. It finds the database through the game's own index, rebuilds the
batch and its file list, repoints the index, and records which files it owns so it can never
replace another mod's files. It can also restore the stock game exactly. The writer's output
for the growth probe is byte-identical to the files the game loaded on 1 Nov, so that result
carries over to this code. Nothing is wired into the app yet; that is stage D.

Package: [nhl24-roster-writer-package-2026-11-01.md](../plans/nhl24-roster-writer-package-2026-11-01.md).
Stage A evidence: [nhl24-fbw1-stage-a-20261101.md](nhl24-fbw1-stage-a-20261101.md).

## 1. What was built

All files are untracked or modified in the working tree; nothing is committed.

- **`crates/tdb-frostbite/src/overlay.rs`** (new)
  - Lists every PS4 TOC chunk-table entry with its native GUID and the absolute byte
    position of its 12-byte record.
  - `repoint` rewrites only the named records. It refuses a missing or duplicate GUID and
    checks that the wrapper and signature bytes are unchanged.
  - Frames data as codec-0 (stored) blocks.
  - Computes the stored-range values that collector manifests record.
- **`crates/tdb-frostbite/src/nhl24.rs`** (new)
  - Roots and view:
    - `Roots` covers `game`, `game-UPDATE`, `game-mods`, and Studio's state folder
      `<install>\studio`, which sits outside the game tree.
    - `View` resolves files the way the emulator stacks them (mods, then UPDATE, then game).
      It can include or skip Studio's own files.
  - Locating the database:
    - Reads the collector EBX from `Patch/Gen4b/globals.toc`.
    - Accepts exactly one candidate manifest chunk that decodes to a directory listing
      `nhl_game.db`.
    - `Directory` reads and checks the 56-byte entries and the packed-chunk size table.
      The batch's files must tile the batch exactly and match their recorded size.
  - Reading and writing:
    - `read_database` returns the database and metadata XML the game currently loads.
    - `stock` returns the inputs a write is rebuilt from. It refuses if another mod in
      `game-mods` replaces `globals.toc`.
    - `plan` rebuilds the batch around the new database, at any length. It rewrites the
      five entries and the size record, keeps the stock framing of the manifest, and puts
      both into one CAS. It repoints the two TOC entries, then reads everything back as
      the game would before returning.
    - `choose_cas` reuses Studio's own CAS, or takes the lowest Patch catalogue-0 index
      that no layer holds (`cas_02` on this install).
  - Ownership, apply and restore:
    - The ownership record is `<install>\studio\nhl24-overlay.json`. It holds each file's
      path and SHA-256, plus the folders Studio created.
    - `apply` checks every target before writing: it must be absent, or Studio's own file
      with an unchanged hash. It writes the CAS before the TOC, through temporary files.
      On failure every target is put back. Studio files the new plan no longer needs are
      removed.
    - `restore` refuses before deleting anything if a Studio file was changed by hand.
      Otherwise it removes Studio's files and created folders, and the record.
  - `apply` and `restore` refuse while `shadPS4`, `nhl24_fix` or `nhl24_new` is running
    (`running_emulator`, which uses `tasklist`).
- **`crates/tdb-frostbite/tests/nhl24_overlay.rs`** (new, `#[ignore]`d): a real-install test
  that writes only into a temporary `game-mods`.
- **`crates/tdb-frostbite/src/lib.rs` and `Cargo.toml`:** add the two modules, plus `sha2`
  as a dependency and dev-dependency. The NHL 27 session's uncommitted edits to these files
  are untouched.

## 2. Checks

| Check | Result |
|---|---|
| `cargo test -p tdb-frostbite`, run on the committed snapshot alone (`git checkout-index` into a temporary folder, without the NHL 27 work): 15 library tests, including 4 overlay and 7 NHL 24 cases, plus the reader tests | pass |
| Synthetic plan: a 70,000 → 200,000-byte database; later file shifted; unrelated entry and chunk unchanged; non-tiling directory refused | automated-tested |
| An ownership record whose path leaves `game-mods` is refused by restore and by the view | automated-tested |
| Synthetic apply/restore: foreign `contentlaunchsb.toc` and `eboot.bin` untouched; re-apply over own files allowed; foreign `globals.toc` refused with nothing written; running emulator refused; hand-edited Studio file blocks restore; CAS index skips files held by any layer | automated-tested |
| `nhl24_overlay` on `F:\Games\NHL24`, temporary `game-mods`, Oodle `oo2core_8_win64.dll` | pass (about 3 s) |
| ↳ stock database reads and round-trips in `tdb-core` | pass |
| ↳ growth edit (`ubPc` +1,000 slots, row 1570 renamed): `plan` output hashes equal the game-verified probe 2 files (`globals.toc` `5e4b1514…`, `cas_02.cas` `eac93e85…`) | pass |
| ↳ apply → read back via the overlay (database and metadata) → re-apply from stock → restore leaves the temporary `game-mods` empty | pass |
| `cargo clippy -p tdb-frostbite --all-targets`: no warnings in the new files | pass |
| `rustfmt` on the three new files | applied |

**Evidence level.** The writer is implemented and automated-tested. For the growth-probe
edit its output is byte-identical to files that are game-verified. Any other edit is
automated-tested only until it is run in game.

## 3. Limits and open points

- **Oodle.** Reading the stock batch and manifest needs the user-supplied Kraken decoder.
  The test used the existing local DLL. Stage D must take the DLL path from user selection,
  as `oodle::Oodle::open` already requires.
- **No crash or power-loss atomicity.** Rollback is best effort. Folders created during a
  failed apply are not removed.
- **Only `Patch/Gen4b/globals.toc` is rewritten.** The batch and manifest have no entries in
  `globalsfull.toc` or the Data TOCs on this build. A layout where they did would make
  `repoint` fail rather than miss a copy.
- **Not yet proven:** populated record inserts, meaning a new player together with its
  companion rows. Stage D adds those through Studio's existing domain operations, and they
  then need their own in-game check.

## 4. Cleanup

- The session scratchpad `fbw1/` (probe databases, Python builders, staged variants,
  about 270 MB) was deleted. This record and the stage A record keep the evidence.
- The real-install test writes only to a temporary folder, which is deleted when the test
  ends.
- `F:\Games\NHL24\game-mods` was not touched in stages B and C (6 files, unchanged).
- No processes were left running.

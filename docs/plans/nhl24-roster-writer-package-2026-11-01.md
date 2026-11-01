# NHL 24 PS4 roster writer — package FB-W1, 1 November 2026

**Status:**
- **Accepted** by the user on 1 Nov 2026. Codex is no longer involved in development.
- **Stage A is done and game-verified:** [stage A record](../validation/nhl24-fbw1-stage-a-20261101.md).
- **Stages B and C are done:** [stages B and C record](../validation/nhl24-fbw1-stages-bc-20261101.md).
- **Stage D is implemented and automated-tested:** [stage D record](../validation/nhl24-fbw1-stage-d-20261101.md). Stage E (the in-app save, checked in game) passed on 1 Nov 2026.

**Summary.** The user wants to change NHL 24 rosters and import portraits and headmeshes.
Studio can read NHL 24 today but cannot write anything into its Frostbite archives, and every
one of those features needs that write path. This package builds it for the roster database
first, because the database is already located and round-trips byte-exactly. Portraits and
headmeshes reuse the same writer later. The package also answers the two questions that block
all NHL 24 writes: whether the emulator checks the index-file signature, and whether it accepts
uncompressed replacement data. Each stage below ends at a review point.

NHL 24 PS4 (CUSA37933, 01.60, shadPS4) is a user scope expansion of 29 Oct 2026 and is outside
the 27 shipping targets. Evidence levels follow CLAUDE.md; nothing below is game-verified.

## 1. What is already established

| Fact | Level | Source |
|---|---|---|
| Database route: `Patch/Gen4b/globals.toc` → `globals/chunkfilecollector` → collector manifest → `gamemodes/database/t3db/nhl_game.db` (13,726,652 bytes) | implemented, run on the real install | [nhl24-binding-implementation.md](../research/arena-library/nhl24-binding-implementation.md) |
| `nhl_game.db` and `nhl_game-meta.xml` share one Kraken-packed chunk (Patch catalog 0, CAS 1) | implemented | same |
| `tdb-core` parses and re-serializes that database byte-exactly; a one-field edit and its restore are exact | implemented (local example `nhl24_arena_stage`) | same |
| Roster joins (`ubPc`, `ZBac`, `eGlu`, `kOtt`) match Studio's existing roster model | reference-tool review, not tested on NHL 24 | [reference-tools-2026-10-29.md](../research/reference-tools-2026-10-29.md) §2 |
| Every `.toc` starts with `00 D1 CE 01` and a 256-byte region that looks like a signature | observed | [frostbite-ps4-file-structure.md](../formats/frostbite-ps4-file-structure.md) §4.1 |
| Oodle provider: decode only, from a user-supplied DLL | implemented | `crates/tdb-frostbite/src/oodle.rs` |

The NHL 27 same-size in-place approach does not carry over: the NHL 24 database sits inside a
compressed batch, so any edit changes the stored size and needs a repoint.

## 2. Open questions this package answers

| ID | Question | How |
|---|---|---|
| FB-Q2 | Does shadPS4 verify the `.toc` signature? | Stage A runtime probe |
| — | What does FMT's `RebuildTOCSignatureOnly` change? | Stage A byte diff of a stock TOC against an FMT-applied one. The install's `__FMTBackup/` may already hold the pair. Characterize by diff only; do not read FMT. |
| FB-Q9 | Does the runtime accept codec `0x00` (stored) replacement blocks? | Stage A runtime probe |
| — | Does shadPS4's `-mods` layer overlay `.toc` and `cas` files the way it overlays `eboot.bin`? | Stage A runtime probe (deployment is `game-mods`, user decision 1 Nov 2026; see §5) |

## 3. Stages

**A. Characterize (research + two runtime probes, on a copy).**
1. Diff stock and FMT-applied TOC pairs and record exactly which bytes the rebuild touches.
2. Probe 0: place an unmodified copy of `Patch/Gen4b/globals.toc` under `game-mods` and confirm
   from `user\log\shad_log.txt` that the emulator opens the overlay copy.
3. Probe 1: in the overlay copy, change one harmless byte of the payload without re-signing,
   then launch and record whether it loads.
4. Probe 2: repoint the database chunk to an identical batch stored with codec `0x00`, held in
   a new overlay-only CAS file, then launch and check the roster loads unchanged.
5. Receipt: the evidence, and whether overlay TOCs may reference a new CAS file. **Stop for
   review.** If the signature is checked and cannot be reproduced, or the overlay does not
   carry TOC/CAS files, the package stops here and reports the blocker.

**B. Frostbite writer library (`tdb-frostbite`, no Studio wiring).**
- Position-aware TOC index: the byte offset of every offset, size and SHA-1 word for a chunk,
  across the TOC chunk table and every bundle copy, in both `Data` and `Patch`.
- Overlay output only: write edited TOCs and new data into `game-mods`, never into `Data`,
  `Patch` or `game-UPDATE`. New blocks go into a Studio-owned CAS file in the overlay if Stage
  A allows it, so that multi-gigabyte stock CAS files are never copied. Otherwise the stock CAS
  is copied into the overlay and appended to. Every reference is rewritten. A span shared with
  another entry is refused.
- Signature handling as Stage A decides.
- Ownership manifest in `game-mods` listing every Studio-written file and its hash. Studio
  refuses to overwrite a file it does not own. The existing modded `game-mods\eboot.bin` and
  any other foreign file are left alone. Restore deletes only Studio-owned files, which leaves
  the stock install untouched.
- Refuse writes while the emulator is running.
- Tests: synthetic TOC/CAS fixtures, plus a scratch overlay round trip where restoring leaves
  only the foreign files, with hashes unchanged.

**C. Database writeback.**
- Rebuild the shared batch (database and meta XML), update the collector directory's offsets
  and lengths, and recompute the content identities the reader already checks.
- Encoding: stored blocks if Probe 2 passes. Otherwise add `OodleLZ_Compress` to the
  user-supplied provider, with a decode round-trip check before writing.
- Tests: the edited database reparses through the full read path; restore is exact.

**D. Studio integration.**
- Open NHL 24 as a direct-editing target backed by the existing roster editors.
- Route writes through the single pending-changes review and coordinated transaction, with
  complete recovery from the Stage B journal.
- Gate each roster operation on its NHL 24 evidence; do not assume the PS4 16–21 rules hold.

**E. Game verification (user).** One visible roster edit, such as a player's name or a
team assignment, applied to the chosen deployment. Confirm it in game, restore, and confirm the
restore. No UI exposes NHL 24 writes before this passes.

Portraits (GNF/RES encode) and headmeshes (gated on the external converter) are follow-on
packages on this writer, not part of FB-W1.

## 4. Limits

- Never write to `F:\Games\NHL24` outside `game-mods`. Stage A probes write there only with
  the user's go-ahead for each probe, and remove their files afterwards. Stages B–D use a
  scratch overlay. Check free space first, and record sizes in the receipt.
- Clean room: format facts come from byte diffs and the existing review notes. Do not read or
  port FMT or NHL24Kit code.
- No QA generators, no capability CSV edits, no commits or pushes without Codex authorization.
- Remove scratch copies at each stage close; keep durable evidence in `docs/validation/`.

## 5. Decisions needed

1. **Codex:** accept this package and its stage order.
2. **Decided by the user, 1 Nov 2026:** writes go to the `game-mods` overlay of
   `F:\Games\NHL24`, and the target is version 01.60. The format document's evidence comes
   from the older `H:\Emulators\ShadPS4\Games\CUSA37933` copy, so Stage A re-checks the facts
   it relies on against the 01.60 install.

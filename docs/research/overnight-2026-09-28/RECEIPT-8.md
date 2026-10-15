# Research receipt 8 (consolidated) — continuation after receipt 7, 28 September 2026

Research lane. Prompt: [`research-20260928-after-receipt7.md`](../../plans/research-20260928-after-receipt7.md). Published 18:52 MDT
(system clock). Authority: [BETA-HANDOFF.md](BETA-HANDOFF.md); log: [STATUS.md](STATUS.md); queue C37 / C38 notes, new C39 / C40.
- No agents, delegation or cross-chat messages; every claim was written before its package.
- No launches, listening, staging, publication, installed / anchor / fixture writes, or app / defs / test / coordinator edits.

## Summary

- **Corrections first:**
  - the board-ad contract is now accepted for implementation;
  - the vault result is narrowed to what was actually tested;
  - last run's two process slips are recorded;
  - the picture review folder is deleted.
- **Recompiled loose-file question.** The recompiled game's own logs show it looking for loose files on its "cache" drive, which maps
  to a folder in Documents, not the game folder. Whether a loose file would win is unknown, so a careful user test card is attached.
- **Two read-route contracts for Recompiled** are ready for the developer: speech, and portraits / heads. The speech one exposes a real
  gap in the existing speech-preview archive reader: it cannot unpack chunklzx files.

## 1. Packages

| # | Output | Result | Evidence class |
|---|---|---|---|
| 1 | [14](14-after-receipt7-corrections.md) | K15 C37 accepted for implementation (developer supplement recorded); K16 vault negative narrowed; K17 delegation and late claim recorded; K18 C38 candidate-only; review folder removed after resolved-path / reparse / ownership checks | review record |
| 2 | [15](15-recomp-loose-override-premise.md) | logs: `game:` = game folder; `cache:` = `Documents\nhllegacy\cache`; loose asset requests appear only under `cache:\` (failures only are logged); no config file exists; precedence **unknown**. Card R-RECOMP-LOOSE-1 (copied install, baseline hashes, reversible A / B changes, found-file control C, restore), not executed | measured log text; inferred mapping |
| 3 | [16](16-recomp-speech-read-contract.md) | 30 speech / context members (audioboot 9, nocache 21), 0 duplicates, 20 chunklzx; 28 / 28 byte-equal to X360 Legacy, 2 `.sbs` size-only; contract for 4 consumers + preview; **gap:** `bank_from_archives` has no chunklzx branch and no `game/` prefix | measured; source reading; failure unreproduced |
| 3b | [17](17-recomp-portrait-head-read-contract.md) | portraits 9 228 (`nocache.big`) / 9 229 (`cache.big`) stored; heads 904 bundles + 685 texlibs chunklzx (`cacherender.big`); 0 duplicates; 24 / 24 sampled equal; 219 bundles without texlib; reuse `read_eb_member` | measured census + sample; source reading |

## 2. Corrected claims

- C37 accepted as an implementation contract: not writer safety, not live installation.
- Vault: "any encoding" / "cannot hold" is replaced by: tested encodings, plus 40 / 47 parsed records, plus "identical bytes cannot by
  themselves encode different complete lists". Superset, filtering and runtime construction remain possible.
- Checkpoint 12 §2 was produced by a delegated search. It is a research audit, not tested coverage.

## 3. Developer-consumable scope (priority)

1. **C37** (accepted): PS3 DXT1 / DXT3 full declared chain under 10 §8, with the supplement's tests.
2. **Recompiled guards** (supplement items 1–2): Hockey Shop refusal (O12-1; 15 adds no loose-loader proof); containment disclosure or
   archive index (17 supplies the index members).
3. **C39** speech route (16), fixing `bank_from_archives`'s chunklzx and `game/` handling.
4. **C40** portrait / head preview route (17).
5. **C38:** candidate-only picture presentation, as before.

## 4. Checks actually run

| Check | Result |
|---|---|
| grep and normalised failed-path counts over the two logs (hashes recorded) | as 15 §2 |
| names-only listing of the cache root | as 15 §1 |
| R145, one run | 30 members, 28 byte comparisons |
| R146, one run | 20 486 member heads, 24 byte comparisons |
| cleanup verification of the owned H:\TEMP folder | removed |
| source reading | `speech_catalogue.rs`, `gen7_speech_preview.rs:190–325`, `gen7_sfx_banks.rs:106–113, 266`, `assets.rs:300–460, 1509, 1541`, `eb.rs:364` |

Nothing accepted was rerun: no census, no format probes, no round trips.

## 5. Remaining blockers and owners

| Blocker | Owner |
|---|---|
| C37 implementation and tests; Recompiled guards; C39 / C40 routes; PS4 hair continuation (receipt 66 delivered, under review) | developer |
| Review of receipts 66+ and of 16 / 17 | coordinator |
| R-RECOMP-LOOSE-1; r64 / r66 cards; P1-A; G-MASK-1; P34-1 / R15-HEAD-1; P34-G08 | user |
| X360 board gates, archive re-key, Austrian phases | withheld |
| Release: MIT holder; artwork replacement before external release | user / developer |

## 6. Cleanup and retention

- **Removed:** `H:\TEMP\nhlms-research-c38-visual-2026-09-28`, exactly 7 owned files and then the folder, after checks (14 K18). The
  session scratchpad is emptied at closeout.
- **Kept:**
  - probes R145 and R146;
  - data files r145 and r146;
  - the manifests r143d / r143f, which describe the deleted sheets (method preserved).
- **Bytecode:** none created (`-B`).
- **Not touched:** shared caches, bundles, toolchain, fixtures, other H:\TEMP folders.
- **Disk:** E: had 2.7 GB free at start. This run wrote only small text files.
- **Claims:** q13–q16 are `ready`.

## 7. Next research task

The arena read route for Recompiled. Produce the same compact contract for `cacherender.big` `rendering/stadium/*`: census, wrapper,
duplicates, the 67 packages already fully compared through R137 plus R128, the consumers `arena_assets.rs:127` / `venue_art.rs:113`
(Recomp not in `MEASURED`), and regressions. Then equipment previews (`rendering/<slot>`), with the same chunklzx check. No write claims.

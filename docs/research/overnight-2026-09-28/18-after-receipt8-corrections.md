# Checkpoint 18 — corrections after receipt 8: authority, handoff 15 and its card, amendments to 16 / 17

Research lane, 1 October 2026, published 01:40 (system clock). Prompt:
[`research-20260928-after-receipt8.md`](../../plans/research-20260928-after-receipt8.md), first package. Claim
`research-q17-after-receipt8-corrections`, written before work began. Sources: the coordinator's
[receipt-8 review](../../validation/research-20260928-receipt8-review.md), the
[receipt-66 review](../../validation/development-20260928-receipt66-review.md), developer
[receipt 67](../../receipts/overnight-development-2026-09-24/67-recomp-routes-and-ps3-boards.md) (developer-verified, **not yet
reviewed**) and the current working-tree source of `speech_catalogue.rs` / `gen7_speech_preview.rs`, read only. The two Recompiled
logs were searched once more (bounded: two files) for any user-data or configuration switch. Nothing launched, no cache touched, no
application edit.

## Plain-language summary

- **Who has accepted what.** Developer receipt 66 has a bounded coordinator acceptance. The speech (C39) and portrait / head (C40)
  contracts are accepted for **reading** only, with the review's amendments. The PS3 board contract (C37) stays PS3-only.
- **Handoff 15 claimed too much.** The game's log lists only files it *failed* to open. That shows where it looked, not in which
  order, and an error that goes away does not prove the file was used. The test card is rewritten below.
- **The card cannot run yet.** The recompiled game always keeps its saves and cache in `Documents\nhllegacy`, even when the game
  folder is copied. Nothing available shows a way to give a test copy its own user-data folder. So the card waits for you to choose
  a backup-and-restore plan for that folder (about 70 MB, 16 files, including saves). No whole-install copy is needed for the first
  trial.
- **Handoffs 16 / 17 are corrected.** "Same bytes as Xbox 360 Legacy" was a historical comparison, not a guarantee about whatever
  install is attached. Loose-first is Studio's own choice, not how the game loads. Duplicate, missing, unreadable and corrupt cases
  are now spelled out.
- **The developer has already built most of the speech route** (receipt 67). Reading it shows two things for the reviewer: an
  incomplete loose speech folder hides all archive copies, and the bank and its stream can come from different places.

## K19 — authority, reconciled

| Item | Status now | Source |
|---|---|---|
| Developer receipt 66 | **bounded acceptance**: F2 and temporary-name ownership closed in tested scope; hair accepted offline; 21 independent tests | receipt-66 review |
| Earlier unreviewed developer work, desktop / game cards | **open** (not closed by receipt 66) | same |
| C39 speech, C40 portrait / head | **accepted for read-route implementation with the amendments** (K21); no writer, no loose-loader support, no runtime acceptance | receipt-8 review |
| C37 PS3 boards | accepted for PS3 implementation only; X360 / Recompiled board writes withheld | receipt-7 / -8 reviews |
| Receipt 67 (speech route, containment index, Hockey Shop guard, C37 writer) | **developer-verified, not reviewed** — research treats its claims as unreviewed developer evidence | receipt 67 header |
| Developer refusal of Recompiled Hockey Shop installs | appropriate; unchanged | receipt-8 review |

## K20 — handoff 15 corrected

**Withdrawn wording.** 15's summary and §3 said the runtime is "first trying" a loose file and "then the archive". The logs record
**failed opens only**: `VFS: entry not found` and `NtCreateFile FAILED`. A successful archive read is never logged, so no order,
fallback or precedence is shown. Withdrawn unless a successful-read trace corroborates it.

**What stands (measured, log text):** the `game:` mount; `cache:` resolving under `Documents\nhllegacy\cache`; failed `cache:\…`
loose requests for individual asset paths; no `game:\` path except `default.xex`. The O12-1 consequence (no evidence for loose
writes into the game folder) is unchanged.

**User-data isolation, from available evidence only.** Both logs print, at start:
```
  Game directory: H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game
  User data:      C:\Users\puckh\Documents\nhllegacy
  Cache root:     C:\Users\puckh\Documents\nhllegacy\cache
```
- The install has no configuration file (15 §1).
- A case-insensitive search of both logs for `command`, `argv`, `args`, `portable`, `config`, `--`, `.toml`, `.ini`, `.json`,
  `settings` and `profile` found no switch, argument or configuration line. The only hits were game asset paths
  (`D:\sku\eu.ini`, `cache:\fe\dynamicsettings.txt`, `cache:\fe\profile\*.png`, and similar).
- **Result: no supported isolated user-data route is established.** Research does not guess runtime switches and does not search
  the loader further.
- A copied install would still read and write `Documents\nhllegacy`.

That root now holds **16 files, 70 118 745 B** (names and sizes only):
- `B13EBABEBABEBABE\454109EC\00000001\…` and `Headers\…`, the profile and saves;
- `cache\guest\` (`809284.ver`, `admanager_cache_images`, `highlights.sav`);
- `cache\shaders\shareable\…`.

### R-RECOMP-LOOSE-1, revision 2 (replaces 15 §4; **pending — not executable as written**)

**Gate.** Execution waits until the user chooses and records a backup / restore procedure for the **whole** `Documents\nhllegacy`
root: every file, including saves and anything the runtime itself rewrites, not only the placed asset. Research does not execute
it. Nothing in this card is authorised by research.

**Space.**
- The `cache:` trial changes no install file, so it needs **no install copy**; the user-data backup needs about 70 MB, on a drive
  the user picks.
- The game-folder trial (T2) would add one file to a game folder. Either the user accepts adding and then deleting one new file in
  the original install, or the user provides a copy destination with at least **6.6 GB** free (the install is 6 588 039 707 B).
  Do not copy onto a nearly full drive.

**Positive control: required before any trial.**
- The asset must have an on-screen role that is **already established**: the user's own prior modding of that exact file on
  Legacy, or an earlier recorded observation. Its path name alone does not establish it.
- `fe/profile/default.png` is **withdrawn** as the default choice: its display role was assumed from its path.
- Research has **no** Recompiled asset with an established role. The user names the asset and the screen before the card runs.
- Alternatively, affirmative read evidence (a logged successful open) would serve, but no supported way to get one is known.
  Research does not guess switches.

**Trials: one location per trial, never two at once.**
1. **Baseline.**
   - Back up `Documents\nhllegacy` and record its full listing with SHA-256 values.
   - Hash `game\*.big`.
   - Launch, record the established screen, exit. Keep the log.
2. **T1, `Documents\nhllegacy\cache\<path>`** (the printed cache root).
   - Place an unmistakable replacement in the same format and size class.
   - Launch, observe, exit. Keep the log.
   - Remove the file, then **restore the whole root** from the backup and verify the listing equals the baseline.
3. **T1b, `Documents\nhllegacy\cache\guest\<path>`**: only if T1 is inconclusive, as a **separate** trial with its own restore. The
   `guest` mapping is a 15 §3 inference.
4. **T2, game folder `game\<path>`** (under the Space rule): place, launch, observe, exit, delete, re-hash.
5. **Restore and verify.**
   - The user-data root equals the baseline backup.
   - `game\*.big` hashes are unchanged.
   - No placed file remains.

**Reading the result:**

| Observation | Meaning |
|---|---|
| The replacement is unmistakably visible on the established screen | positive result for that location, asset and build |
| Stock screen, and the `FAILED` line for that path gone | **inconclusive.** A missing error does not prove the file loaded, or that the screen uses it |
| Stock screen, `FAILED` line still present | that path at that location was not opened; nothing more |
| Any trial | says nothing about other assets, loose `rendering` files, or precedence over archives (that needs a separate designed trial) |

**15 §4 step 4 (control C) is withdrawn.** Its logic, "the error disappears, so the loader found the file", is exactly the
inference the review rejects.

## K21 — amendments to handoffs 16 and 17

These apply to both contracts. They restate the review's amendments in research terms, plus what the current source shows.

**Provenance versus historical equality.**
- R145 / R146 / R128 compared the bytes **of this one install, when measured**, with X360 Legacy. The archives are attachable and
  modifiable, so that is history, not a property of whatever is attached later.
- The UI states the **selected source**: `archive:member`, stored size, wrapper, unwrapped length. It may say "matches the X360
  Legacy reference" only after hashing the current source against a recorded reference.
- Research sample scope is separate from whole-archive history:
  - R146: 24 sampled members;
  - R145: 28 byte comparisons plus 2 size-only;
  - R128: archive SHA-256.
- Neither is a per-member census of every portrait or head.

**Root forms.**
- Two forms: `<install>` with `game\*.big`, or `<install>\game` with `*.big`.
- Resolve deterministically: the direct form first, then `game/`.
- If both forms hold an archive of the same name, and not as one identical path, refuse as ambiguous. Do not take the first.
- This is a contract rule. Research did not measure such a root.

**Duplicates.**
- `read_eb_member` (`assets.rs:1509`), `locate_member` (`gen7_speech_preview.rs:218`) and `list_members` are **decoders and
  locators, not duplicate validators**. `locate_member` takes the first exact path (`find`).
- The contract needs a separate index pass that rejects conflicting duplicates **within one archive** (two front-table entries with
  one path) and **across archives**.
- Byte-equal duplicates may be accepted only with both locations disclosed and a bounded comparison.
- R145 / R146 measured **0** duplicate paths across the 13 EB archives. That is a fact about this install, not a reason to omit the
  check.

**Allocation and streams.**
- `read_eb_member` and `read_member_unwrapped` read the whole member into memory. That is fine for the speech tables and banks:
  the largest non-stream member is 1 614 652 B unwrapped (R145). R146 did not record the largest portrait or head member, so that
  size is not stated here.
- The two `.sbs` streams are 789 000 896 and 129 156 980 B, stored, and must stay on the existing bounded streaming path:
  - `stream_from_archives` locates the member;
  - `read_window` reads only the sample's window from the member offset.
- Validate before streaming:
  - the wrapper is `stored`;
  - `offset + size` lies within the archive length;
  - each window lies within `size`.
- Never hash or copy a whole stream for provenance.

**Missing, unreadable, corrupt.**

| Case | Result |
|---|---|
| Path absent from a **readable** complete index | not present (for a texlib: "no texture library", a valid readable result; 219 of 904 head bundles here) |
| An archive that cannot be listed (for example `gamedata/gps.big`, BIG4 without `.bh`; `gamedata/anim/faceposer.big`, an EB variant not read; receipt 67 §3) | **unknown** for any path not found elsewhere; never "not shipped" |
| A member present with a wrapper that fails to decode, or an unknown wrapper | **error** naming the member; never "stored", never "missing" |
| A member whose range runs past the archive | error |

*Source note:* `locate_member` returns `None` both for "not a member" and for an archive whose table cannot be read. A route built
only on it cannot separate absent from unknown. Receipt 67's containment index does separate them (§3 there).

**Loose-first is Studio policy.**
- Studio prefers a loose file when present. That is an application source-selection rule, chosen so the X360 Legacy loose tree is
  unchanged.
- It is **not** observed Recompiled loading (K20). Label what was read.

**Partial loose folders, per-file fallback.**
- When a loose directory exists but lacks a file, fall back **for that file only**, to the member. Report the file's source.
- Refuse a source that is present but unreadable or incompatible (it fails to parse). Do not silently skip to the archive.
- Add a partial-tree regression: a loose `audio/speech` holding only `pbp_eng_speecheventdef.csi`, with the remaining files from
  members. Every source is reported, and the results equal the all-member route when the loose file is the identical stock file.

**Bank / stream coherence.**
- A preview needs its `.sbr` bank and `.sbs` stream from **one coherent source**: both loose, or both members of the same install.
- If the bank resolves loose and the stream from an archive (or the reverse), refuse unless the bank's declared stream extent fits
  the stream and both are disclosed. A mismatched pair can index wrong offsets: that would be a silent wrong read, not an error.
- Research gives the rule. It does not implement a second resolver.

### Current delivery, read against K21 (receipt 67 working tree; unreviewed; source reading only, no test run)

| Point | Current source | K21 |
|---|---|---|
| `game/` root, chunklzx unwrap for banks | present: `archive_beside` `:277`, `read_member_unwrapped` `:288` (chunklzx / chunkzip / refpack decode errors refuse) | meets |
| Catalogue duplicates across archives | `AudioDir::read` refuses differing bytes and discloses equal copies (`speech_catalogue.rs` ~`:200`) | meets across archives; **within one archive not checked** (`locate_member` `find`) |
| Bank preview duplicates | `bank_from_archives` `:308` returns the first hit in `BANK_ARCHIVES` order | **first match, no duplicate check** |
| Partial loose `audio/speech` | `AudioDir::open` returns `Loose` whenever the directory exists. A required file missing there is an error, with no member fallback | **a partial folder hides every member** |
| Bank / stream coherence | `bank_entries` `:336` and `resolve_stream` `:360` fall back independently | **a loose `.sbr` with a member `.sbs` (or the reverse) is possible and undisclosed as a pair** |
| Unreadable archive | `locate_member` → `None` → treated as not a member | absent and unknown merged on the speech route |
| Large streams | `stream_from_archives` + `read_window`: windowed, not whole-member | meets the allocation rule; the range checks are not verified here |

None of these is reproduced as a failure on the registered install. It has no loose `audio` tree, so every source there is a member.
These are reviewer items, not research implementation.

## K22 — notes placed

Dated notes pointing here were added to [15](15-recomp-loose-override-premise.md) (summary, §3, §4),
[16](16-recomp-speech-read-contract.md) §2 and [17](17-recomp-portrait-head-read-contract.md) §2. The original text is kept as
history.

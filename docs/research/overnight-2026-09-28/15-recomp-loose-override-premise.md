# Handoff 15 (28 Sep) — O12-1 premise: does Legacy Recompiled read loose files, and from where? Static evidence from its own logs; precedence unknown; runtime card R-RECOMP-LOOSE-1

Research lane, 28 September 2026, published 18:46 MDT (system clock). After-receipt-7 prompt, second package. Claim
`research-q14-recomp-loose-override`, written before work began.
- **Sources:** only the registered install's own files, read-only: its two logs, and a names-only listing of the user-data / cache
  folder that the log itself names.
- **Not done:** no launch, no disassembly, no other repository, no writes. No configuration file exists in the install.

## Plain-language summary

*(Correction 1 Oct 01:40, [18](18-after-receipt8-corrections.md) K20: the logs record failed opens only. "First trying" a loose file, and any order against the archive, is **withdrawn**. The card in §4 is replaced by revision 2 in 18 K20, which is pending a user backup / restore plan.)*

When the recompiled game asks for a file such as a goalie-pad texture, its log shows it first trying a **loose file** on the Xbox
"cache" drive. On this PC, that drive is a folder under your Documents (`Documents\nhllegacy\cache`), **not** the game folder. The
log shows no attempt to read loose files from the game folder.

The log only records failures, though. So it cannot say whether a loose file placed in either location would actually win over the
archive copy. That needs a short in-game test, described below. It does not change the developer's current task: Studio should not
offer Recompiled gear installs as loose files (O12-1).

## 1. Sources and hashes

| Source | Identity |
|---|---|
| `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\logs\nhllegacy_001.log` | 526 231 B, 3 588 lines, 2026-06-06 03:42–03:47, SHA-256 `6f3f5243…d94` |
| `…\logs\nhllegacy_002.log` | 114 609 B, 797 lines, 2026-06-15 15:55–15:56, SHA-256 `f8eff53a…863` |
| install root | `nhllegacy.exe` (269 635 584 B, 6 Jun), `rexruntimerd.dll`, `TracyClientrd.dll`, `game\`, `logs\`. **No configuration file** (no ini / toml / json / cfg / xml outside `game\`). The runtime DLLs were not read (binaries, not configuration) |
| `C:\Users\puckh\Documents\nhllegacy\cache` (names only) | `guest\` (`809284.ver`, `admanager_cache_images`, `highlights.sav`), `shaders\shareable\…`. **No** `rendering`, `fe`, `db` or `audio` folders |

## 2. Exact excerpts (log 001; date and thread id trimmed from each line, text otherwise verbatim)

```
[03:42:49.747] [info] [core]   Game directory: H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game
[03:42:49.747] [info] [core]   User data:      C:\Users\puckh\Documents\nhllegacy
[03:42:49.747] [info] [core]   Cache root:     C:\Users\puckh\Documents\nhllegacy\cache
[03:42:50.443] [info] [sys]   Mounted H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game at \Device\Harddisk0\Partition1
[03:42:50.890] [info] [sys] Loading XEX image: game:\default.xex
[03:42:55.168] [error] [fs] ResolvePath(update:\) failed - device not found
[03:42:55.168] [warning] [krnl] [NtCreateFile] FAILED: path='update:\patch.big' -> 0xc000000f
[03:42:55.204] [warning] [krnl] [NtCreateFile] FAILED: path='cache:\data0.big' -> 0xc000000f
[03:43:31.652] [warning] [fs] VFS: entry not found for 'cache:\rendering\goaliepad' (via symlink '\CACHE\rendering\goaliepad') on device '\CACHE'
[03:43:31.652] [warning] [krnl] [NtCreateFile] FAILED: path='cache:\rendering\goaliepad\goaliepad_0.rx2' -> 0xc000000f
```

Failed-path classes, from `path='…'` normalised with digits → N:
- **Log 001:** `cache:\db\nhlng.db` ×149; `cache:\audio\sfx\sfx.ast` ×104; `cache:\rendering\helmet\helmet_N_N.rxN` ×48;
  `cache:\rendering\player\player_N.rxN` ×41; `cache:\rendering\glove\glove_N_N_tm.rxN` ×24;
  `cache:\rendering\goaliepad\goaliepad_N_N_tm.rxN` ×12; `cache:\fe\ion\artassets\teamlogosreflection\rN.big` ×15; and more.
- **Log 002:** `cache:\db\nhlng.db` ×124; `cache:\fe\profile\default.png` ×3; `cache:\<archive>.big` ×2–3; `update:\patch.big` ×1.
- **No `game:\` path** other than `game:\default.xex` appears in either log.

## 3. What this establishes (measured) and what it does not (unknown)

**Measured (log text):**
- The runtime maps the game folder as `game:` (`\Device\Harddisk0\Partition1`).
- It maps `cache:` to a user-data folder (`\CACHE`, cache root under `Documents\nhllegacy`).
- During play it requests **individual loose asset paths under `cache:\`**. Examples: pad / glove / helmet / player textures, the
  database, audio streams, front-end packages.
- It also requests archives under `cache:\` and `update:\patch.big`.

**Inferred, not measured:**
- *(Withdrawn 1 Oct, 18 K20: the order below is not shown by failed-open logs.)*
- These look like the game's original Xbox cache-partition lookups: first a cached or loose copy, then the archive in `game:`.
- The 2026-06-15 run expected `cache:\809284.ver`, and the folder now holds `guest\809284.ver`. That suggests `\CACHE` currently
  resolves to `cache\guest`. **Not verified.**

**Unknown:**
- Whether a loose file present at the `cache:` path is loaded, and whether it wins over the archive member.
- Whether any loose `game:\rendering\…` path is ever consulted. Successful opens are not logged, so their absence from the log is
  **not proof** of no support.
- Whether loose files survive the runtime's own cache writes.

**Consequence for O12-1.** Studio's loose Hockey Shop output would go under the **game folder** (`<anchor>/[_compiled/]rendering`). No
logged request points there. The coordinator's guard stands: refuse Recompiled install plan / apply until a supported archive writer
or a proven loose-loader contract exists. This handoff supplies no such proof.

## 4. User runtime card R-RECOMP-LOOSE-1 (not executed by research)

*(Superseded 1 Oct 01:40 by revision 2, [18](18-after-receipt8-corrections.md) K20. It places one location per trial, withdraws `fe/profile/default.png` as the default and withdraws control C. Copying the install does not isolate `Documents\nhllegacy`. Kept below as history; do not execute.)*

**Goal:** decide whether a loose file overrides the archive copy, and from which location. **Not in scope:** save compatibility and
roster limits (separate cards).

**Asset:** `fe/profile/default.png`.
- Log 002 shows it requested as `cache:\fe\profile\default.png`, and it is shown on the profile / front-end screen.
- Take the stock bytes from the X360 Legacy loose tree `fe\profile\default.png`, which R138 recorded as an archive-member path match.
  Hash them.

**Steps:**
1. **Copy and baseline.**
   - Copy the whole install to an owned folder, e.g. `…\NHL Legacy Recompiled LOOSE-TEST`.
   - Record SHA-256 values for every `game\*.big` in the copy.
   - The runtime's user-data folder is shared: `Documents\nhllegacy`. Record a full file listing with hashes of
     `Documents\nhllegacy\cache`, and copy that folder aside as its backup.
2. **Baseline observation.** Launch the **copy**, go to the screen that shows the default profile picture, note or photograph it, and
   exit. Keep the new log.
3. **Change A: the `cache:` location.**
   - Make an unmistakable replacement with the same size and PNG format: a solid magenta image with a large "A".
   - Place it at `Documents\nhllegacy\cache\guest\fe\profile\default.png`, **and** at
     `Documents\nhllegacy\cache\fe\profile\default.png`, because the mapping in §3 is unverified.
   - Record both hashes, launch, observe, exit.
   - **Verify the hashes are unchanged after exit**, and check whether the new log still shows `FAILED … cache:\fe\profile\default.png`.
4. **Control C: loader versus asset choice.**
   - Put a **byte-identical stock** copy at the same paths and launch.
   - If the log's `FAILED` line for that path disappears, the loader **found** the file.
   - If the loader found it (C) but change A showed no difference, the screen draws a different asset. That is wrong asset selection,
     not missing loader behaviour.
5. **Change B: the game-folder location.**
   - Remove the `cache:` copies.
   - Place the "B" magenta image at the copy's `game\fe\profile\default.png`.
   - Launch, observe, exit, and verify the hash again.
6. **Restore.**
   - Delete every placed file.
   - Restore `Documents\nhllegacy\cache` from the backup, and confirm the listing and hashes equal the baseline.
   - Confirm the copy's `game\*.big` hashes are unchanged.
   - Delete the copied install.

**Reading the result:**

| Observation | Meaning |
|---|---|
| A shows magenta, B stock | loose override works from `cache:` only |
| B shows magenta | loose override from the game folder (then precedence against `cache:` still needs A + B together) |
| A / B stock, C found | the file is read, but that screen uses another asset: choose another asset and repeat |
| A / B stock, C still FAILED | this path is not served at that location (mapping unknown) |

**One unchanged result does not prove loose overrides never work.** It rules out only that asset, at that location, in that build.

## 5. Next

Research stops here on O12-1. The static sources are exhausted within bound: two logs, the folder listing, and no configuration. The
card belongs to the user. The developer guard is unaffected.

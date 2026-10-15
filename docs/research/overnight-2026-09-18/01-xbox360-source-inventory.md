# Handoff R0 — Xbox 360 source inventory and corrections (X:\Games\Emulation\Roms\XBOX360)

Research lane, overnight 2026-09-18. Claim `r0-xbox360-source-inventory`. Consumer: developer work-order
item 2 (integrate Xbox installations; Legacy Recompiled `_compiled`; NHL 12 input; League parity on Xbox rows).

Machine-readable (all in [`data/`](data/)):
`xbox360-sources.json` (full file walk of the ten title roots: every path, size, magic for archive/db/xex
files, SHA-256 of db/loc/xex/.bh and every file ≤ 4 MB, XEX2 execution ids, ISO sniff),
`xbox360-key-files.csv` / `xbox360-archives.csv` (derived), `xbox360-key-member-map.json` (where each
database/metadata/dictionary lives — loose and archive member — with hashes vs anchors),
`iso/<title>-xdvdfs.json` (full XDVDFS directory of each of the seven ISOs, no extraction),
`xbox360-anchor-hashes.txt`, `xbox360-league-code-key-tables.json` (see handoff P1.1-X).
Probes: `local/runs/overnight-research-2026-09-18/probes/{xbox360_inventory,xiso_list,eb_list,big4_list,member_map,xex_image}.py`.

Evidence: **observed** unless marked *inferred* / *by-reference*. Nothing launched; nothing under `X:` or
`H:` written.

## 0. Plain-language summary and corrections

**Correction of the batch/completion records:** NHL 07, 08, 09, 10, 11, 12, 13, 14 and 15 for Xbox 360 **are
installed** on this machine, as extracted title-root trees under `X:\Games\Emulation\Roms\XBOX360\<NHL nn>\`
(each containing `default.xex`, the `.big` archive set and loose `db/`, `fe/`, `rendering/`, `audio/` …
directories), and NHL 10–15 and Legacy additionally have their original **XGD2/XGD3 ISOs** next to the tree.
Every `default.xex` carries the expected retail title id (`454107F3` … `454109EC`), all are disc 1/1 base
executables (`base_version == version`; no title-update `.xexp` anywhere), and **every database, metadata and
dictionary file — loose and inside the archives — is byte-identical to the repository's Xbox 360 anchors.**
So the rows the completion record marked "not installed" (12, 14, 16, 18, 20, 22, 24, 26, 27) now have a
concrete, vanilla, hash-bound source; the "anchor-only" qualifier can be dropped for them.

Three things the developer must know before treating a tree as a launch source:

1. **Discs NHL 11 → Legacy ship no loose `db/` or `fe/loc/`** — only the `.big` set + `gamedata/` (ISO
   listings). The loose directories in those X: trees are *extractions* (identical bytes, but not what the
   disc has). NHL 10's disc *does* ship `db/nhlng.db` and `fe/loc/*.db` loose (plus streamed audio/video), and
   for NHL 07–10 the archives contain only the RefPack'd `-meta.xml`s — the DB and dictionary are loose
   originals there. Which copy Xenia's guest actually reads when both exist is a runtime question (see §4).
2. **`cache:` shadowing.** Xenia's cache dir `C:\Users\puckh\Documents\xenia\cache\` holds a game-made
   install of **NHL 09** (its `.bh/.big` set, `db/nhlng.db` = anchor, `gamedata/nis.*`, all 2026-07-17
   17:07) plus eight `<build>.ver` markers. The recompiled Legacy log shows the engine opening
   `cache:\<archive>` before `game:\` (failing over on `0xc000000f`). A candidate placed in the NHL 09 tree
   can therefore be shadowed by the cache copy; and NHL 09 booted to profile creation on 2026-08-05 with
   `player.big` absent from its tree (renamed `player.big.bak`, byte-identical to the cache's `player.big`) —
   consistent with cache precedence (*inferred*, one observation).
3. **NHL 09's tree is not pristine:** `player.big` is renamed `player.big.bak` (2026-07-17; vanilla bytes
   `d7c915ff…`, identical to the cache copy); `rendering/player/head_208_0_0_0_cm.rx2` was **modified on
   2026-08-05 20:37** (its `.bak` sibling of 07-17 is the extracted original; `head_208.rx2` is unchanged) —
   a user head-texture experiment, evidently relying on loose `rendering/` files being read when
   `player.big` is absent. `cl.ini` (`180119`) and `version.txt` are present. Every DB/dictionary equals
   the anchors. All other titles: only the prior-tool `.nhlmodproj` sidecars post-date extraction
   (`xbox360-sources.json` holds every hash; the `lobbyimg/backup.big` files are shipped assets, not backups).

Legacy Recompiled `_compiled`: **not found in its installation.** See §5.

## 1. Per-title identity (observed from `default.xex` XEX2 headers and the trees)

| Studio id | Tree (`X:\…\XBOX360\`) | Title id · media id | version / base | regions | XEX enc/comp · image | `default.xex` SHA-256 | Files · bytes (tree) | ISO |
|---|---|---|---|---|---|---|---|---|
| nhl07 | `NHL 07\NHL 07 (USA)\` | `454107F3` · `610CF9FA` | 1 / 1 | `0x000000FF` (NTSC) | enc 1 · **basic** · `nhl07rwf.pe` · 0xAE0000 | `0fc761b2…` (10 858 496) | 15 614 · 8.31 GB | none |
| nhl08 | `NHL 08\NHL 08 (USA)\` | `45410820` · `45CE10FB` | 1 / 1 | `0x000000FF` | enc 1 · **basic** · `nhlzf.exe` · 0xE60000 | `9386dba8…` (13 185 024) | 19 338 · 8.26 GB | none |
| nhl09 | `NHL 09\NHL 09 (USA)\` | `45410872` · `10719FCC` | 2 / 2 | `0x000000FF` | enc 1 · LZX · 0x12E0000 | `dcaf909c…` (6 516 736) | 20 761 · 13.80 GB | none |
| nhl10 | `NHL 10\NHL 10\` | `454108BA` · `20D374A2` | 1 / 1 | `0x000000FF` | enc 1 · LZX · 0x1650000 | `ccd996d6…` (7 774 208) | 20 885 · 13.70 GB | `NHL 10 (USA).iso` XGD2 |
| nhl11 | `NHL 11\NHL 11\` | `454108FD` · `23FE7FC9` | 2 / 2 | `0x000000FF` | enc 1 · LZX · 0x1820000 | `e769d808…` (8 491 008) | 29 157 · 15.29 GB | `NHL 11 (USA) (En,Fr).iso` XGD2 |
| nhl12 | `NHL 12\NHL 12\` | `45410964` · `41AF51B7` | 4 / 4 | `0xFFFFFFFF` (all) | enc 1 · LZX · 0x1A10000 | `9950cd88…` (9 109 504) | 35 750 · 18.17 GB | `NHL 12 (USA, Europe) (…).iso` XGD2 |
| nhl13 | `NHL 13\NHL 13\` | `4541099B` · `583EFE34` | 2 / 2 | `0x00FF00FF` | enc 1 · LZX · 0x1BB0000 | `17caa7b3…` (9 826 304) | 36 423 · 19.11 GB | `NHL 13 (USA, Europe).iso` XGD2 |
| nhl14 | `NHL 14\NHL 14\` | `454109BE` · `4AB5AEF5` | 2 / 2 | `0xFFFFFFFF` | enc 1 · LZX · 0x1E80000 | `5dfbb082…` (10 436 608) | 40 590 · 20.71 GB | `NHL 14 (USA, Europe) (…).iso` XGD2 |
| nhl15 | `NHL 15\NHL 15\` | `454109DC` · `5F460E79` | 5 / 5 | `0xFFFFFFFF` | enc 1 · LZX · 0x1EA0000 | `ca73955a…` (10 457 088) | 40 858 · 18.87 GB | `NHL 15 (USA).iso` **XGD3** |
| nhllegacy | `NHL Legacy\NHL Legacy\` | `454109EC` · `3CF2A23F` | 4 / 4 | `0xFFFFFFFF` | enc 1 · LZX · 0x1EA0000 | `d818eb15…` (10 448 896) | 44 622 · 19.55 GB | `NHL Legacy Edition (USA, Europe) (…).iso` **XGD3** |

Notes. "version / base" is the raw execution-id `version`/`base_version` word (small integers on these
retail discs; not a title-update number). Media ids match the ones Xenia logged (NHL 09 `10719FCC`) and
AS7 (Legacy `3CF2A23F`). The ISO-embedded `default.xex` and the key archives (`boot.big`+`fe.big` on 11,
`cache.big` on 12/13, `cacheboot.big` on 14/15/Legacy) are byte-identical to the tree copies (verified by
extracting the ISO members with `xiso_list.py --extract`). Full hashes are in `xbox360-sources.json`.
Studio's `discovery::title_ids` already carries exactly these ten title ids.

Also present in the ROMs root and **out of scope**: `NHL 2K10 (Europe)` (folder + 7z), `Archives\` (7z copies
of the 11–15 discs and 2K10 — not inspected beyond listing; the active installation is the extracted tree
plus its sibling ISO), `pavol demitra.png`. Each title root carries a prior-tool `.nhlmodproj` sidecar
(826–858 B; recorded, **not read**).

## 2. Container families (observed magics)

| Family | Titles | Archive form | Notes |
|---|---|---|---|
| **BIG4 + Viv4** | NHL 07, 08, 09, 10 | `<name>.big` = `BIG4` (embedded directory: u32 LE size, u32 BE count, u32 BE index size; entries u32 BE offset/size + name) with a sibling `<name>.bh` = `Viv4` index | `boot.big` also embeds the `dataN.bh` indices. Payloads: raw or RefPack (`10FB`). The DB / dictionaries are **not** inside these archives (only their `-meta.xml`, RefPack'd) — they ship loose. |
| **EB `EB\0\3` v`0x00000400`, 16-byte rows** | NHL 11, 12, 13, 14, 15, Legacy | `<name>.big` only (no `.bh`) | Same version word and row layout as the Legacy archives Studio already binds (`KNOWN_VERSION_TAGS`), so `tdb-archive-big` should open **NHL 11–15** unchanged — *inferred from the header/row parse in `eb_list.py`; a Studio-side bind per title is the cheap check*. Payloads raw / `chunklzx` / `StrE`/`Vers` attribdb blobs. |

Archive set per title (top level): 07 `boot, data0..5`; 08 `audioboot, boot, data0, data1`; 09 `audioboot,
boot, data0, fe, player, rendercache, rendernocache, stadium`; 10 `audio, audioboot, audioboot2, boot, data0,
fe, player, rendercache, rendernocache, stadium`; 11 `audioboot, audioboot2, audiostream, boot, data0, fe,
rendercache, rendernocache, stadium, transitions`; 12 `audioboot, audioboot2, boot, cache, cacherender,
data0, nocache, nocacherender`; 13 = 12 + `cacheaudio, cachemovie, cacheoverlay`; 14/15/Legacy = 13 +
`cacheboot, renderboot`. Sizes in `xbox360-archives.csv`. `gamedata/{aidata,gps,nis}.big` and
`gamedata/anim/faceposer.big` are loose on every disc.

## 3. Key-member map (where the DB, metadata and dictionaries live) — all equal to anchors

| Title | `db/` | `fe/loc/` (English) | Other locales present |
|---|---|---|---|
| nhl07 | loose `NHL07NG.db` `67a00bea…`, `NHL07NG-meta.xml`; `data1.big` carries `fe/loc/nhl_eng_us-meta.xml` + `nhl_eng_us-databasehashedattributes.h` | loose `nhl_eng_us.db` `5d3c0b01…` (3 952 rows) | — |
| nhl08 | loose `nhlng.db` `587204c3…`, meta | loose `nhl_eng_us.db` `308021e7…` | — |
| nhl09 | loose `nhlng.db` `0f7c826b…`; `boot.big` carries RefPack'd `db/nhlng-meta.xml` | loose `nhl_eng_us.db` `d9e0463b…` | cze, fin, fre, ger, rus, swe (loose) |
| nhl10 | loose `nhlng.db` `e87b8c80…` (= disc); `boot.big` carries RefPack'd meta | loose `nhl_eng_us.db` `f1a9a1dd…` (= disc) | fre (loose, = disc) |
| nhl11 | **`fe.big`** #0 `draft_roster.db`, #1 `nhlng-meta.xml`, #2 `nhlng.db` `461f8d64…` (raw) + loose copies | **`boot.big`** #96 `nhl_eng_us.db` `82520ab3…` (raw) + loose | fre |
| nhl12 | **`cache.big`** #32 `draft_roster.db`, #33 meta, #34 `nhlng.db` `94b31ff2…` + loose | `cache.big` #3 `nhl_eng_us.db` `354a1905…` + loose | cze, fin, fre, ger, lng, rus, swe |
| nhl13 | `cache.big` #30/#31/#32 (`nhlng.db` `64460a15…`) + loose | `cache.big` #3 `b93fa83f…` + loose | same 8 |
| nhl14 | **`cacheboot.big`** #47 `draft_roster.db`, #48 `dynamic_ratings.db`, #49 meta, #50 `nhlng.db` `84e1bdff…` + loose | `cacheboot.big` #3 `e5aff828…` + loose | same 8 |
| nhl15 | `cacheboot.big` #47–#50 (`nhlng.db` `028ae06a…`) + loose | `cacheboot.big` #3 `86167614…` + loose | same 8 |
| nhllegacy | `cacheboot.big` #47–#50 (`nhlng.db` `105d2186…`) + loose | `cacheboot.big` #3 `f00926ce…` + loose | same 8 |

`nhl_lng_lg.db` (840 084 B, `dc9d319a…`) is byte-identical across NHL 12, 13, 14, 15 and Legacy — evidently
a title-independent key/language table (not decoded tonight; candidate for the 1.3 localization map).
Every `.db` member is stored raw (magic `DB\0\x08`), so the S2-A same-length member replacement applies to
NHL 11–15 exactly as to Legacy. Offsets/sizes per member: `xbox360-key-member-map.json`.

## 4. Launch registration and runtime source precedence (what could be read without launching)

- **Xenia** (`H:\Emulators\Xenia\xenia.exe`, build `master@95a5c3ee2`, 2026-02-18): `xenia.config.toml` has
  empty `storage_root/content_root/cache_root` (defaults under `Documents\xenia`), `mount_cache = false`
  **now**, `license_mask = 0`, no recent-path list. `xenia.log` (54 MB) records only the **last** launch:
  NHL 09 (`45410872`, media `10719FCC`), mounted as `\Device\Harddisk0\Partition1\default.xex` — the host
  path is not logged. No `.xexp` patch was found (`default.xexp` probe fails in the log).
- **Evidence all ten titles have booted** (dates): Xenia host module cache `Documents\xenia\cache_host\modules\`
  has ten module hashes (June 6/23/26, July 17), and `Documents\xenia\content\` has saves for `45410820`
  (NHL 08, 07-18), `45410872` (NHL 09, 08-05) and `454109EC` (Legacy, 08-06); the repository's
  `anchors/xbox360/*/savefiles` were produced 06-27 (10–Legacy) and 07-17/18 (07–09).
- **`cache:` contents** (`Documents\xenia\cache\`): `180119.ver` (= NHL 09 `version.txt`), `249123.ver`,
  `308315.ver`, `365472.ver`, `443858.ver`, `467414.ver`, `669669.ver`, `809284.ver` (build ids of the other
  booted titles — mapping not established; no build string is embedded in the executables), the NHL 09
  cache install (§0.2), `highlights.sav`, `shaders/`. The engine's `cache:`-before-`game:` probe order is
  observed in the recompiled log (same engine family); whether Xenia's current `mount_cache=false` hides
  the directory from the guest is **unknown** — it is the first thing to check tomorrow (hash
  `cache\db\nhlng.db` before/after a stock boot; or move the cache copy aside, user decision).
- **Loose vs. archive precedence** for NHL 11–15/Legacy trees that carry both: not established (same open
  question as the H: Legacy trees). The disc never has loose copies, so an archive-member candidate is the
  disc-faithful route; a loose candidate is the extraction-tree route. Both are structurally available.
- **NHL 10–15 ISO route:** RPCS3-style ISO registration does not apply to Xenia; nothing inspected records
  whether the user launches the tree or the ISO. Developer claim `p1.1-xbox360-iso-member-route` covers
  the ISO read path; the XDVDFS listings here (`data/iso/*.json`) give every member's sector/size for a
  cross-check (root dir sector/size in each file's header block; XGD2 base `0xFD90000`, XGD3 `0x2080000`).

## 5. Legacy Recompiled — `_compiled` not found; what exists

- Install: `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\` — `nhllegacy.exe` (269 635 584 B,
  2026-06-06, SHA-256 `7264bec6…` per the batch), `rexruntimerd.dll`, `TracyClientrd.dll`, `game\`
  (the Legacy `.big` set + `gamedata\` + `default.xex` `d818eb15…` — identical to the X: tree and the
  ISO; `cacheboot.big` `b558b4f6…`), `logs\nhllegacy_00{1,2}.log`.
- **No `_compiled` directory exists** under that install, under `Documents\nhllegacy`, or anywhere else on
  `H:\Emulators`, `E:\Projects`, `E:\Personal Projects`, `C:\Users\puckh\Documents` or the X: ROM root
  **except** `H:\Emulators\games\XBOX\NHL Legacy - Vanilla\_compiled\` (2026-05-30), which is a full
  extraction (`db/`, `fe/`, `rendering/`, …) beside a `xenia-file-source-manifest.txt` (45 238-line tree
  listing headed `Extracted BIG Files`) and an `_loose_tex\` dir. The June recompiled binary contains no
  `_compiled` / `_loose` string at all; its runtime strings show `--game_data_root`, `Game directory`,
  `User data`, `Update data`, `Cache root`, `\CACHE`, `cache:`.
- Runtime layout from `nhllegacy_002.log` (2026-06-15): game dir mounted at `\Device\Harddisk0\Partition1`;
  user data `C:\Users\puckh\Documents\nhllegacy`; cache root `…\nhllegacy\cache`; the game probes
  `update:\patch.big` (absent) and `cache:\*.big` (absent) before `game:\`.
- **Recompiled saves** (Xenia-like layout + a `Headers\` sidecar):
  `C:\Users\puckh\Documents\nhllegacy\B13EBABEBABEBABE\454109EC\00000001\<TYPE ts>\<TYPE ts>` and
  `…\454109EC\Headers\00000001\<TYPE ts>.header`; latest activity 2026-09-15 (BUILDYOURAI), ROSTER
  2026-08-14, DYNASTY ×2 2026-08-15 — the June binary is in active use. Studio's `detect_recomp` takes an
  explicit save path; this is that path.
- **Disposition (reworded per R1 item 4).** `_compiled` is a **discovery/path convention supplied by the
  user**, not something this lane derived from the binary; the observations above describe **one inspected
  instance** (the June 2026 `NHL Legacy Decomp Test` install) and say nothing about other or newer builds.
  Supported candidate root layouts, with provenance:

  | Candidate root | Observed where | Contents / provenance |
  |---|---|---|
  | `<install>\game\` — archive route | `NHL Legacy Decomp Test\game\` (observed) | the Xbox Legacy `.big` set + `gamedata\` + `default.xex` `d818eb15…`, byte-identical to the X: tree and the ISO |
  | `<game_data_root>\_compiled\<loose tree>` — loose route | `NHL Legacy - Vanilla\_compiled\` (observed, 2026-05-30, beside `xenia-file-source-manifest.txt` "Extracted BIG Files" and `_loose_tex\`) | a full extraction (`attribdb db fe gamedata ice rendering scenario scrape shaders sku …`) — provenance: user-made extraction of the same archives; not produced by the inspected binary |
  | `<game_data_root>\_compiled\` under the Decomp Test install | **absent** in the inspected instance | the user's statement (convention); expected to hold the same loose members |
  | `cache:` / `update:\patch.big` | probed by the engine before `game:` (log) | absent in the inspected instance |

  Identities bind from `game\` (Xbox Legacy bytes) on every candidate; loader precedence between `_compiled`
  loose members and archive members is a runtime question. Only the developer changes discovery; this
  lane does not ask for a user investigation before the shared Xbox format work continues.

## 6. Other recorded instances (unchanged, by-reference from the batch)

`H:\Emulators\games\XBOX\NHL Legacy\` (loose tree, no `.big`), `NHL Legacy - Vanilla\` (`.big` set +
`_compiled` + `Extracted BIG Files`), `NHL Legacy - Modded\` (not inventoried), plus `default.decrypted.xex`
/ `default.decompressed.exe` (user-made, raw PE layout) in `H:\Emulators\games\XBOX\`. All four Legacy
`default.xex` copies on H: and X: are `d818eb15…`. These remain additional instances of the same source,
not competing assumptions.

## 7. Disposition table for the completion-record rows

| Rows | Old status | New status (research) | What development needs to do |
|---|---|---|---|
| 12 NHL 15 X360, 14 NHL 14, 16 NHL 13, 18 NHL 12, 20 NHL 11 | not installed | **installed** — tree + ISO; DB/dict = anchors; DB and dictionary are raw members of one EB v0x400 archive (`cacheboot.big` / `cache.big` / `boot.big`+`fe.big`) | bind the title's archive through the existing Legacy stager; mint the LOC-W1 dictionary candidate (key tables now **traced**, see P1.1-X); decide loose-vs-member placement per §4 |
| 22 NHL 10, 24 NHL 09, 26 NHL 08 | not installed | **installed** — tree (+ ISO for 10); DB/dict loose (also loose on the NHL 10 disc) | loose-file candidate route (S2-C style); NHL 09: resolve the `cache:` copy and `player.big.bak` first |
| 27 NHL 07 | not installed; label source untraced | **installed**; label keys partially traced (P1.1-X §2) | browse/classify on the install; `CHTM_LEAGUE_*` source still open |

## 8. Cleanup

Scratch retained for the night (regenerable): decrypted executable images `local/runs/overnight-research-2026-09-18/scratch/xex/*.bin`
(~200 MB, ~30 s each to rebuild). ISO member extractions were deleted after hashing. Nothing else written
outside the research lane.

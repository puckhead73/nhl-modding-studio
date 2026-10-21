# EA NHL Series Overview

**Evidence:** measured on the reference databases and on installed copies of every title (Xbox 360 NHL 07 through Legacy, PlayStation 3 NHL 08 through Legacy, PlayStation 4 NHL 15 through 21); family groupings are the project's own, built from those measurements.

EA's NHL games from NHL 07 to NHL 21 share one database technology and a small number of archive and save designs, but the details change in steps. Knowing which **family** a title belongs to tells you most of what you need before opening its files.

## The families

| Family | Titles | Database | Archives | Front-end art |
|---|---|---|---|---|
| **gen6** | NHL 07 (X360) | `NHL07NG.db`, loose, with its own schema shape (no equipment companion tables, a two-level team table) | `BIG4` with `.bh` sidecar indexes; `.bh` copies also packed inside `boot.big` | RefPack-compressed `BIGF` packages holding raw RW4 |
| **gen7 BIG4** | NHL 08, 09, 10 (X360 and PS3) | `nhlng.db`, loose | `BIG4` + `.bh` | RefPack `BIGF` with raw RW4 |
| **gen7a** | NHL 11 (X360) | `nhlng.db` packed in `fe.big`, dictionary in `boot.big` | EB archives (16-byte rows) | `BIGF` → RefPack → RW4 |
| **gen7b** | NHL 12, 13, 14, 15, Legacy (X360) | `nhlng.db` and dictionary packed in `cache.big` (12, 13) or `cacheboot.big` (14 on) | EB archives (16-byte rows) | `BIGF` → RefPack → XPR2 |
| **PS3 (08 to Legacy)** | NHL 08 through Legacy | main database byte-identical to the Xbox 360 release; dictionaries are **separate files** on every title | BIG4 (08 to 10) or EB (12 to Legacy) with the same member names; `chunkzip` payloads | DDS payloads on 12 to Legacy; RW4 for PS3 on 08 to 11 |
| **Legacy Recompiled** | NHL Legacy, PC | identical bytes to the Xbox 360 Legacy release | identical | identical |
| **Kettle (PS4)** | NHL 15 through 21 | `nhl_game.db`, little-endian, packed inside `inxp_00.big` | EB archives: NHL 15 uses a 16-byte-row variant, NHL 16 on a 20-byte-row variant | NHL 15 PNG league logos; NHL 16 on `chunkzip` → GNF (BC7) |
| **Frostbite** | NHL 22, 23, 24 (PS4) | a different engine with superbundles and a layered table of contents; no TDB database | Frostbite bundles | Frostbite |

"gen6", "gen7" and "Kettle" are the project's working names; the games themselves do not use them.

## Product codes

The games are identified by codes the platforms themselves expose. These are public and appear in folder names, so Studio uses them to recognise an installation.

| Title | Xbox 360 title id | PS3 (North America) | PS3 (Europe) | PS4 (known ids) |
|---|---|---|---|---|
| NHL 07 | 454107F3 | | | |
| NHL 08 | 45410820 | BLUS30062 | | |
| NHL 09 | 45410872 | BLUS30168 | | |
| NHL 10 | 454108BA | BLUS30345 | BLES00567 | |
| NHL 11 | 454108FD | BLUS30568 | BLES00984 | |
| NHL 12 | 45410964 | BLUS30791 | BLES01357 | |
| NHL 13 | 4541099B | BLUS31003 | BLES01691 | |
| NHL 14 | 454109BE | BLUS31179 | BLES01853 | |
| NHL 15 | 454109DC | BLUS31436 | BLES02037 | CUSA00561, CUSA50061 |
| NHL 16 | | | | CUSA01909 |
| NHL 17 | | | | CUSA04326, CUSA04330 |
| NHL 18 | | | | CUSA07569, CUSA07580 |
| NHL 19 | | | | CUSA11126 (North America) |
| NHL 20 | | | | CUSA15439, CUSA15447 |
| NHL 21 | | | | CUSA18182 |
| NHL Legacy | 454109EC | BLUS31540 | BLES02153 | |

PlayStation 4 ids are region-specific, so the list is not exhaustive; the game folder's own `sce_sys/param.sfo` is the authority (title, id, region prefix `UP` North America, `EP` Europe, `JP` Japan, `HP` Asia, and the patch level in `APP_VER`).

## Where the editable data lives

| Platform | Main database | Metadata | Dictionary |
|---|---|---|---|
| Xbox 360 | `db/nhlng.db` (NHL 07: `NHL07NG.db`), loose on 07 to 10, packed from 11 | `nhlng-meta.xml` beside it (NHL 10 ships none) | `fe/loc/nhl_eng_us.db`, packed on 11 and later |
| PS3 | `USRDIR/db/nhlng.db`, loose on every title | beside it | loose on every title, and also inside `cache.big` or `cacheboot.big` on 12 to Legacy |
| PS4 | `nhl/data/common/gamemodes/database/t3db/nhl_game.db` inside `inxp_00.big` | `nhl_game-meta.xml` beside it | `nhl/data/kettle/loc/eng_us.db` in the same archive; NHL 15 and NHL 19 ship patched copies in `patch_inxp*.big` |

The PS4 main database is the counterpart of `nhlng.db`: it holds the default rosters the game installs from, and roster saves hold a separate, smaller copy. Beside it the PS4 titles ship `entry_draft.db`, `expansion_draft.db`, `initial_experience.db`, `threes_campaign.db` and, on NHL 20, `proam.db`, all strict subsets of the main schema.

## Byte order

Xbox 360 and PlayStation 3 are big-endian PowerPC machines; their TDB files are big-endian and their bit-packed records are read most-significant bit first. PlayStation 4 is x86-64; its TDB files are little-endian and read least-significant bit first. The byte order is declared in every file's header, so a reader never has to guess from the platform. See [TDB Database Format](TDB-Database-Format).

## Locales

| Family | Dictionaries shipped |
|---|---|
| Xbox 360 NHL 07, 08 | English only |
| Xbox 360 NHL 09 | English, French, German, Russian, Czech, Finnish, Swedish |
| Xbox 360 NHL 10, 11 | English, French |
| Xbox 360 NHL 12 to Legacy, Recompiled | the seven above plus `lng_lg`, an all-caps language template |
| PS3 NHL 08 | English only |
| PS3 NHL 09, 10, 11 | English, French |
| PS3 NHL 12 to Legacy | the eight above |
| PS4 NHL 15 to 20 | seven (Czech, English, Finnish, French, German, Russian, Swedish) |
| PS4 NHL 21 | seven plus `lng_lg` |

Last reviewed: 2026-10-20

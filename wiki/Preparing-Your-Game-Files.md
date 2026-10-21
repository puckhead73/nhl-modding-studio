# Preparing Your Game Files

Studio works on a copy of the game that lives on your PC as ordinary files. This page describes what that copy has to look like for each platform and how Studio finds the pieces it needs. It does not cover dumping discs or copying files off a console; use your own legally obtained copies and the documentation of your emulator or console tools for that.

## What Studio needs

For every game Studio wants three things, and it finds them for you once you point it at the right folder:

- the **main database** (`nhlng.db`, `NHL07NG.db` on NHL 07, `nhl_game.db` on PlayStation 4);
- its **metadata file** (`…-meta.xml` beside the database), which describes the database layout. If the game ships without one, Studio uses its bundled schema;
- the **localization dictionary** (`nhl_eng_us.db`, or `eng_us.db` on PlayStation 4), which holds team and league names and other text.

For asset editing (logos, jerseys, portraits, heads, audio) Studio also needs the game's **archives** (`.big` files) extracted into loose files. Studio can do that extraction itself; see below.

Saves are separate. The Modding Studio edits the game's default database; saves are the [Save Editor](Save-Editor)'s job.

## Two ways to start a project

**Create New Project** starts from Studio's bundled reference copy of the game's database and asks for a **game source**: either a game ISO, which Studio extracts into a destination folder you choose, or a folder where the game is already dumped. Which option is offered depends on the platform:

| Platform | Game source options |
|---|---|
| PlayStation 3 | A decrypted ISO (Studio extracts the game files), or a dumped ISO folder. |
| Xbox 360 | A dumped ISO folder containing `default.xex`. The Xbox import flow can also open a staged ISO directly. |
| PlayStation 4 | A dumped game folder, or a PKG that Studio extracts. |
| Legacy Recompiled | The recompiled game folder. |

For ISO extraction, pick a new or empty folder on a drive with enough space. Extraction starts after you click **Next** on the Project Setup screen and a cancelled extraction can be retried at the same location.

**Import non-studio project** takes files you already have: any one of a main database, a localization dictionary or a folder of game assets (packed or extracted) is enough. Studio lists each file with a ✓ or ✗ and a reason before it creates the project. Files are read sources: they are copied into the project and never written.

## Folder layouts Studio recognises

### Xbox 360

Point Studio at the folder that holds `default.xex`. The title id inside that file names the game. On NHL 10 and later the database is packed inside the game's `.big` archives; NHL 08 and NHL 09 ship the database loose but keep the dictionary packed. Either way Studio offers extraction when it is needed.

### PlayStation 3

Any of these works: a disc root, its `PS3_GAME/USRDIR` folder, a bare `USRDIR`, or an RPCS3 folder (the one containing `dev_hdd0`). Inside an RPCS3 folder Studio looks under `dev_hdd0/game/<product code>/USRDIR`, where the product code is the game's `BLUS…` (North America) or `BLES…` (Europe) id. The database and dictionary are loose files.

### PlayStation 4

Point Studio at the game folder itself, or at a parent folder that holds several game folders. Studio never trusts a folder's name: it reads each folder's `sce_sys/param.sfo` and reports the title, the `CUSA…` id, the region (North America, Europe, Japan, Asia) and the installed patch version, then picks the one that matches the title you chose. If more than one folder matches, Studio asks you to choose with a **Use this folder** link. The database lives deep inside the game's data tree and is usually packed; Studio locates it after extraction.

### NHL Legacy Recompiled

Point Studio at the recompiled game folder. It is packed the same way as the Xbox 360 build and uses the same database and save formats, so extraction is offered in the same way. The recompilation has no fixed save location, so you tell Studio where saves are when a tool needs them.

## Extracting game containers

When a project's game source still has its database or assets packed, the wizard shows **Extract game assets now?** after creating the project. The step first runs a **Game file verification**, comparing the source against the files Studio expects for that game; if files differ or cannot be checked you can still continue with **Continue with modified files** or **Continue with unverified files**. Extraction then unpacks the archives beside the game. It needs free disk space and can be paused and resumed; if it finishes with skipped files Studio says so.

You can skip this step and run it later from **Settings › General › Tools › Extract game containers**. Until extraction has happened, asset-editing controls show the note "Container extraction is required to edit this asset. Run Extract game containers in Settings → Tools."

Extraction is the only time Studio writes inside your game folder during project creation. Everything you edit afterwards lives in the project folder until a mod is activated with the Mod Manager.

Last reviewed: 2026-10-20

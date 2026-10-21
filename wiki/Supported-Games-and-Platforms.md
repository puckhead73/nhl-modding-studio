# Supported Games and Platforms

Studio targets 17 games across four platforms: every EA NHL title from NHL 07 to NHL 21, plus NHL Legacy and the NHL Legacy Recompilation. Games are supported only on the consoles they actually shipped on, which gives 27 title-and-platform combinations. There is no Xbox One version of any of these games, and no PC version except the Recompilation.

## What "supported" means right now

Every combination in the table below is **implemented and automated-tested**: Studio's engine reads and edits that game's database, and automated tests assert the result against a reference copy of the game's data. **No combination is game-verified yet**, meaning nobody has recorded a run where Studio's edited output was loaded by that exact game on that exact platform and behaved correctly. That final step is a manual gate the project has not passed for any title, and the wiki will say so until it does.

The wizard's own footer puts it this way: "Beta — built and verified from NHL 07 through NHL Legacy. PS4 titles are still a work in progress."

## The games

| Game | Xbox 360 | PlayStation 3 | PlayStation 4 | Legacy Recompiled |
|---|:---:|:---:|:---:|:---:|
| NHL 07 | yes | research only | | |
| NHL 08 | yes | yes | | |
| NHL 09 | yes | yes | | |
| NHL 10 | yes | yes | | |
| NHL 11 | yes | yes | | |
| NHL 12 | yes | yes | | |
| NHL 13 | yes | yes | | |
| NHL 14 | yes | yes | | |
| NHL 15 | yes | yes | yes | |
| NHL 16 | | | yes | |
| NHL 17 | | | yes | |
| NHL 18 | | | yes | |
| NHL 19 | | | yes | |
| NHL 20 | | | yes | |
| NHL 21 | | | yes | |
| NHL Legacy | yes | yes | | |
| NHL Legacy Recompiled | | | | yes |

Notes:

- **NHL 07 on PlayStation 3** is studied for research but is not a target you can mod; the picker offers NHL 07 on Xbox 360 only.
- **NHL Legacy Recompiled** is a recompilation of the Xbox 360 build. Its game data is byte-identical to the Xbox 360 release, so it shares that version's support, but it keeps its own save locations and needs its own in-game verification.
- **Later titles** appear in the game picker marked *In development*. Choosing NHL 22, 23 or 24 opens a separate read-only asset browser for those games' Frostbite files; nothing can be edited there yet. Titles beyond that cannot be opened.

## What each platform means for your files

| Platform | How the game is identified | Database file |
|---|---|---|
| Xbox 360 | The folder holding `default.xex`; the title id is read from that file. | `nhlng.db` for NHL 08 onwards, `NHL07NG.db` for NHL 07, packed inside `.big` archives on NHL 10 and later until extracted. |
| PlayStation 3 | A disc folder, a `USRDIR` folder, or an RPCS3 folder; the product code (`BLUS…` or `BLES…`) names the title. | `nhlng.db`, loose in the game's `USRDIR`. |
| PlayStation 4 | Any game folder, identified by its own `sce_sys/param.sfo` (title, `CUSA…` id, region, patch version). | `nhl_game.db`, packed in the game's archives until extracted. |
| Legacy Recompiled | The recompiled game folder, packed like the Xbox 360 build. | `nhlng.db`, extracted from the `.big` archives. |

See [Preparing Your Game Files](Preparing-Your-Game-Files) for the folder layouts Studio looks for and how extraction works.

## Capabilities differ by game

Even within the supported set, the games do not all carry the same data. Studio probes the loaded database and shows only the screens that game's data supports: NHL 07 has no arena table, so the Arenas tab is hidden; some tabs appear read-only with an "unverified" note where the meaning of the data has not been confirmed. [Core Concepts](Core-Concepts) explains these markers.

Last reviewed: 2026-10-20

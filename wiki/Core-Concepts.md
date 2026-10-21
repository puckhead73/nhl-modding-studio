# Core Concepts

A handful of ideas run through every Studio screen. Knowing them makes the rest of the wiki read easily.

## Project

A **project** is a folder that holds everything about one mod for one game on one console: a working copy of the database, the dictionary, any staged asset files, the record of your changes, and the project's name, version, author and description. Your game installation is never edited by the Modding Studio; only the project folder changes. Projects live under the folder you chose on first run, organised by platform, then game, then project name.

A project has a **mode** set by what it was created from: *database* (the normal case), *roster save* (a save file as the editing target) or *assets only* (a folder of extracted game assets with no database). An assets-only project shows an "Assets only" pill in the header and skips the database checks described below.

## Session, undo and saving

Every edit is applied the moment you make it and can be undone. The top-right of the Studio holds the session controls:

- **Undo** (Ctrl+Z) and **Redo** (Ctrl+Y or Ctrl+Shift+Z) step through your edits.
- **Save Session** writes the current state to the project folder. The **Last Save** label shows the time, or "Unsaved".
- Studio also autosaves changed project files at an interval you set under **Settings › General › Session autosave** (default every 10 minutes).
- **Close** asks "Close project session?" and offers **Save and Close**, **Close Without Saving** (when there are unsaved changes) or **Close Session**.

Closing the window does the same check, so you are never asked twice.

## Project Overview

The **Project Overview** button in the header (it shows the number of changes) opens the page that lists **all changes since this project was created**, grouped by what they touched, each with a revert link. This is also where you set the project's details and build packages; the [Quick Start](Quick-Start-Edit-a-Player-and-Build-a-Mod) walks through it.

## Build, package and activate

A **Build** turns the project's changes into a **`.nhlmod` package**, one per console the package can be installed on. A package carries changes, not whole files, so it is small and applies on top of the recipient's own game. The **Mod Manager** *activates* a package on a game installation, backing up what it replaces, and *deactivates* it by restoring those backups. **Package Dev Project Files** produces a `.nhlproject` archive instead: the complete project for sharing with another Studio user. See [NHLMOD Packages](NHLMOD-Packages).

## Automatic repairs when a project opens

Two checks run every time a database project opens, before you can edit:

1. **Roster layout repair.** Older tools sometimes left teams' roster blocks stored out of order. Studio moves the slots back into place without changing any player's team, number or lines, and shows "Repaired the roster layout" with the affected teams. The fix is staged as a pending, undoable edit: **Save** to keep it.
2. **Duplicate jersey numbers.** If two players on a team wear the same number, the first keeps it and the others get the lowest free number. Studio shows "Fixed N duplicate jersey numbers" with each change; these too are staged edits you can review in Project Overview and undo.

If the roster layout cannot be repaired, Studio shows **This database is corrupted** and blocks editing. The recommended way out is **Back to start** and a fresh, known-good database. **Override anyway (advanced)** lifts the block after a second warning ("Proceed on a corrupted database?"); this is unsupported and the resulting mod may crash the game or produce a save it refuses to load.

## Validation and the errors pill

After the repairs, Studio audits the project. If the audit finds error-level problems, a red **N errors detected** pill appears in the header; clicking it opens **Settings › Dashboard**, which lists the findings by severity and offers inline repairs such as **Fix duplicate jerseys** and **Repair roster blocks**. Warnings and notes never show the pill.

## Capability-gated screens

Not every game carries every kind of data. Studio inspects the loaded database rather than assuming anything from the game's name, and adjusts the screens:

- a tab is **hidden** when the game has no such data (NHL 07 has no Arenas tab);
- a tab is shown **disabled with a reason** when the data exists but editing it is not yet possible;
- a tab is shown **read-only with an "unverified" note** when the data exists but its meaning has not been confirmed for that game.

Asset editing has its own gate: until the game's archives are extracted, controls that need loose files show "Container extraction is required to edit this asset. Run Extract game containers in Settings → Tools."

## Recent trail

Under the main tabs, a **Recent** bar lists the views you have clicked through. Earlier entries are buttons that jump back; the ← button goes one step back.

## The three tools, side by side

| | Modding Studio | Mod Manager | Save Editor |
|---|---|---|---|
| Edits | The game's default database and assets, inside a project | Nothing; installs packages | One save file or database, in place |
| Needs a project | Yes | No | No |
| Writes to the game folder | Only during extraction | Yes, with backups | No; writes the save file you opened, after backing it up |
| Output | `.nhlmod` and `.nhlproject` | An installation record you can restore | The edited save |

## Developer Mode

The project's plans reserve the name **Developer Mode** for advanced features such as canonical cross-game projects and immutable builds. That mode is not part of the current beta; every feature on the wiki is available without it.

Last reviewed: 2026-10-20

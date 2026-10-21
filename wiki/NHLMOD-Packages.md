# NHLMOD Packages

A `.nhlmod` file is the unit of sharing: a small archive of *changes* that the Mod Manager applies on top of someone else's copy of the same game. It never contains the game's own files, which is what keeps it small and keeps it legal to share. This page explains what a package carries, which consoles it can target and where its limits are.

## What a package contains

| Content | How it is recorded | How it is installed |
|---|---|---|
| **Database cells** | Each changed field, named by table, entity and field, with the before and after values. The row is identified by its key values and a fingerprint of the original row, never by its position in the file. | The recipient's row is found by its keys; the field's current value must equal "before" or the install is refused as a conflict. A value that already equals "after" is skipped. |
| **Dictionary texts** | Each renamed text, by key, with before and after. | Applied to the recipient's own dictionary after checking the current text. A missing key blocks. |
| **Loose texture replacements** | The replaced file's relative path plus the source image you imported (kept inside the package). | The recipient's own file at the same path under the game's asset root is re-encoded from your image. A missing or unrecognised file blocks with the reason. |
| **Project details** | Name, version, author and description, shown in the Mod Manager. | |

Roster-only packages (from a project created around a roster save) install into a save location and need no game installation at all.

## One package per console

**BUILD** writes a separate `.nhlmod` for each console the project's changes can target, and lists any it could not build with the reason ("Not built: …"). The rules:

- A package only ever targets **the same game**. Cross-game installation is refused.
- **NHL Legacy and NHL Legacy Recompiled** count as one game on two runtimes, so a Legacy package can be built for both.
- **Database cells** can be built for another console of the same game only when the two consoles' default databases are identical byte for byte. For NHL 08 through NHL 15 and NHL Legacy the Xbox 360 and PlayStation 3 databases are identical; NHL 15 on PlayStation 4 is a different database.
- **Dictionary texts** can target any console that has a bundled dictionary; keys are verified one by one at install.
- **Loose textures** can target another console only where the games' loose-file layouts are registered as shared, which today means NHL Legacy on Xbox 360 and Recompiled. Every other pair is refused for textures.

A build never drops a change silently. If a project contains a kind of change the package format cannot carry, the build refuses and names it.

## What cannot be packaged yet

- Changes inside archive members (`.big` members on PlayStation 4 and Xbox 360, PlayStation 3 board pairs).
- Rows added or removed, and new dictionary keys.
- Game-settings defaults, schedule edits and dictionary growth. These are saved in the project and can be deployed from it, but not shared as a package.
- Native-only outputs such as cloned kits whose source image was not retained.

These are listed by the build as blockers rather than left out.

## Installation records and backups

Every activation writes an installation record beside the backups of the files it changed. Records make **Deactivate** and **Recover installation…** possible and let the Mod Manager tell an installed package from one that merely has the same name. A file whose content is neither the installed bytes nor the backed-up original is a foreign change, and both restore and recovery refuse to touch it.

## Sharing projects instead: `.nhlproject`

**Package Dev Project Files** in Project Overview produces a `.nhlproject` archive: the complete project, for another Studio user to open with **Import Project Package** and keep editing. A `.nhlproject` is for collaborators; a `.nhlmod` is for players.

## Reading a package before installing

In the Mod Manager, selecting a package shows **Changes in this mod** before anything is written, with the same tree the author saw in Project Overview. Use it to check what a downloaded mod will touch.

Last reviewed: 2026-10-20

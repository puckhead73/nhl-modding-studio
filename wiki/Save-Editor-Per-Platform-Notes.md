# Save Editor: Per-Platform Notes

What the [Save Editor](Save-Editor) can open and write differs by platform and by title. This page is the reference; the format details are in [Save Containers for Xbox 360 and PS3](Save-Containers-Xbox-360-and-PS3) and [Save Containers for PS4](Save-Containers-PS4).

Evidence level throughout: implemented and automated-tested on reference saves. No save written by Studio has yet been recorded as loading in a game.

## Xbox 360 and PlayStation 3

| Titles | Roster save | Mode saves (franchise, season, Be a Pro) | Notes |
|---|---|---|---|
| NHL 07, 08 | Change lists against the game database: opened applied to the bundled database, edited, saved as a regenerated change list. Only fields the save can record are editable; the rest read-only. | read-only | The NHL 07 writer reproduces the game's own saved files byte for byte. NHL 08 writes are automated-tested only. |
| NHL 09 | Full roster TDB in a v3 container; edit and save. | read-only | |
| NHL 10, 11 | Edit and save; **writes marked experimental** because one container tag is not fully reversed. | read-only | Whether the game accepts an edited NHL 10 or 11 save is open. |
| NHL 12, 13 | Edit and save. | read-only; NHL 13 mode saves are split into parts and refused | |
| NHL 14, 15, Legacy | Edit and save; the zlib body is reproduced exactly. | read-only; split PS3 saves refused | |

Xbox 360 saves must already be out of their console package; Studio edits the inner save file. PS3 saves must be the plain exported form; a console-sealed save is refused with a hint to export it first.

Settings, profile and Ultimate Team saves are refused on every title. Hockey-card saves contain no database and are refused.

## PlayStation 4

| Titles | Roster save | Mode saves | Notes |
|---|---|---|---|
| NHL 15 | Edit and save (chunkzip payload). | open read-only | Structural write-back only. |
| NHL 16 | Edit and save (bare LZ4). | open read-only | The repack is equivalent to, not identical with, the game's file. |
| NHL 17 to 21 | Edit and save (LZ4 with a size word). | NHL 17 onwards nest a second compressed layer that Studio does not read; they fail identification. | The roster save carries a trailer with the database size; while it is present, an edit that changes the database length (adding rows) is refused. 3-on-3 saves open after acknowledgement. |

A title whose bundled definitions do not match the file can still be opened through **Advanced: choose the game yourself** with that game's own metadata file.

## All platforms

- Tables a save does not contain are shown read-only from the game's default database and never written into the save.
- Mode saves hold their mode's tables, not rosters; the teams and players shown for them are the vanilla defaults, which is not necessarily what the mode uses. Mode progress is not adjusted for edits.
- Before the first overwrite of a session Studio copies the original into its `save-editor-backups` folder.

Last reviewed: 2026-10-20

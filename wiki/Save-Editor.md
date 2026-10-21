# Save Editor

The Save Editor opens one save file, or one extracted game database, and edits it in place. There is no project and no game installation: pick the file, let Studio work out which game and console it belongs to, change what you need, and save. It uses the same database screens as the Modding Studio, trimmed to what a save can hold.

Evidence level: the Save Editor is implemented and automated-tested against reference saves for each supported container. No save written by it has yet been recorded as loading in a game; treat it as beta and keep your own copy of any save you care about (Studio also keeps one, see below).

## What you can open

Press **Choose file…** and pick one of:

- **PlayStation 3:** the `SYS-DATA` file inside the save's folder (for example a folder named `BLUS31179…-ROSTER`).
- **Xbox 360:** the save file itself (for example `ROSTER`).
- **PlayStation 4:** the `DATA` file inside the save's folder.
- **An extracted game database** such as `nhlng.db` or `nhl_game.db`.

| Kind of file | What happens |
|---|---|
| Roster saves | Open and edit. Tables the save does not contain (arenas, league tables and so on) are shown read-only from Studio's copy of the game's default database. |
| Franchise, Dynasty, Season, Be a Pro and 3-on-3 saves | Open after you tick "I understand. Open this save anyway." These hold their mode's own tables, not the rosters, so teams and players appear read-only from the default database. Mode progress is not adjusted for your edits. |
| NHL 07 and NHL 08 roster saves | These store a list of changes against the game's database. Studio shows the resulting rosters, lets you edit the fields the save can record, and writes a regenerated change list. |
| Extracted databases | Open and edit; nothing is supplemented. |
| Settings, profile and Ultimate Team saves | Refused: settings are outside the Save Editor's scope. |
| Split PlayStation 3 saves (`DATA0000…`) | Refused: multi-part saves are not read. |

## Identification

After you pick a file Studio shows a card with the file name, its format, the kind of save, and the game and console it matched, with the game's tile. Identification uses the file itself: the container format and checksums, the byte order (which separates PlayStation 4 from the others), and whether every table and field in the file matches a game's bundled definitions. The folder names (`BLUS…`, `BLES…`, Xbox title ids, `CUSA…`) count as extra evidence.

- When the layout fits several consoles of the same game (an Xbox 360 save outside an emulator folder, say) Studio asks: "This file's layout is the same on several platforms. Choose the one it came from."
- When several games fit, it asks "This file matches more than one game. Choose the one it came from." and marks any with an "exact layout match".
- **Details** expands the path, container version, whether checksums are valid, how many tables the file holds and how many are shown read-only, and the evidence for the match.

A yellow note means Studio will open the file with a caution you must acknowledge; a red note means it refuses, with the reason.

**Advanced: choose the game yourself** appears when nothing matched. You pick the game and platform and supply the game's metadata file (`…-meta.xml`) from the same game and update. Studio still checks that it describes every table and field in the file before opening.

## Editing

**Open in Save Editor** opens the file with a reduced menu: **Save Data** (Leagues, Teams, Players, Arenas, Transactions, Pending Changes) and **Validation** (the Dashboard). The header reads *Save Editor · game · console · kind*, and "Data not stored in this save is read-only" when tables are supplemented. Historical, Legends, Hockey Shop, Assets, Game Settings and dictionary renaming are not available in a save session.

Fields from supplemented tables cannot be edited, and an operation that would change one is refused by name. Undo and redo work as in the Modding Studio.

## Saving

**Save** writes the file you opened; **Save As** writes to another file. Before the first overwrite of a session Studio copies the original into its `save-editor-backups` folder in the application data and tells you the backup's name. Checksums are recomputed; an unedited save round-trips byte for byte.

Platform notes shown by the editor:

- Writes to the NHL 10 and NHL 11 container are marked experimental.
- PlayStation 4 write-back is checked structurally. On some titles the save ends with a trailer that embeds the database size; while that trailer is present, an edit that would change the database length (adding rows) is refused.

Last reviewed: 2026-10-20

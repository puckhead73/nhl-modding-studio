# FB-W1 stage D: NHL 24 roster editing in Studio, 1 November 2026

**Summary.** NHL 24 now opens in Studio as an ordinary roster project. Picking NHL 24 on the
project picker leads to a new page that asks for the install folder and the Oodle DLL. Opening
reads the database the game currently loads and shows it in the usual team and player editors.
Saving writes Studio's files into `game-mods` through the stage C writer. A Restore button on
the same page removes them again. Against the real install, the views read NHL 24 correctly
(531 teams, 9,067 players, McDavid #97 C on Edmonton with a $12.5M cap hit for 3 years, as in
game). A rename saved, reopened and restored end to end. The app itself has not yet been run
by the user, and no save made from the app has been checked in game.

Package: [nhl24-roster-writer-package-2026-11-01.md](../plans/nhl24-roster-writer-package-2026-11-01.md).
Earlier stages: [A](nhl24-fbw1-stage-a-20261101.md), [B and C](nhl24-fbw1-stages-bc-20261101.md).

## 1. Behaviour

- **Open** (`nhl24_roster_open`):
  1. Checks the folder holds `game` and `game-UPDATE\Patch\Gen4b\globals.toc`, and loads the
     user's Oodle DLL.
  2. Reads the effective database (Studio's overlay included) and writes it, with its
     metadata XML, to `<install>\studio\working\`.
  3. Opens that file with the existing workspace (`open_workspace_from_paths`, game id `nhl24`,
     PS4).
  - A second open of the same install is refused.
- **Save:**
  - The app's normal save (`save_workspace`) routes NHL 24 sessions to `nhl24_save`.
  - It packages the edited database and checks that it parses and round-trips. It then
    rebuilds the overlay from stock (`stock`, `choose_cas`, `plan`, `apply`), refusing while
    the emulator runs. Only after that does it save the working copy and adopt it as the
    baseline.
  - The game is always the source of truth. If the working copy fails to save after the
    overlay was written, the error says so, and the next open re-extracts from the game.
- **Restore** (`nhl24_roster_restore`): removes Studio's files from `game-mods`. It is refused
  while the NHL 24 project is open or the emulator runs.
- **Status** (`nhl24_roster_status`): lists Studio's files, counts other mods' files, and
  reports a running emulator and the open project.
- **UI:**
  - New page `/nhl24` (`app/src/pages/Nhl24Roster.tsx`). The project picker sends NHL 24
    there; NHL 22 and 23 still go to the asset browser.
  - After opening, the page navigates to the database editors. No install is attached to the
    project, so art, audio, localization and settings surfaces stay hidden, and only database
    editing is offered.
- **Encoded cells:** no player-row encoding contract claims the game id `nhl24`
  (`encoding::SchemaFamily::of_game`). Weight, ratings, birth dates and contract dollars
  therefore display on the schema's PS4 encoding but are not editable, and the page says so.

## 2. Checks

| Check | Result |
|---|---|
| `nhl24_roster` (studio-api, ignored; `NHL24_INSTALL`, `NHL24_OODLE`): a temporary install made of junctions to the real `game` and `game-UPDATE` with an empty `game-mods` | pass (about 20 s) |
| ↳ open from stock; second open refused | pass |
| ↳ `list_teams` / `list_players`: 531 teams, 9,067 players; bio row 1570 is Connor McDavid, #97, C, Edmonton Oilers (EDM) | pass |
| ↳ cap hit 12,500,000 and 3 contract years (match the in-game screenshot; display only) | observed |
| ↳ derived overall 99 against 97 in game (the formula is an NHL 14/15 fit; display only) | observed |
| ↳ `set_field` rename, then `save_workspace`: 2 overlay files; restore refused while open | pass |
| ↳ close, reopen from the overlay; the rename is in the working copy; close, restore, `game-mods` empty | pass |
| `tdb-frostbite` library tests (19) | pass |
| Vitest: `Nhl24Roster.test.tsx` (open → project context → editors; status; restore), `OpenProject.test.tsx`, `FrostbiteBrowser.test.tsx` (39 tests) | pass |
| `tsc --noEmit` on the app: no errors in the touched files | pass |
| `cargo clippy` on studio-api and tdb-frostbite: no warnings in the new files | pass |
| Real `F:\Games\NHL24`: `game-mods` still has its 6 files, and no `studio` folder exists | verified |

## 3. Stage E: the user's in-app save (game-verified, 1 Nov 2026)

- **Run:** Studio was launched with `npm run dev`. The user opened NHL 24 from the picker,
  renamed `ubPc` row 1570 to "McJesus" in the editors, and saved. The user's in-game
  screenshot then showed "C. McJesus" on the Edmonton Oilers roster and in the player header,
  with every other value unchanged.
- **Files Studio wrote:** two files, recorded in `F:\Games\NHL24\studio\nhl24-overlay.json`:
  - `game-mods\Patch\Gen4b\globals.toc` (65,804 bytes; SHA-256 `e4525c22…e81adc`);
  - `game-mods\Patch\Gen4b\superbundlelayout\nhl_installpackage_00\cas_02.cas` (15,197,946
    bytes; SHA-256 `88798ed7…11bff9`).
- **Other mods:** the deke mod's five files and `eboot.bin` were unchanged.
- **State left in place:** the edit was left active at the user's choice. Restore from the
  NHL 24 page removes it.

This is the first save made from Studio itself that is game-verified for NHL 24. It covers a
same-size text edit through the app. Growth is covered separately by stage A probe 2.

## 4. Open points
- **Encoding contract for NHL 24.** Cap hit and contract years already match the game. Weight,
  height, birth date and the rating scale need the same comparison before `of_game` may claim
  `nhl24`. McDavid's screenshot gives 6'1", 193 lb and age 26 to compare against.
- **Adding players.** Inserting new players with their companion rows goes through the
  existing domain operations; it is untested on NHL 24 and needs its own in-game check.
- **Localization.** No NHL 24 dictionary is attached, so labels that need one show their keys.

## 5. Cleanup

- The tests wrote only to temporary folders. The junctions are removed before the folder is
  deleted.
- No game file was written, and no process was left running.

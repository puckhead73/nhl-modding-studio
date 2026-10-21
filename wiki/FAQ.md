# FAQ

**Is this released?** No. The current build is a beta shared with testers. No title is yet verified in the game itself; see [Supported Games and Platforms](Supported-Games-and-Platforms) for what that means.

**Does Studio include any game files?** It edits copies of the games you own. What ships with Studio is described in [Requirements and Installation](Requirements-and-Installation).

**Which should I use: Modding Studio or Save Editor?** The Modding Studio changes the game's *default* data, what a fresh "Restore default rosters" loads, and produces a package anyone can install. The Save Editor changes one save file in place. See [Core Concepts](Core-Concepts).

**Can I share a mod?** Yes, as a `.nhlmod` built from Project Overview. The recipient installs it with the Mod Manager on their own copy of the same game. Some kinds of change cannot be packaged yet; the build tells you. See [NHLMOD Packages](NHLMOD-Packages).

**Can I install two mods at once?** No. Activating one deactivates the other; the Mod Manager restores the original files in between.

**Why is a tab missing or read-only?** The loaded game's data does not carry it, or its meaning is not confirmed for that game. See the capability notes in [Core Concepts](Core-Concepts).

**Why can't I edit a logo or jersey?** The game's archives have not been extracted. Run **Settings › General › Tools › Extract game containers**. See [Preparing Your Game Files](Preparing-Your-Game-Files).

**Studio "repaired the roster layout" when I opened my file. Did it change my rosters?** No players changed teams, numbers or lines. Older tools stored teams' roster blocks out of order; Studio put the slots back where the game expects them. It is staged as an undoable edit. See [Dashboard and Validation](Dashboard-and-Validation).

**Why does my player's overall look wrong on NHL 10 to 13?** The calculated overall uses EA's formula fitted on NHL 14 and 15 and reads about 13 points low on earlier titles. See [Known Limitations](Known-Limitations).

**Can I import a whole historical team?** Yes: open the team in [Historical Teams](Historical-Teams), choose the season and the EA team it replaces, and send it to the [Roster Builder](Roster-Builder).

**Can I put a Legends team into the game?** Not yet. Puck's Picks is a browser in this beta; send individual seasons through the Roster Builder instead.

**Can Studio replace audio?** No. It lists and previews speech, sound effects, horns and songs where it can decode them, but there is no encoder.

**Can Studio edit NHL 22, 23 or 24?** No. They open in a read-only Frostbite browser.

**Where are my settings stored?** In Studio's application data on this PC, not in projects. See [Settings](Settings).

**Something looks wrong on a page.** Check [Troubleshooting](Troubleshooting), then file a report following [Reporting Bugs](Reporting-Bugs).

Last reviewed: 2026-10-20

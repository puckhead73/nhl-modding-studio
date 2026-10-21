# Settings

**Settings** holds "Studio preferences — stored on this machine, independent of any open project." It has three subtabs: **General**, **Paths** and **Dashboard** ([Dashboard and Validation](Dashboard-and-Validation)).

## General

**Default startup.** "Which title onboarding jumps straight into when you launch the Studio." Choose a **Default game** (or "No default (show game picker)") and a **Default console**. With a default set, Studio opens the wizard at that game and console with its saved paths pre-filled; it never opens a project by itself.

**Session autosave.** "Automatically saves changed project files. Default: every 10 minutes." Enter a whole number of minutes from 1 to 1440 and press **Apply**.

**Tools.** "Extract game assets from your source folder for editing." With a project open and a game source linked, the **Extract game containers** panel appears, the same one offered during project creation: choose **Extract assets to**, read the preflight ("N container(s) need(s) extracting."), press **Extract now** or **Extract remaining**, and watch the per-container and overall progress. A finished extraction re-links the project to the extracted folder. Without a project or a source: "Choose a game source folder in Project Overview to extract its assets."

**Player manifest (CSV).** "Every player installed in the loaded database — slug, team, league, and which of them are still missing art." One row per player with id, bio record, name, export slug, position, team, league, and whether a portrait and a head mesh resolve on disk. EA's shared placeholder portrait counts as no portrait and is flagged in its own column, "so the export is usable as a to-do list". Needs an install tree.

## Paths

**Project storage.** The folder new projects are created under. "Changing this location affects new projects; existing projects stay where they are."

**Game source folders.** One tab per console Studio knows. For each title, a **Game source folder** ("Game / emulator folder", or on PS4 "The game's own folder (or a folder holding your PS4 games)") that the wizard pre-fills. "Studio reads source files here; Mod Manager chooses where builds are installed."

## Where these are stored

In the application's local data on this PC. The Mod Manager's library folder and installation targets are kept separately in the same way, and the Hockey Shop's store lives in the application data folder. None of them live inside a project, so projects move cleanly between machines.

Last reviewed: 2026-10-20

# Quick Start: Edit a Player and Build a Mod

This walkthrough takes about fifteen minutes the first time and covers the whole loop: create a project, change one player's jersey number and a rating, build a package, and activate it on your game. It assumes Studio is installed ([Requirements and Installation](Requirements-and-Installation)) and you have a dumped copy of a supported game ([Preparing Your Game Files](Preparing-Your-Game-Files)).

## 1. Create the project

1. On the start screen choose **Modding Studio**, then **Create New Project**.
2. The first time, **Choose your default NHL Project folder** and press **Continue**.
3. **Select Game**: click your title. If it shipped on more than one console, **Select Console** follows.
4. On **Project Setup**, type a project name. Press **Select Game Source** and pick either your dumped game folder or an ISO with an extraction destination. The **Next** button enables once a source is chosen.
5. Press **Next**. Studio creates the project. If the game's archives need extracting it shows **Extract game assets now?**; you can press **Continue to Studio** now and extract later from **Settings › General › Tools**, because this walkthrough only edits the database.

Studio lands on **Game Database › Teams**.

## 2. Find the player

1. Click the **Players** subtab. The list shows every player with position and team; free agents are highlighted. Use the position filter or sort by a column to narrow it.
2. Click a player to open their detail page. It has tabs: **Player Data**, **Career Stats**, **Attributes** and **Equipment** (some may be hidden or read-only depending on the game; see [Core Concepts](Core-Concepts)).

## 3. Change a jersey number

On **Player Data**, each value shows "Click to edit" when you hover. Click the jersey number, type the new value and press **Enter** to commit, or **Escape** to cancel. The edit is applied immediately; the **Project Overview** button in the header now shows one change, and **Undo** is enabled.

If the number is already worn by a teammate, the validation audit flags it as a duplicate; **Settings › Dashboard** offers **Fix duplicate jerseys**, or simply choose another number.

## 4. Change a rating

Open the **Attributes** tab. Ratings are shown in the game's own scale and edited the same way: click, type, Enter. The tab also shows a calculated overall where the game's rating layout is one Studio knows how to calculate for; otherwise it says so rather than guessing.

## 5. Save the session

Press **Save Session** in the header. "Project has been saved" appears and **Last Save** shows the time. Autosave would have done this for you eventually; saving by hand simply makes the next step immediate.

## 6. Review and build

1. Press **Project Overview** in the header.
2. Under **Project Details** fill in at least **Name** and **Version** (both are required to build); **Author** and **Description** are shown to whoever installs the mod. Press **Save details**.
3. The **Project changes** card lists your two edits with the before and after values. Each has a revert link if you change your mind.
4. Press **BUILD**. Studio saves the project and writes one `.nhlmod` file per console the package can be installed on into the `Builds` folder under your project storage. "Build complete." lists the files; **Open containing folder** shows them in Explorer.

Instead of **BUILD** you can press **Send to Mod Manager**, which builds and then opens the Mod Manager with the new package already selected and the matching installation target preselected.

## 7. Activate the mod

1. Open the **Mod Manager** (from the start screen, or via **Send to Mod Manager**).
2. The first time, choose your **mod library** folder and then **Choose your game installation**: the game, the console and the folder containing the game's executable. See [Mod Manager](Mod-Manager).
3. Select your package in the **Mod library** list. The right-hand side shows its details and **Changes in this mod**, the same two edits.
4. Press **Activate**. The Mod Manager checks the installation, deactivates any other active mod, prepares the package against your game's own files and installs it. "Mod activated." appears and the package shows an **Installed** pill.

Launch the game. Your player has the new number and rating.

## 8. Undo it all

Back in the Mod Manager, select the package and press **Deactivate**. "Mod deactivated. Original files restored." confirms the game folder is back to how it was. The project is untouched, so you can keep editing and rebuild any time.

## What to read next

- [Core Concepts](Core-Concepts) for projects, sessions, automatic repairs and capability gating.
- [NHLMOD Packages](NHLMOD-Packages) for what a package can and cannot carry.
- [Known Limitations](Known-Limitations) before you share a mod.

Last reviewed: 2026-10-20

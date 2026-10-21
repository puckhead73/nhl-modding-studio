# Projects: Create, Load and Import

The Modding Studio always works inside a project. This page covers the project wizard that opens when you choose **Modding Studio** on the start screen: its four entry points, each screen's fields, and what Studio does behind them. For the concepts behind projects see [Core Concepts](Core-Concepts); for the folder layouts Studio looks for see [Preparing Your Game Files](Preparing-Your-Game-Files).

## The four entry points

| Button | Use it when |
|---|---|
| **Create New Project** | You want to start from Studio's bundled reference database for a game and add your own game files as the source of assets. |
| **Load Project** | You have a project folder (it holds `nhl-studio-project.json`) or want one from the recent list. |
| **Import Project Package** | Someone sent you a `.nhlproject` archive. |
| **Import non-studio project** | You have your own database, dictionary or extracted assets and want a project built around them. |

Every entry point first needs a project storage folder. If none is set, **Choose your default NHL Project folder** appears with the note "Projects are organized by platform, game, then project name. Released builds go in Builds." Change it later under **Settings › Paths › Project storage**.

If a project is still open from before, every button is disabled with the reason "The previous project is still open — close it (or return to it) first."

## Create New Project

1. **Select Game**: every title Studio knows, in release order. Titles marked *In development* ("Support for this title is still being added; opening its files may not work yet.") are not supported; choosing NHL 22, 23 or 24 opens the read-only [Frostbite Browser](Frostbite-Browser-NHL-22-24).
2. **Select Console**: appears only for titles that shipped on several consoles. **Change game** goes back.
3. **Project Setup** shows the game and console as a pill with **← Change game**, the project name and folder fields, and **Select Game Source**.
   - The game source is an ISO (with an **Extraction destination** that should be "a new or empty folder on an available drive") or a dumped folder. The card then reads "Game source", with the ISO path, the destination or "Dumped ISO folder", and **Change game source**.
   - If Studio has no bundled reference database for this game and console, a red "Bundled baseline: not available" note lists what is missing and **Next** stays disabled. Use **Import non-studio project** instead. A partial baseline shows an expandable "Baseline availability notes".
4. **Next** creates the project. For an ISO, "Disc extraction: … " shows progress with **Cancel extraction**; a cancelled extraction can be retried at the same destination. Studio then checks the game folder and, if archives still need unpacking, shows **Extract game assets now?** (below). Otherwise it opens the Studio on **Game Database › Teams**.

**Next** stays disabled with a one-line reason until the project location is valid, a game source is chosen and nothing is still running.

## Import non-studio project

After **Select Game** and **Select Console**, Project Setup shows:

- **Game source folder (optional)** with **Browse…** and **Detect**. "Studio reads game files from this folder. Mod Manager chooses where your built mod is installed." Detection runs automatically shortly after you type or browse, and shows **Detected in the game source** with ✓ or — rows for the main database, metadata and localization, plus on PS4 the list of game folders it considered.
- **Your files (read sources — copied into the project, never written). Any one of them is enough.**
  - **Main database** (`nhlng.db`, `NHL07NG.db` on NHL 07, `nhl_game.db` on PS4): "its -meta.xml beside it, or the bundled schema is used".
  - **Localization dictionary (optional)**.
  - **Game assets folder (packed or extracted, optional)**: "A dumped game folder with .big archives, or extracted rendering/, fe/, …". Choosing one scans it and fills in the database and dictionary it finds.
- Platform helpers: on PS3 a disc panel that reads a disc or ISO; on PS4 a package panel that extracts a PKG; on Xbox 360 an ISO panel that stages an ISO and opens it directly (this path skips the project folder and lands on Teams).
- An **Import review** list with ✓ or ✗ per role and a detail. **Next** enables when the review passes; until then the reason reads "Choose at least one of your own files: a game database or a folder of extracted assets." or "Waiting for the review of your files."

## Extract game assets now?

"Your project is ready. Prepare game assets now, or continue and do this later." The step shows:

- **Game file verification**: "Checking game files…", then either a pass or **Continue with modified files** / **Continue with unverified files**.
- **Extract game containers**: "Prepare textures, models and audio for editing. You can do this later in Settings → Tools." with the **Extract assets to** folder, a preflight ("N container(s) need(s) extracting." / "No extraction needed."), and **Extract now** or **Extract remaining**. Blocked states read "Not enough free space. Free up space and try again." or "Some files cannot be extracted safely. You can skip this step for now." Progress shows "Current container" and "Overall extraction"; a run can be cancelled and resumed later.
- **Continue to Studio** once verification has answered and no extraction is running. Skipping is allowed; the extraction panel is the same one found under **Settings › General › Tools**.

NHL 07 and NHL 08 on Xbox 360 always show this step when a game folder is linked.

## Load Project

- **Browse for project…** opens a folder picker titled "Pick the project folder (it holds nhl-studio-project.json)".
- **Open existing project — folder**: a path box and **Open**.
- **Recent projects**: each with **Forget** ("Remove from the recent list (the folder is not touched)") and **Delete** ("Delete the project folder and its files"), the latter after the confirmation "Permanently delete this project and all files in its folder? This cannot be undone."
- A `.nhlproject` can also be imported from this screen.

Opening shows **Resume project**: the folder, the mode (database, roster save, assets only), the number of saves, the last save time, staged asset files, the linked game source ("— not linked" if none) and any notes, then **Continue where I left off** or **Continue to Studio**.

## Import Project Package

Picks a `.nhlproject`, extracts it under your project storage and opens it. If the import succeeds but the open fails, the message says where the imported project was saved and offers **Retry opening project**.

## What a project folder holds

The working copy of the database and dictionary, staged asset files, the change record, the project details and settings, autosaves, and the record of any deployments. Everything you build from it goes to the `Builds` folder under project storage. The game installation you linked is read, never written, except by the extraction step.

Last reviewed: 2026-10-20

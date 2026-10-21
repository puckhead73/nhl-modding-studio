# Troubleshooting

Messages are quoted as Studio shows them. If your problem is not here, see [Reporting Bugs](Reporting-Bugs).

## Studio does not start

**"NHL Modding Studio could not start its window."** The message names the underlying error first, then explains that Studio needs the Microsoft Edge WebView2 Runtime, which is part of Windows 11 and installed on most Windows 10 PCs. Release archives bundle their own copy, so this normally means the archive was not extracted completely.

**"The bundled WebView2 runtime is incomplete. Extract the entire archive again."** Part of the bundled runtime is missing. Re-extract the whole archive into an empty folder.

**"Could not prepare the bundled runtime's read permissions. Extract Studio to a local folder you own."** Studio could not grant the runtime the permissions Windows 10 requires. Move the Studio folder out of a protected or network location.

## Opening a game

**"The game source was not recognized. Select the dumped game's root folder."** Point Studio at the folder layout described in [Preparing Your Game Files](Preparing-Your-Game-Files): the folder holding `default.xex`, a PlayStation 3 `USRDIR` or disc folder, or a PlayStation 4 game folder with `sce_sys/param.sfo`.

**"Bundled baseline: not available"** on Project Setup means Studio has no reference database for that game and console, so **Create New Project** is blocked for it. Use **Import non-studio project** with your own database instead.

**Several PlayStation 4 game folders match.** Studio lists every folder it considered with its title, id, region and patch version. Click **Use this folder** beside the one you mean.

**"Couldn't scan that folder"** or a red problem line in the detection summary names what was missing: the database, its metadata or the save location. The text says which.

**"The previous project is still open — close it (or return to it) first."** Studio will not open a second project over one that is still open. Go back and close it from its session controls.

## After the project opens

**"Repaired the roster layout"** or **"Fixed N duplicate jersey numbers"** are not errors. They are automatic repairs staged as pending edits; press **Got it**, then **Save Session** to keep them, or undo. See [Core Concepts](Core-Concepts).

**"This database is corrupted"** means the automatic repair could not fix the roster layout and editing is blocked. Start again from a fresh, known-good database. **Override anyway (advanced)** exists but is unsupported; the result may crash the game.

**A red "N errors detected" pill** in the header means the validation audit found errors. Click it to open **Settings › Dashboard**, where each finding has details and, where possible, a repair button.

**"Container extraction is required to edit this asset. Run Extract game containers in Settings → Tools."** The asset you are trying to edit lives inside a packed archive. Run the extraction from **Settings › General › Tools** and return.

**"Not enough free space. Free up space and try again."** Extraction needs room beside the game for the unpacked files.

**"Extraction finished. Some files were skipped."** or **"Some files cannot be extracted safely."** Studio unpacked what it could. Edits that need the skipped files stay gated; everything else works.

**A field shows "This field is not part of the selected game's database."** The game you loaded does not carry that value; nothing is wrong.

**A tab is missing or marked unverified.** The loaded game's data does not support it, or its meaning is unconfirmed. See the capability notes in [Core Concepts](Core-Concepts).

## Building and installing

**BUILD is disabled.** Both **Name** and **Version** must be filled in under Project Details, and a project storage folder must be set.

**"Not built: …"** after a build names a console the package could not target and why; see the rules in [NHLMOD Packages](NHLMOD-Packages).

**"No mods for this game and platform. Add .nhlmod files to your mods folder, then refresh."** The library filter is set to the selected installation. Switch **Filter platforms** to *All platforms* to see everything in the folder.

**"Select a matching installation target to activate this mod."** The package was built for a different game or console than the selected installation.

**Activation refused with a conflict.** The installed game's file no longer matches what the package author started from, often because another mod or an earlier edit changed it. Deactivate other mods first, or restore the game's original files.

**"An earlier operation needs recovery. Deactivate this mod first."** or **"An earlier operation was interrupted."** Press **Deactivate**, or use **Recover installation…** in the header, which finishes the interrupted operation from what the files currently hold.

**"This package changed. Refresh your mod library to review it."** The `.nhlmod` file was replaced while the Mod Manager had it open. Press **Refresh**.

## Save Editor

**"Studio could not match this file to one of its bundled games."** Use **Advanced: choose the game yourself** with the game's own metadata file, or check that the file is a save and not a settings or profile file, which the Save Editor refuses.

**A refusal in red** names the reason: settings saves, split PlayStation 3 saves and files that are not databases are not opened.

**An edit is refused naming a table.** The table is one the save does not contain and is shown read-only from the default database. Those values cannot change through a save.

## Settings

**"Your settings could not be remembered on this device. They will work for this session."** Studio could not write its local preferences. They apply until you close the app.

Last reviewed: 2026-10-20

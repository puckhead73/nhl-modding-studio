# Mod Manager

The Mod Manager installs `.nhlmod` packages on a game and takes them off again. It is the only part of Studio that writes into a game installation on purpose, and it keeps a backup of every file it changes so a mod can always be deactivated. Open it from the start screen, or from the Modding Studio with **Send to Mod Manager**.

## Setting up

The first time, a dialog asks for two folders:

1. **Your mod library** — "Choose the folder where you keep downloaded .nhlmod files. Mod Manager will include mods in its subfolders too."
2. **Choose your game installation** — pick the **Game** and **Platform**, then the **Game installation folder**: "Choose the folder containing the game's .xex file" (Xbox 360), ".bin or .elf file" (PlayStation) or ".exe or .elf file" (Recompiled). "This is where Mod Manager will install your mods." Studio checks that the folder really holds such a game before accepting it.

Later, the header offers **Mods folder** to change the library, **Add game** to register another installation and **Change installation folder** for the selected one. The **Installation target** dropdown switches between registered installations, and the current installation folder is printed under it.

## The library

The left pane lists the packages found in your mods folder, filtered by **Filter platforms**: *Selected game and platform* (default), *All platforms* or one console. Packages that came from **Send to Mod Manager** are included even when they live outside the library folder. Each entry shows the mod's name and version and an **Installed** pill when it is active on the selected installation. **Refresh** re-reads the folder and the installation status.

If the list is empty: "No mods for this game and platform. Add .nhlmod files to your mods folder, then refresh."

## Mod details

Selecting a package shows its name, version and author, the consoles it can be installed on, the author's description, and **Changes in this mod**: a tree of every change the package carries, named by table, entity and field with before and after values, or by file for textures and dictionary texts. Nothing is written while you look. The package's creation date and path are at the bottom.

A package can only be activated on an installation of the same game and console. Otherwise the page says "Select a matching installation target to activate this mod."

## Activate

Pressing **Activate** runs these steps and reports each as it goes ("Checking installation…", "Deactivating …", "Preparing mod…", "Installing mod…", "Refreshing installation status…"):

1. Re-reads the installation records, so a package is never shown as installed on the strength of an earlier failed write.
2. If another mod is active on this installation, restores it first: only one mod is active at a time, and older stacked installations from earlier Studio versions are unwound newest first.
3. Prepares the package against your game's own files. Every change is checked against what the file currently holds: a row is found by its key values, a field's current value must match the package's "before" value, a texture's current content must match what the author replaced. A mismatch is a conflict and the activation is refused with the reason; nothing is written.
4. Applies the package. Each file that changes is backed up and an installation record is kept. "Mod activated." appears and the **Installed** pill shows.

If an earlier operation on this mod was interrupted, Activate refuses with "An earlier operation needs recovery. Deactivate this mod first."

## Deactivate

**Deactivate** restores every file the package changed from its backup and removes the installation record: "Mod deactivated. Original files restored." Any file that no longer holds either the installed bytes or the original is treated as a foreign change, and the whole operation refuses rather than guess.

## Recover installation…

This button appears when the selected installation has records that can be restored. Use it after a crash or an interrupted operation. It classifies each record ("committed", "rolled-back", "split", "restore-incomplete", "publishing", "restored") from what the game files actually contain now and finishes whatever was left half-done, putting back any file that still holds installed bytes.

## Return to Studio

When a project is open the header shows **Return to Studio**, which goes back to its Project Overview; otherwise **Main menu** returns to the start screen.

## Standalone installer

A more detailed three-step installer (package, destination, planned changes) exists for installing a package somewhere other than a registered installation, including a roster-only package into a save folder with no game installation. It is not linked from the Mod Manager's screens in the current beta.

Last reviewed: 2026-10-20

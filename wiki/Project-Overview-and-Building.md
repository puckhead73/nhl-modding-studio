# Project Overview and Building

The **Project Overview** button in the header opens the page that describes the project, lists every change you have made, and turns the project into packages. The header button shows the current change count.

## Project Details

**Name**, **Version**, **Author** and **Description**. Name and version are required to build; author and description are shown in the Mod Manager to whoever installs the mod. **Save details** stores them; the status beside it reads "Unsaved details" or "Details saved".

## The three outputs

| Button | Output |
|---|---|
| **BUILD** | "Saves your edits and creates a .nhlmod for each compatible platform." The packages go to the `Builds` folder under your project storage; "Build complete." lists each path with **Open containing folder**, and "Not built: …" names any console that could not be targeted and why. |
| **Package Dev Project Files** | "Saves and packages your complete project for sharing." Produces a `.nhlproject` archive. See [Portable Projects (NHLPROJECT)](Portable-Projects-NHLPROJECT). |
| **Send to Mod Manager** | Builds, then opens the [Mod Manager](Mod-Manager) with the new package selected and the matching installation preselected. |

All three first save the project. If the project cannot be saved the action stops with "The project could not be saved. Resolve the save issue before packaging." Without a project storage folder the storage field appears here.

What a package can and cannot carry is explained in [NHLMOD Packages](NHLMOD-Packages).

## Project changes

"All changes since this project was created." A tree grouped by what was touched, each change with before and after values and a revert link; a pill says **Saved** or **Unsaved changes**. Notices appear above the tree, for example when a category of change cannot be packaged. The automatic repairs that run when a project opens (roster blocks, duplicate jerseys) appear here too.

Reverting a change is itself an undoable edit.

## Project folder and inputs

Below the details: the project's folder (with a field to relocate it), and the **Game source folders** card: "Studio uses these folders to find source files. Mod Manager chooses where your built mod is installed." It holds the **Game source folder** and the **Extracted assets source folder**, each with **Browse…**, and **Update sources**, which re-links the project to the new folders and refreshes everything.

## Recovery

A **Project Recovery** card appears at the top when an earlier save or deployment was interrupted, with the steps to finish or roll it back.

## PS4 copied-game archive output

On PlayStation 4 projects a collapsible panel, **PS4 copied-game archive output**, prepares database and dictionary changes inside a *copy* of the game's archive, with review and restore. It exists because the PS4 game reads its database and many assets from inside `inxp_00.big` and the other archives rather than from loose files. Changes that target archive members, such as **Replace in archive…** on logos, a Game Settings CSV cell or a PS3 board raster, are held as pending until "the next copied-target save" stages the archive. Archive-member changes cannot yet be shared in a `.nhlmod`; they are deployed from the project.

Last reviewed: 2026-10-20

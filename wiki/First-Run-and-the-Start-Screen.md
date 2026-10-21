# First Run and the Start Screen

Studio opens on a start screen with three large tiles. Each is a separate tool with its own purpose, and you can return to this screen from any of them.

| Tile | What it is for |
|---|---|
| **Mod Manager** | Activate and deactivate `.nhlmod` packages on a game installation, and restore the original files. Use this to install mods, yours or anyone else's. |
| **Modding Studio** | Create and edit a mod: the game database (leagues, teams, players, arenas), historical imports, equipment, uniforms and other assets. Produces `.nhlmod` packages. |
| **Save Editor** | Open one save file (or an extracted database) and edit it directly, with no project and no game installation. |

## Modding Studio: the first screens

Choosing **Modding Studio** opens a screen titled *Mod Studio* with four choices:

- **Create New Project** — start from Studio's bundled reference copy of a game's database, plus a game source of your own for assets.
- **Load Project** — reopen a project folder, from the recent list or by browsing to the folder that holds `nhl-studio-project.json`.
- **Import Project Package** — open a `.nhlproject` archive someone shared with you.
- **Import non-studio project** — start from your own files: a database, a dictionary or a folder of game assets.

### Choosing where projects live

The very first time you create or import a project, Studio asks you to **Choose your default NHL Project folder**. Everything you make is stored under it: "Projects are organized by platform, game, then project name. Released builds go in Builds." You can change this later under **Settings › Paths › Project storage**.

### Picking the game and console

**Select Game** shows every title Studio knows. Titles marked *In development* are listed but not supported yet. If the game shipped on more than one console, **Select Console** follows; otherwise Studio goes straight to **Project Setup**.

### Project Setup

Here you name the project and, for a new project, choose its **game source** (see [Preparing Your Game Files](Preparing-Your-Game-Files)). The **Next** button stays disabled until everything required is in place, and a short line underneath says what is still missing. Studio then creates the project, checks whether the game's archives need extracting, and either offers **Extract game assets now?** or takes you straight into the Studio on **Game Database › Teams**.

### Returning to a project

**Load Project** lists recent project folders. Each has **Forget** (removes it from the list; the folder is not touched) and **Delete** (permanently deletes the project folder after a confirmation). Opening a project shows a **Resume project** summary: its mode (database, roster save or assets only), how many saves it holds, when it was last saved, any staged asset files, the linked game source and any notes. **Continue where I left off** returns you to the last screen you were on.

## Mod Manager: first run

The first time you open the Mod Manager it asks two things in a short dialog:

1. **Your mod library** — the folder where you keep downloaded `.nhlmod` files. Subfolders are included.
2. **Choose your game installation** — the game and console, and the folder containing the game's executable (`.xex` on Xbox 360, `.bin` or `.elf` on PlayStation, `.exe` or `.elf` for the Recompilation). This is where mods are installed.

Both can be changed later from the Mod Manager's header. See [Mod Manager](Mod-Manager).

## Save Editor: first run

The Save Editor asks for nothing up front. **Choose file…** picks a save or database and Studio identifies it. See [Save Editor](Save-Editor).

## Where Studio keeps its settings

Preferences such as the project folder, the default startup game, saved game-source folders and the recent-projects list are stored in the application's local data on this PC, not inside any project. The Mod Manager keeps its own library folder and installation targets the same way. If Studio cannot write these, it says "Your settings could not be remembered on this device. They will work for this session."

Last reviewed: 2026-10-20

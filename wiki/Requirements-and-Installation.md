# Requirements and Installation

NHL Modding Studio is a single folder you extract and run. There is no installer, no account and no network connection. The current build is a beta shared with testers, so expect rough edges and read [Known Limitations](Known-Limitations) before relying on it.

## Requirements

| Requirement | Detail |
|---|---|
| Operating system | 64-bit Windows 10 or Windows 11. Other versions, ARM64 Windows and other operating systems are not supported. |
| Browser engine | The Microsoft Edge WebView2 Runtime. Release archives bundle a private copy, so nothing needs installing on most PCs. |
| Disk space | The archive is several hundred megabytes because it bundles the historical hockey database. Extracted game files for a title you mod can take several more gigabytes. |
| Your own game files | Studio edits copies of EA NHL games you own, dumped from your discs or console. It ships no game content and cannot obtain any. |

## Installing

1. Download the release archive (a `.zip`).
2. Extract the **entire** archive into a new folder on a local drive you own. Do not run it from inside the zip, from a network share or from a folder that needs administrator rights.
3. Run `nhl-studio.exe` from the extracted folder. Keep its bundled folders together: Studio reads its game definitions, the historical database and its helper programs from beside the executable.

Windows may show an "unknown publisher" warning the first time you run it. The release is not code-signed, so this warning is expected. Choose to run anyway.

## First launch

Studio opens on the start screen with three tools: **Mod Manager**, **Modding Studio** and **Save Editor**. See [First Run and the Start Screen](First-Run-and-the-Start-Screen) for what each does and what Studio asks for the first time.

## Updating

Extract a new release into its own folder rather than over an old one. Projects, Mod Manager settings and backups live outside the Studio folder, so they survive an update. Save any open project before switching builds.

## Uninstalling

Delete the Studio folder. Your projects stay where you created them. Mod Manager keeps backups of game files it changed; deactivate any active mod before removing Studio if you want the game restored.

## Third-party components

The release includes an unmodified build of FFmpeg, used only as a separate helper program to decode Xbox 360 XMA audio for previews, plus libspeex and open-source Rust and JavaScript libraries. Their licences and the complete corresponding source of the FFmpeg helper are inside the archive, with the full list in `THIRD-PARTY-NOTICES.txt`. Studio's own source code is released under the MIT License.

Last reviewed: 2026-10-20

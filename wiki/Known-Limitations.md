# Known Limitations

This page lists what the current beta does not do, or does with caveats, so that nobody is surprised after sharing a mod. It is kept current with each build; the date at the bottom says when it was last checked.

## The big one

**No title is game-verified.** Every supported game is implemented and covered by automated tests against reference data, but the project has not yet recorded a manual run proving that Studio's output loads and behaves in each game on each console. Until it does, treat every mod as a test. See [Supported Games and Platforms](Supported-Games-and-Platforms).

## Games and platforms

- PlayStation 4 titles are described by the app itself as "still a work in progress". Database editing works; the PlayStation 4 save write path is structurally checked only, and some titles' save files carry a trailer that blocks edits which change the database size.
- NHL 22, 23 and 24 open a read-only Frostbite asset browser. Nothing can be edited for those games.
- Original-console packaging of saves (for example Xbox 360 STFS containers) is not produced. Studio edits the inner save; wrapping it for a real console is up to you.
- Split PlayStation 3 saves (`DATA0000…`) cannot be opened.

## Packages and installation

- Only one mod can be active on an installation at a time. Activating another deactivates the current one first.
- A package cannot carry rows added or removed, new dictionary keys, archive-member edits, game-settings defaults, schedule edits or dictionary growth. Such changes stay in the project and are deployed from it, never shared. The build names them as blockers.
- Loose-texture packages can target another console only for NHL Legacy between Xbox 360 and the Recompilation.
- There is no general repacker for loose files into the games' `.big` archives. Keep the extracted asset structure for loose-file workflows.

## Editing

- The dressed/scratched control on rosters is known to be wired to the wrong field in the game data. Do not rely on it until this page says otherwise.
- The calculated overall rating on the roster list uses EA's formula fitted on NHL 14 and NHL 15; on NHL 10 through NHL 13 it reads roughly 13 points low. The Attributes tab only shows the calculation where it is calibrated.
- Goalie type codes on some screens are a best guess from research, not verified.
- Game Settings is a read-only catalogue except for stored-default editing on NHL 15 PlayStation 4.
- Localization editing is limited to renaming through the dictionary; a general string-table editor is not built.
- Legends (Puck's Picks) is browse-only: lines and strategy are not built and there is no path to import a Legends roster into the game.
- Arena editing does not create or duplicate arenas; ice quality, crowd type and crease settings are read-only.

## Assets and audio

- Texture Lab is a placeholder.
- Portrait and logo import from an outside library, with format conversion, is not built; you replace images one at a time.
- Audio can be previewed (speech, sound effects, goal horns, songs, commentary catalogues, depending on the game) but not replaced or re-encoded. EA-Opus speech on NHL 18 through 21 cannot be previewed.
- Curved nameplate baselines in the uniform editor are experimental and preview-only.
- Head-mesh conversion between games is being completed outside Studio and is not available in the app.

## Housekeeping

- Studio has one language, English.
- About and licence information is bundled in the archive (`THIRD-PARTY-NOTICES.txt`) but the in-app About page is not reachable from the current menus.

Last reviewed: 2026-10-20

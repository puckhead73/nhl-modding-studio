# Arenas

**Game Database › Arenas** lists the venues the loaded game stores and opens each for editing: identity, presentation and audio assignments, and a read-only view of the venue's shipped art and files. The tab is hidden on titles whose database has no arena table (NHL 07 uses its own editor, below).

## The list

Columns **ID**, **Name**, **City** and **Teams** (the number of teams playing there, "—" for none). Search covers name, city and id.

## Arena detail

The header shows the name, id, city, and pills for seats, ice type and the number of teams or **Unused**.

- **Identity**: Name, City, Capacity ("seating capacity", distinct from the arena id teams reference), Ice Surface (NHL 200×85 or international 200×100; every North American arena carries 0, every European one 1).
- **Presentation**: Avg Attendance, Stanchion, Center Line, Center Ice Logo, Arena Art ID, Power Ring, Puck, Bounce Map, Self Index. Each has a tooltip; some are record-index echoes to edit with care.
- **Audio**: Goal Horn, Period Horn, Chant Audio, City Audio, Arena Audio. On Xbox 360 and PS3 titles a horn value links to its sound bank in the [Commentary and Speech Catalogue](Commentary-and-Speech-Catalogue).
- **Teams playing here**: the teams whose home this is, or "No team references this arena as its home. Unassigned arenas are usually neutral-site venues or unused alternates."
- **Venue parameters**: every field of the title's arena table classified by name and value space (read-only, constant, number only, not in the catalogue), with constants hidden by default. "A class groups fields by name and value space — it is not a statement about what the game reads."
- **Arena assets** (hidden in the Save Editor): the shipped files joined to this arena where the research measured the join, with badges for how the binding was established and whether each file is present. Read-only; re-pointing is not offered.
- **Venue art** (hidden in the Save Editor): the venue package's materials by name, kind and slot, with previews. On PS3 only, board-ad rasters (DXT1 or DXT3) have **Replace…**: the image must be the raster's own size, the change is held as a pending change written by the next copied-target save, and the dialog names every material that shares the raster.
- **Misc fields**: everything else.

To change which arena a team plays in, use the team's **Venue** card on [Teams](Teams); arena city changes apply to every team using that arena.

## NHL 07 arena editor

NHL 07 replaces the detail page with a form in five groups: **Arena Details** (name, city, capacity, average attendance, ice surface), **Ice and Presentation** (centre-ice logo and visibility, door logo, jumbotron, puck, centre line, three ice ads, board ads and their visibility), **Audio Assignments** (goal and period horns, crowd chant, city and arena commentary) and **Venue Appearance** (roof, walls, railings, steps, board trim, seat tiers). Every choice except Arena Details is "an existing option by the venue that uses it", to be checked in game. Creating or duplicating arenas is not supported for NHL 07.

## What is not covered

Ice quality, crowd type and crease behaviour are read-only; new arenas cannot be created or duplicated on any title. See [Known Limitations](Known-Limitations).

Last reviewed: 2026-10-20

# NHLViewNG Comparison

**Evidence:** a functional description of the earlier community tool as a user of it experiences it, plus what its published data-definition files show. Nothing here describes its internals.

NHLViewNG was for years the community's editor for EA NHL rosters on consoles, and NHL Modding Studio is its spiritual successor. This page records what it did, so that readers coming from it know what carries over and what the Studio does differently.

## What NHLViewNG covered

- **Titles:** NHL 09 through NHL 16, one build with per-game definitions.
- **Files:** exported PlayStation 3 save folders (it handled the console's save protection itself), exported and decrypted PlayStation 4 save folders, rosters extracted from Xbox 360 packages, and plain `.db` files.
- **Editing:** players (bio, ratings, equipment, appearance), teams, rosters with a line editor, draft, contracts, awards, arenas and more, across the games' 134 tables.
- **Saving:** one save action that re-serialised the whole container with its integrity tags recomputed, including the NHL 10 and 11 container and the PS4 container.
- **Metadata-driven:** the editor was driven by rich per-field definitions, the externalised form of which are the "EA DB Editor" XML files that circulate in the community, giving human names, control types and grid layouts per table.

## What the Studio keeps

- The same metadata-first idea: every field knows its name, width, range and whether it is writable, read from each game's own schema file (see [TDB Metadata XML](TDB-Metadata-XML)).
- Byte-exact output: an unedited file round-trips identically, and edits change only what they must.
- The same mental model of rosters, lines, pools and companions, now verified on the games' files ([Roster Model](Roster-Model)).

## What the Studio does differently

| Area | NHLViewNG | Studio |
|---|---|---|
| Title range | NHL 09 to 16 | NHL 07 to 21, NHL Legacy and the Recompilation, on their actual platforms |
| Roster slots | a fixed 40-slot block per team was assumed | blocks are real but **variable in size** (20 to 35 slots); the Studio enforces and repairs the block layout |
| Free agents and hidden players | three "pools" in the interface | the same pools, now understood as slot presence, with player type kept separate from roster status |
| Editing target | the save file | the game's **default database** in a project, with saves handled by a separate Save Editor |
| Sharing | edited saves | `.nhlmod` packages of changes, installed and reversible through the Mod Manager |
| Imports | none | a 1909-to-present historical database with rating conversion and roster building |
| Assets | none | portraits, logos, uniforms, equipment, heads, audio previews |
| Validation | field-level ranges | field ranges plus entity invariants plus a whole-database audit on save |

## Known mismatch corrected along the way

Several facts the community inherited from the earlier tool's era turned out to need refinement when measured on the games' files: the block size above, the identity of the roster-status field (the player-type field was being read as status), and the exact rating offsets on the older titles. The knowledge base pages carry the measured versions.

Last reviewed: 2026-10-20

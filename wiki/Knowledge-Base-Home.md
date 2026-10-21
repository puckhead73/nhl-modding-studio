# Modding Knowledge Base

**Evidence:** index page; each linked page carries its own evidence line.

This section collects what the NHL Modding Studio project has established about how EA's NHL games store their data, from NHL 07 on Xbox 360 to NHL 21 on PlayStation 4. It exists so that the knowledge outlives any one tool. Everything here was measured on the games' own files or inferred from such measurements, and every page says which.

Start with [Evidence and Verification Conventions](Evidence-and-Verification-Conventions) to read the labels correctly, then [EA NHL Series Overview](EA-NHL-Series-Overview) for the map of engine generations the rest of the pages refer to.

## Databases

- [TDB Database Format](TDB-Database-Format): the binary layout of EA's TDB version 6 files, bit-packed records, CRCs.
- [TDB Metadata XML](TDB-Metadata-XML): the schema files that describe each title's tables and fields.
- [TDB Id Reversal and Naming](TDB-Id-Reversal-and-Naming): why the same table has two four-letter names.
- [TDB String Pools and Localization Database](TDB-String-Pools-and-Localization-Database): Huffman-coded text, the dictionary database and its limits.
- [Schema Drift Across Titles](Schema-Drift-Across-Titles): what changes from one year and platform to the next.
- [Schema Index](Schema-Index): generated per-title pages listing every table and field of each reference database.
- [Surface Capabilities by Title](Surface-Capabilities-by-Title): generated matrix of which editing surfaces each title supports.
- [Field Encodings](Field-Encodings): ratings, birth dates, heights, weights, nationality codes and other values that are not stored the way they are shown.
- [Roster Model](Roster-Model): how teams, roster slots, players and their companion rows link together.

## Saves

- [Save Containers for Xbox 360 and PS3](Save-Containers-Xbox-360-and-PS3): the roster-file container, its versions and checksums.
- [Save Containers for PS4](Save-Containers-PS4): the record stream, LZ4 and chunkzip payloads, trailers.
- [Save Formats by Title](Save-Formats-by-Title): generated table of every sample save's detected type, codec and round-trip status.

## Game files

- [Archives and Compression](Archives-and-Compression): BIG4, BIGF, the EB archive generations, RefPack, chunkzip and chunklzx.

## Art and assets

- [Textures](Textures): RenderWare texture libraries on Xbox 360 and PS3, XPR2 and DDS front-end art, PS4 GNF, and what a resize must respect.
- [Logos and League Imagery](Logos-and-League-Imagery): the league art id, package chains per platform, team crests and the logo set.
- [Meshes and Rigs](Meshes-and-Rigs): RenderWare player meshes, the shared 122-bone rig, PS4 RSF containers.
- [Head UV Templates and Transfer](Head-UV-Templates-and-Transfer): which heads share a UV layout and how a head is carried between games.
- [Player Heads, Portraits and Appearance Binding](Player-Heads-Portraits-and-Appearance-Binding): portrait and head ids, bespoke and pool heads, the PS4 binding chain.
- [PS4 Attrib/VLT Vault](PS4-Attrib-VLT-Vault): the typed registry that binds PS4 database ids to files.
- [Equipment and Goalie Gear](Equipment-and-Goalie-Gear): file grammar, goalie mask chains, PS4 catalogues.

## Leagues, teams and arenas

- [Leagues, Competitions and Schedules](Leagues-Competitions-and-Schedules): league codes and keys, groups, schedule tables and the settings catalogue.
- [Team Identity, Logos and Kits](Team-Identity-Logos-and-Kits): the team row, the art id, kits, fonts and glyph coverage.
- [Arenas, Boards and Venues](Arenas-Boards-and-Venues): the arena table, venue packages, PS4 levels, board ads.

## Audio

- [Audio](Audio): the map of containers and codecs per title.
- [Audio Banks and Streams](Audio-Banks-and-Streams), [Audio Codecs](Audio-Codecs), [Goal Horns, Chants and Music](Goal-Horns-Chants-and-Music), [Commentary and Speech Data](Commentary-and-Speech-Data).

## Other runtimes

- [NHL Legacy Recompiled](NHL-Legacy-Recompiled): the PC recompilation of the Xbox 360 game.
- [Frostbite: NHL 22 to 24](Frostbite-NHL-22-24): the engine after the TDB era, read-only.

## Sources and tools

- [Historical Corpus and Rating Formulas](Historical-Corpus-and-Rating-Formulas): the 1909-to-present hockey database and how its ratings map onto EA's.
- [NHLViewNG Comparison](NHLViewNG-Comparison): how the earlier community editor approached the same files, as a functional comparison.

Last reviewed: 2026-10-20

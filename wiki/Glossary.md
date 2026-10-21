# Glossary

**Anchor.** A reference copy of a title's shipped database, metadata and sample saves, one set per title and platform, used to measure format facts and as the bundled baseline for new projects.

**Archive (`.big`).** A container file holding many of a game's assets, and from NHL 11 its databases. Three directory formats exist: BIG4, and two generations of EB. See [Archives and Compression](Archives-and-Compression).

**Assets-only project.** A project created from a folder of extracted assets with no database; it stages texture edits and packages without a database.

**Automated-tested.** A support level: a named automated test executed the operation against reference data. See [Evidence and Verification Conventions](Evidence-and-Verification-Conventions).

**Baseline.** The bundled reference database a new project starts from.

**BIGF package.** A small inner archive holding front-end art such as league logos, found inside the larger archives.

**Build.** The action that turns a project's changes into `.nhlmod` packages.

**Change list.** The form of NHL 07 and NHL 08 roster saves: a list of inserts, deletes and updates against the game's database rather than a full roster.

**chunkzip, chunklzx.** Chunked compression wrappers around archive members: zlib-based on PS3 and PS4, LZX-based on Xbox 360.

**Companion row.** A row in a skater or goalie ratings or equipment table that shares a player's key. A usable player is a bio plus its companions. See [Roster Model](Roster-Model).

**Copied-target save.** On PS4, a save of the project that stages changes inside a copy of the game's archive, with restore; archive-member edits stay pending until it runs.

**Developer Mode.** The name reserved for the advanced mode of the application. Not part of the current beta.

**Dictionary.** The localization database (`nhl_eng_us.db` or `eng_us.db`) that holds the game's displayed text, keyed by hash.

**EB archive.** The `.big` directory format used from NHL 11 on Xbox 360 and PS3 and on every PS4 title, in 16-byte-row and 20-byte-row variants.

**Extraction.** Unpacking a game's archives into loose files beside the game so that assets can be read and edited.

**FHM corpus.** The historical hockey database, built from a management simulation's exports into SQLite.

**Formula profile.** A named mapping from historical attributes to EA ratings, per position.

**Frostbite.** The engine of NHL 22 and later, unrelated to the TDB-based titles.

**Game-verified.** A support level: a recorded manual run on the exact title, platform and build. No title has reached it.

**gen6, gen7, Kettle.** The project's names for the engine families: NHL 07; NHL 08 to Legacy on Xbox 360 and PS3; NHL 15 to 21 on PS4. See [EA NHL Series Overview](EA-NHL-Series-Overview).

**Huffman pool (type-66 pool).** The compressed string storage of a TDB table, used by the dictionaries.

**Implemented.** A support level: the code exists.

**Installation record.** The record the Mod Manager keeps of an activation, with backups, so the package can be deactivated or recovered.

**Legends rating.** The EA overall computed over each attribute's three best seasons; what Puck's Picks ranks on.

**Main database.** The game's default data: `nhlng.db`, `NHL07NG.db` or `nhl_game.db`.

**Mod Manager.** The tool that activates and deactivates `.nhlmod` packages on a game installation.

**`.nhlmod`.** A package of changes built from a project, installed by the Mod Manager.

**`.nhlproject`.** A complete project archived for another Studio user.

**Project.** The folder that holds one mod's working files and change record.

**Project Overview.** The page listing every change in a project, with project details and the build actions.

**Pool (free agents, hidden).** Players whose index row is referenced by no roster slot.

**Raw id, real id.** The two spellings of a four-letter table or field id, reversed relative to each other on big-endian files. See [TDB Id Reversal and Naming](TDB-Id-Reversal-and-Naming).

**RefPack.** EA's stream compressor used for front-end art on the Xbox 360 and PS3 titles.

**Roster block.** The contiguous run of roster-table records that belongs to one team.

**Roster save.** The save file holding a user's roster; a subset of the main database (NHL 09 on) or a change list (NHL 07 and 08).

**Save Editor.** The tool that edits one save file or database in place.

**Session.** The open editing state of a project: its undo history and unsaved changes.

**Slug.** The export name of a player, `lastname_firstname_dd_mm_yyyy`.

**Store (Hockey Shop).** The per-slot library of equipment textures kept in Studio's application data.

**TDB.** EA's database format, version 6, used by every title from NHL 07 to NHL 21. See [TDB Database Format](TDB-Database-Format).

**Unverified.** A tab or field shown read-only because its meaning is not confirmed for the loaded game.

Last reviewed: 2026-10-20

# Glossary

Domain terminology used across NHL Modding Studio.

- **TDB** — EA's proprietary database format used by the NHL titles to store editable game data
  (players, teams, rosters, attributes). Format details vary by year and platform. See
  [03-tdb-format.md](03-tdb-format.md).
- **NHLViewNG** — the prior community tool this project is the spiritual successor to.
- **Title** — a specific game in the series (NHL 07 … NHL 21, plus NHL Legacy and NHL Legacy
  Recompiled).
- **Platform** — the console target: Xbox, PS3, PS4, or the Legacy Recompilation.
- **NHL Legacy Recompilation** — a recompilation based on the Xbox build of the game; treated as a
  distinct platform target that closely tracks the Xbox data layout.
- **Save data** — per-user, platform-wrapped saved game state, distinct from the shipped TDB
  databases. See [05-save-data.md](05-save-data.md).
- **Source database / historical database** — the built-in, independently compiled database of
  hockey players/teams/rosters from 1909 → present, used as an import source. Implemented as the
  **FHM corpus**. See [04-source-database.md](04-source-database.md).
- **FHM corpus** — the SQLite embodiment of the historical source database, built from FHM
  (Franchise Hockey Manager) CSV data by the `tdb-fhm*` pipeline. Read-mostly; the import source.
- **studio-api / studio-domain** — the two Rust service layers above the engine: `studio-api` is the
  thin Tauri/DTO/registry surface; `studio-domain` is pure, GUI-agnostic orchestration. See
  [DESIGN.md](DESIGN.md).
- **ImportChangeSet / preview→commit** — the import PLAN model; preview and commit lower through the
  same code path so they produce byte-identical results (a golden-test invariant).
- **Parity** — byte-for-byte equivalence with the NHLViewNG / reference-app save/write output;
  the engine's correctness guardrail.
- **type-66 pool** — Huffman-coded string/localization pool in the TDB that must be rebuilt
  (pool-fit constrained) when localized strings change.
- **Domain model** — the format-agnostic representation (Player, Team, Roster, League, Season) the
  UI and import pipeline operate on, decoupled from any single game's raw schema.
- **Codec / schema registry** — the layer that resolves the correct binary layout for a given
  (title, platform) pair.
- **Endianness** — byte order; big-endian on PowerPC platforms (Xbox 360 / PS3), little-endian on
  x86-64 (PS4).
- **Round-trip** — reading data and writing it back; a lossless round-trip reproduces the original
  bytes when no edits are made.
- **rexglue-sdk** — a graphics/Vulkan reference SDK present on the build machine; a potential
  reference for later asset/graphics work, not part of the initial data-layer scope.

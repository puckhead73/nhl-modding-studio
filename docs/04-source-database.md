# 04 — Historical Source Database

A defining feature of NHL Modding Studio is a **built-in historical hockey database** used as a
*source* from which to import players, teams, and rosters into any supported game.

## Scope of the source database

- **Time span:** 1909 → present (~117 years of hockey).
- **Breadth:** hundreds of leagues worldwide, not just the NHL.
- **Entities:** players (with biographical and rating-relevant data), teams, and rosters/seasons.

This is an **independently compiled** dataset. It contains no EA-copyrighted content and is the
project's own data asset.

## Role in the tool

```
Historical source DB ──► mapping/import pipeline ──► target game's TDB
   (format-agnostic)        (domain model)           (version/platform-specific)
```

- The source DB is expressed in the format-agnostic **domain model** (Player/Team/Roster/League/
  Season).
- The **import pipeline** maps domain entities onto a specific game's schema for a chosen
  (title, platform), handling attribute scaling, name/ID fitting, and roster-size constraints.

## Open design questions (to resolve with the roadmap)

- **Storage format:** embedded SQLite is the leading candidate (queryable, file-based, ships with
  the app); alternatives include a packed binary or a set of structured data files. **TBD.**
- **Distribution:** bundled with the app vs downloaded/updatable as a separate data pack. Given its
  size and update cadence, a separately versioned, downloadable data pack is the working assumption.
  (Note: `.gitignore` currently excludes `*.db`; if the source DB is committed in any form, add an
  explicit exception and confirm it carries no copyrighted data.)
- **Schema:** canonical tables for players, teams, leagues, seasons, and roster memberships, plus
  the attribute fields needed to drive game imports.
- **Provenance & licensing:** track where data came from and under what terms; document before
  release.
- **Attribute mapping:** how historical/biographical data becomes game ratings per title (ratings
  scales differ across NHL 10–21).

## Requirements

- Fast lookup/search across a large dataset (responsive UI for the import wizard).
- Deterministic, repeatable imports (same source + same target → same result).
- Clear handling of partial matches and constraints (e.g., a target roster that can't fit all
  selected players).
- Versioned schema so the data pack can evolve independently of the app.

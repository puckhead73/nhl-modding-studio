# 00 — Vision

> Architecture: [DESIGN.md](DESIGN.md) · Sequencing: [ROADMAP.md](ROADMAP.md) · Scope matrix:
> [01-scope-and-compatibility.md](01-scope-and-compatibility.md).

## Why NHL Modding Studio exists

The EA NHL modding community has long relied on a patchwork of aging, single-purpose tools.
**NHLViewNG** was a milestone, but it was constrained to a narrow set of titles and use cases.
NHL Modding Studio is the **consolidated, polished successor**: one coherent application that
spans a decade-plus of EA NHL titles, multiple platforms, save data, and a rich import pipeline.

## Principles

1. **Simple by default, deep on demand.** A first-time user should be able to open a database,
   change a player rating, and save — without reading a manual. A power user should be able to
   reach raw tables, batch operations, and cross-database imports.
2. **Correctness over convenience.** Game data is fragile and version-specific. We never silently
   corrupt a database. Writes are validated, reversible where possible, and explained.
3. **Version- and platform-honest.** We model the real differences between NHL 10–21 and between
   Xbox / PS3 / PS4 / Legacy rather than pretending they are uniform.
4. **No piracy, no copyrighted data.** The tool edits the user's own legally obtained files and
   ships none of EA's content.
5. **Longevity.** Clear architecture and documentation so the project outlives any single author.

## Success criteria for the release candidate

- Open, edit, and safely save TDB databases for the supported titles without data loss.
- Round-trip save data on at least the primary supported platforms.
- Import players/teams/rosters from the historical source database into a target game.
- A UI a newcomer can use unaided, with advanced surfaces available for experts.
- Documentation sufficient for a new contributor to be productive.

## Audiences

- **Casual modders** — tweak rosters, ratings, teams; minimal friction.
- **Power users / roster makers** — bulk edits, historical imports, deep schema access.
- **Contributors** — extend format support and features.

## Legal & ethics

NHL Modding Studio is a fan project, not affiliated with or endorsed by EA. It:

- Operates only on files the user already owns.
- Ships **no** EA-copyrighted databases, assets, or save data.
- Compiles its historical hockey database from independently gathered information.

The project license is **to be determined** (a permissive OSS license is the working assumption).
Resolve this before the first public release.

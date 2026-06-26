# 04 — Historical Source Database (FHM corpus)

A defining feature of NHL Modding Studio is a **built-in historical hockey database** used as a
*source* from which to import players, teams, and rosters into any supported game.

> **This is the FHM corpus.** In the engine and [DESIGN.md](DESIGN.md) this dataset is the
> **FHM corpus** — an embedded **SQLite** database built by the `tdb-fhm` / `tdb-fhm-store` /
> `tdb-fhm-build` pipeline from FHM CSV data. "Historical source database (1909→present)" and "FHM
> corpus" are the same thing.

## Scope of the source database

- **Time span:** 1909 → present (~117 years of hockey).
- **Breadth:** hundreds of leagues worldwide, not just the NHL.
- **Entities:** players (bio + career + rating-relevant data), teams, rosters/seasons, contracts,
  jerseys, captaincy, draft.

Independently compiled; contains no EA-copyrighted content.

## Role in the tool

```
FHM corpus (SQLite) ──► formula engine ──► import pipeline ──► target game's TDB
   (1909→present)        (FHM→EA ratings)   (preview/commit)    (title/platform-specific)
```

- The corpus is read-mostly SQLite (`rusqlite`).
- The **formula engine** computes EA ratings from FHM/biographical data (normalisation modes,
  archetype classification, saved profiles) — ratings scales differ across NHL 10–21.
- The **import pipeline** is two-phase **preview → commit** over an `ImportChangeSet` PLAN; preview
  and commit lower through the *same* code path so they are byte-identical (a golden-test invariant).
- The **instance registry** (separate SQLite sidecar) ledgers imported player-season instances for
  dedup (non-blocking).

See [DESIGN.md](DESIGN.md#service-layer--studio-api--studio-domain) for the import pipeline and
formula engine, and [ROADMAP.md](ROADMAP.md) Phase 5.

## Build & distribution

- **Build:** `tdb-fhm-build` ingests FHM CSV (season/career/captaincy/jersey/contract/draft) into the
  SQLite corpus as a build-time step (a separate bin, not part of the app runtime).
- **Distribution:** the corpus ships as bundled resources / a versioned data pack. Because it can be
  large, prefer a separately versioned downloadable data pack over committing it to the repo.
  (Note: `.gitignore` currently excludes `*.db`; if any corpus artifact is committed, add an explicit
  exception and confirm it carries no copyrighted data.)

## Requirements

- Fast lookup/search across a large dataset (responsive Roster Builder UI), e.g. a lazy
  `EaSlugIndex` for slug→EA-player matching (ambiguity-aware).
- Deterministic, repeatable imports (same source + same target → same bytes).
- Clear handling of partial matches and capacity constraints (a target roster that can't fit all
  selected players surfaces storage blockers in the preview).
- Versioned schema so the data pack evolves independently of the app.

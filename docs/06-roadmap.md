# 06 — Roadmap

> **Placeholder.** A detailed roadmap is being prepared from the prior rough-draft application and
> will be ingested here. Once provided, this document becomes the authoritative sequencing of work.

## Working phase outline (provisional, pending the real roadmap)

1. **Foundations**
   - Scaffold the Tauri app (Rust core + web UI), CI, and test harness.
   - Establish the schema/codec registry keyed by (title, platform).
   - Define the format-agnostic `domain` model.

2. **TDB core**
   - Lossless read/write for an initial target (title, platform).
   - Expand the per-title/per-platform schema coverage across NHL 10–21.
   - Validation, backups, and safe-write guarantees.

3. **Editing UI**
   - Database browser and core editors (players, teams, rosters).
   - Simple-by-default flows with progressive disclosure for power users.

4. **Save data**
   - Open/inspect/edit platform save containers; integrity handling per platform.

5. **Historical source database**
   - Storage/distribution decision, schema, and the import pipeline.
   - Import wizard: source DB → target game.

6. **Release candidate**
   - Cross-platform packaging, docs, licensing resolution, polish.

## Notes captured for the roadmap

- Verify the title × platform release reality (see [01-scope-and-compatibility.md](01-scope-and-compatibility.md)).
- Fold confirmed TDB and save-format facts from the reference implementation into
  [03-tdb-format.md](03-tdb-format.md) and [05-save-data.md](05-save-data.md).
- Resolve source-DB storage/distribution and licensing ([04-source-database.md](04-source-database.md)).

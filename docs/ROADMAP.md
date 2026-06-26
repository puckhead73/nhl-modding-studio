# ROADMAP — NHL Modding Studio

> **Canonical phased plan.** Derived from the prior app's design brief
> ([rebuild-design.md](rebuild-design.md), kept as the reference source) and extended for the agreed
> release-candidate scope. Architecture lives in [DESIGN.md](DESIGN.md).

## Scope this roadmap delivers

- **Titles:** NHL 10 → NHL 21.
- **Platforms:** Xbox, PS3, **PS4**, NHL Legacy Recompilation.
- **Functionality:** core studio + production asset tools (see [DESIGN.md](DESIGN.md#scope-release-candidate)).

The reference app proved **Legacy/14/15 on PS3/Xbox/Recomp**. Two scope deltas are **net-new** work,
called out as their own phases below:

- **PS4 support** — little-endian x86 engine paths + a different save container.
- **Full NHL 10–21 title range** — per-title schema reversing for years the reference app did not
  cover. Support is gated by *verified* round-trip + in-game validation per (title, platform); see
  [01-scope-and-compatibility.md](01-scope-and-compatibility.md) for the live status matrix.

## Phases

**Phase 0 — Repo scaffold & contracts.** Cargo workspace + Tauri v2 + Vite/React. Wire `ts-rs`/
`specta` DTO codegen. CI: `cargo test`, `tsc`, lint. Bundled-resources layout (games/, vanilla-saves/,
corpus/). *Gate:* empty app builds and runs on all dev platforms; CI green.

**Phase 1 — Engine port.** Bring over `tdb-core`, `tdb-edit`, `tdb-defs`, `tdb-enums`,
`tdb-relations`, `tdb-domain-validation`, `tdb-savedata` with their tests. *Gate:* full test suite
green; round-trip + CRC + save-container byte-equality pass for the proven titles.

**Phase 2 — Service skeleton.** `studio-api` registry + project load/save spine; `studio-domain`
edit-session orchestration. *Gate:* load a real nhlng.db, stage an edit, validate, save, byte-diff
vs. reference-app output.

**Phase 3 — EA DB core views.** Dashboard, Teams, Arenas, Players (+ career stats), Lines/depth
chart, Pending Edits, save-as/pack. *Gate:* a usable editor for the proven titles.

**Phase 4 — Roster ops & transactions.** Entity ops (repairing transactions), Transactions view,
roster capacity normalization, repair tools.

**Phase 5 — Historical (FHM) + import.** FHM corpus build (the 1909→present historical source DB),
Historical Teams/Players, Formulas, Roster Builder (preview/commit pipeline + instance registry).

**Phase 6 — Production asset tools.** Texture Lab, Portraits/Logos, Equipment/Hockey Shop,
Appearance Studio, Localization, Music/PA/PBP.

**Phase 7 — Shell polish & onboarding.** Startup/onboarding, Builds, Settings + bulk tools/exports,
session header UX.

**Phase 8 — Parity gap closure.** Finish deferred parity items: v1 + Xbox-STFS save write-back,
companion-row auto-creation on position flip, multi-select transactions, clone varchar-name edit.

### Net-new scope phases (beyond the reference app)

**Phase 9 — PS4 platform.** Add `Ps4` to the platform model: little-endian x86 codec paths in
`tdb-core`, PS4 save-container extract/pack + integrity in `tdb-savedata`, onboarding for PS4 roots.
*Gate:* round-trip + container byte-equality + in-game/PS4-tool validation for at least one title.

**Phase 10 — Title-range expansion (NHL 10–21).** Reverse and register per-title schemas for the
years the reference app did not cover, plugging each into the (title, platform) registry. Iterative;
each (title, platform) flips to ✅ in the matrix only when round-trip + in-game verified. *Gate:* the
full matrix reaches its target support level (see scope doc) or documented exceptions.

> Phases 9–10 run partly in parallel with 3–8 where the architecture allows (the registry and codec
> seams are established early in Phases 1–2 precisely so the matrix can expand without churn).

## Verification gates

Each phase is gated by the relevant parts of the verification strategy in
[DESIGN.md](DESIGN.md#verification-strategy): engine test suites green, golden byte-equality vs. the
reference app, DTO round-trip, domain audit on known-clean/dirty saves, and an end-to-end emulator/
PS4 smoke for parity-critical paths.

## Risks

See [DESIGN.md](DESIGN.md#risks). The roadmap-specific watch items: hidden coupling in the reference
`commands.rs`, asset-codec edge cases, unfinished v1/Xbox-STFS write-back, and the unproven breadth
of per-title schema reversing across the full NHL 10–21 range (Phase 10) — treat as discovery work,
never assume a year's layout matches a neighbor's until verified.

## Out of scope (deferred to a separate `nhl-research` roadmap)

ReXGlue recomp, NHL2K10 RE, rig-audit, head-import
([MESH_IMPORT_EXCLUSIVE_PATH.md](MESH_IMPORT_EXCLUSIVE_PATH.md)), NHL24 weight-remap, FMT plugins,
EBOOT tunable patcher.

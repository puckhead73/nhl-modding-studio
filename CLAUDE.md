# CLAUDE.md — working notes for AI assistance

This file orients any AI assistant (and new contributors) working in this repository.

## What this project is

**NHL Modding Studio** — a release-quality modding toolkit for EA's NHL games (NHL 10 → NHL 21),
the spiritual successor to NHLViewNG. It edits EA **TDB** databases, handles platform save data,
and imports players/teams/rosters from a built-in historical hockey database (1909 → present).

Read [README.md](README.md) and the [docs/](docs/) folder for full context before making changes.

## Canonical docs

- **[docs/DESIGN.md](docs/DESIGN.md)** — architecture, engine crate set, `studio-api`/`studio-domain`
  layers, feature inventory, cross-cutting systems. The authoritative architecture.
- **[docs/ROADMAP.md](docs/ROADMAP.md)** — phased plan + verification gates. Align new work here.
- Both are derived from [docs/rebuild-design.md](docs/rebuild-design.md) (the prior app's brief,
  kept as the reference source).

## Tech stack

- **Tauri v2** app: **Rust** core (`src-tauri/`) + **React + TanStack Query** frontend (`src/`).
- **SQLite** (rusqlite) for the FHM historical corpus.
- Rust toolchain: stable (currently 1.96). JS package manager: **npm** (pnpm not installed).
- Binary parsing (TDB, save formats) lives in Rust for performance and safety.
- Generate TS DTO types from Rust (`ts-rs`/`specta`) — do not hand-maintain `dto.ts`.

> The application has **not been scaffolded yet** — only documentation and the repo skeleton exist.
> The rebuild starts at ROADMAP Phase 0.

## Scope (release candidate)

- Titles **NHL 10 → 21**; platforms **Xbox / PS3 / PS4 / Legacy Recomp**.
- The reference app proved **Legacy/14/15 on PS3/Xbox/Recomp**; **PS4** (Phase 9) and the **rest of
  the title range** (Phase 10) are net-new. Never assume an unproven (title, platform) layout matches
  a neighbor's — gate support on verified round-trip + in-game checks.
- Deferred/out-of-scope research: recomp, NHL2K10 RE, rig-audit, head-import
  ([docs/MESH_IMPORT_EXCLUSIVE_PATH.md](docs/MESH_IMPORT_EXCLUSIVE_PATH.md)), NHL24 weight-remap,
  FMT plugins, EBOOT patcher.

## Hard rules

1. **Never commit copyrighted EA game data.** No real `.tdb`/`.db`/`.viv`/save files in the repo.
   `.gitignore` enforces this; do not weaken it. Use synthetic/redacted fixtures for tests.
2. **Format facts must be verified, not guessed.** TDB has real per-year/per-platform nuances.
   Do not assert binary-layout specifics you have not confirmed against the reference
   implementation or real (locally held, un-committed) data. Mark unverified specifics clearly.
3. **Version- and platform-aware by default.** Anything touching game data must account for the
   title (10–21) and platform (Xbox / PS3 / PS4 / Legacy) it targets.
4. **Two audiences.** Keep the common path simple; put depth behind progressive disclosure.

## Conventions

- Keep documentation in `docs/` current when scope or architecture changes.
- Conventional-style commit subjects; end commit messages with the required Co-Authored-By line.
- Branch off `main`; commit/push only when asked.

## Useful references

- Prior rough-draft application (referenced, not imported) — ask the user for its location.
- Reference working dir on this machine: `E:\Tools\rexglue-sdk` (graphics/Vulkan reference).

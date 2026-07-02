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

1. **Clean-room repository — no outside influence.** This repo is built from the ground up. Do
   **not** read, open, search, clone, or copy from any other tool repository on this machine — the
   prior `nhl-database-studio` app, `E:\Tools\rexglue-sdk`, or anything else — **unless the user
   explicitly points you at it for that specific instance**. The only prior-app material permitted
   here is what the user has deliberately placed in this repo (e.g. `docs/rebuild-design.md`,
   `docs/MESH_IMPORT_EXCLUSIVE_PATH.md`). When prior-app knowledge would help, ask first.
2. **EA data policy (updated 2026-06-26).** Vanilla EA game data that **ships with the tool** — the
   `anchors/` databases and save files (the clean-slate baseline) — **is committed** (it's data
   shipped with the tool, not proprietary code). Anchors are **plain git binaries** (all files are
   small — largest ~6.5 MB — so no LFS; git's own packing keeps `.git` small). Outside `anchors/`,
   still keep stray/real game files and private working data out of the repo (the broad
   `*.db`/`*.sav`/… patterns remain); use synthetic fixtures for tests.
3. **Format facts must be verified, not guessed.** TDB has real per-year/per-platform nuances.
   Do not assert binary-layout specifics you have not confirmed against material the user has
   provided in this repo or real (locally held, un-committed) data. Mark unverified specifics clearly.
4. **Version- and platform-aware by default.** Anything touching game data must account for the
   title (10–21) and platform (Xbox / PS3 / PS4 / Legacy) it targets.
5. **Two audiences.** Keep the common path simple; put depth behind progressive disclosure.

## Conventions

- Keep documentation in `docs/` current when scope or architecture changes.
- Conventional-style commit subjects; end commit messages with the required Co-Authored-By line.
- Branch off `main`; commit/push only when asked.
- **Human-readable surfaces (rule).** Anything shown to the end user — findings, errors, labels,
  summaries, tooltips — must use the **localized / resolved reference**, never the internal TDB
  convention. Resolve studio table/field ids to their metadata `name`, and numeric entity ids to
  their real names (team → team name, player → player name, etc.) wherever a resolution exists.
  Keep the raw ids available only behind progressive disclosure (a tooltip / “details” affordance)
  for power users, never as the primary text. Do the resolution at the layer that has the name
  data (studio-domain/API), not by string-parsing in the frontend.

## Useful references

- The canonical docs above and any material the user places in this repo are the **only** in-repo
  sources of prior-app knowledge.
- Other repositories on this machine (the prior `nhl-database-studio` app, `E:\Tools\rexglue-sdk`,
  etc.) are **off-limits** unless the user explicitly directs you to one (see Hard rule 1). The
  design/roadmap describe *intent* to port the proven engine; the actual code is only consulted when
  the user opens that door.

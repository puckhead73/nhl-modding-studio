# CLAUDE.md — working notes for AI assistance

This file orients any AI assistant (and new contributors) working in this repository.

## What this project is

**NHL Modding Studio** — a release-quality modding toolkit for EA's NHL games (NHL 10 → NHL 21),
the spiritual successor to NHLViewNG. It edits EA **TDB** databases, handles platform save data,
and imports players/teams/rosters from a built-in historical hockey database (1909 → present).

Read [README.md](README.md) and the [docs/](docs/) folder for full context before making changes.

## Tech stack

- **Tauri** app: **Rust** core (`src-tauri/`) + **web** frontend (`src/`).
- Rust toolchain: stable (currently 1.96). JS package manager: **npm** (pnpm not installed).
- Binary parsing (TDB, save formats) lives in Rust for performance and safety.

> The application has not been scaffolded yet — only documentation and the repo skeleton exist.
> A detailed roadmap derived from the prior rough-draft app is being ingested into
> [docs/06-roadmap.md](docs/06-roadmap.md). Prefer to align new work with that roadmap.

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

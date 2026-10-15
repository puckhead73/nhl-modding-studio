# CLAUDE.md — working notes for AI assistance

**Current direction, 17 September 2026:** follow [entity editing and cross-game parity](docs/plans/entity-editing-parity-roadmap.md): leagues → teams → players → equipment → miscellaneous presentation, one step replicated across applicable targets. Projects/Builds and canonical Project-specific completion are deferred to final optional stretch work, overriding older required-scope statements below and in C0/C8. Existing implementation ownership and direct-editing safety requirements remain. The new roadmap's named arena/ice/presentation operations are active scope.

Entry point for any AI assistant (and new contributors) working in this repository. Keep it short:
it points at the authoritative documents rather than restating them.

## What this project is

**NHL Modding Studio** — a release-quality modding toolkit for EA's NHL games, the spiritual
successor to NHLViewNG. It edits EA **TDB** databases, handles console save containers, imports
players/teams/rosters from a built-in historical hockey corpus (1909 → present), and edits the
games' graphical and audio assets.

The application **is implemented** — roughly 82k lines of Rust, 24k lines of TypeScript and 115
Tauri commands as of 2026-09-05. It is not a scaffold.
[docs/STATE-OF-THE-REPO.md](docs/STATE-OF-THE-REPO.md) is the current implementation inventory.

## Where the code actually lives

**Build storage maintenance (30 September 2026):** routine dev/test profiles disable full debug symbols and incremental caches. Check free space at session start; investigate a `target` above 40 GiB or E: below 20 GiB free. Coordinate any cache cleanup at an idle checkpoint; never delete another session's active artifacts or unowned/recovery data. See [storage cleanup and retention policy](docs/validation/storage-cleanup-20260930.md).

| Path | Contents |
|---|---|
| `crates/` | 20 Rust crates: engine (`tdb-core`, `tdb-edit`, `tdb-defs`, `tdb-enums`, `tdb-relations`, `tdb-domain-validation`, `tdb-savedata`, `tdb-formula`), codecs (`tdb-rx2`, `tdb-archive-big`, `tdb-archive-codec`), historical (`tdb-fhm`, `tdb-fhm-build`, `tdb-fhm-store`), sidecars (`tdb-instance-registry`), QA (`tdb-qa`), mesh (`tdb-headmesh`, `blender-job`), services (`studio-domain`, `studio-api`) |
| `app/` | React + TanStack Query frontend; generated DTO bindings in `app/src/bindings/` |
| `src-tauri/` | **Thin Tauri v2 shell only** (`lib.rs`, `main.rs`, `capabilities/`, `tauri.conf.json`) — not the Rust core |
| `anchors/` | Immutable vanilla ground truth: 552 files, **27 main-database anchors** (10 PS3, 10 Xbox 360, 7 PS4) plus metadata, localization DBs and save fixtures |
| `defs/` | Metadata, enum and relationship definitions, plus `defs/appearance/*.toml` |
| `docs/` | Canonical design docs, plans, `audits/` (verified format evidence), `validation/` (work-package records) |
| `corpus/` | Built FHM SQLite corpus — untracked, ~800 MB |

Toolchain: Rust stable (`rust-version = 1.96`); **npm** (pnpm is not installed); Node 20 in CI.

## Authoritative references

| Doc | Role |
|---|---|
| [docs/plans/entity-editing-parity-roadmap.md](docs/plans/entity-editing-parity-roadmap.md) | **Current execution sequence:** leagues, teams, players, equipment and presentation; parity after each step. |
| [docs/plans/coordinator-developer-working-agreement.md](docs/plans/coordinator-developer-working-agreement.md) | **Binding work-package process:** Codex leads/reviews; Claude implements; cleanup, receipt format and session handoffs. |
| [docs/plans/project-completion-roadmap.md](docs/plans/project-completion-roadmap.md) | Detailed C0–C10 requirements, subject to the current roadmap's scope overrides. |
| [docs/plans/parallel-completion-plan-2026-09-16.md](docs/plans/parallel-completion-plan-2026-09-16.md) | **Current ownership and scheduling.** Three separate streams; Studio feature completion consumes existing findings. Supersedes the discovery-first sequence below. |
| [docs/plans/completion-design-questions.md](docs/plans/completion-design-questions.md) | Accepted decisions (Q-IDs) and the queue of open questions |
| [docs/DESIGN.md](docs/DESIGN.md) | Architecture: layers, crate set, cross-cutting systems |
| [docs/EDIT-MODEL.md](docs/EDIT-MODEL.md) | **Binding** database-editing contract |
| [docs/plans/full-game-qa-editability.md](docs/plans/full-game-qa-editability.md) | Detailed QA and verification contract |
| [docs/01-scope-and-compatibility.md](docs/01-scope-and-compatibility.md) | Titles, platforms and per-target evidence level |
| [docs/STATE-OF-THE-REPO.md](docs/STATE-OF-THE-REPO.md) | What is actually implemented today |
| [docs/audits/](docs/audits/) · [docs/validation/](docs/validation/) | Verified format evidence · work-package baseline records |
| [docs/ROADMAP.md](docs/ROADMAP.md) | **Historical.** The original phase record, including its stale NHL 10–21 scope wording. Do not sequence work from it. |

## Scope (accepted — do not re-derive or re-litigate)

- **17 catalog entries**: NHL 07 → NHL 21, NHL Legacy and NHL Legacy Recompiled, on the actual
  catalog platform combinations (Xbox 360, PS3, PS4, Legacy Recompiled) — **27 shipping targets**.
  This is not a Cartesian matrix, and **there is no Xbox One target**.
- **NHL 07 PS3 is research-only**, not a shipping target. The 27 main-database anchors are a
  *different* set from the 27 shipping targets: they include that research copy and lack an
  independent Recompiled anchor.
- **Windows is the required initial desktop platform**; Linux and macOS are stretch goals.
- **No release until the entire agreed project is complete.** Verified subsets are internal QA
  milestones, never incremental releases.
- **Standard mode** supports direct target-native editing with no Projects. **Developer Mode** and its canonical Projects, target reconciliation and immutable Builds are deferred final stretch work.
- Database, save and asset edits share **one** pending-changes review and apply as **one**
  coordinated transaction with complete recovery.
- Required saves: roster, Franchise, Season and identifiable settings. Source schedule workflows are
  required; live mode state (standings, accumulated records, finances) is out of scope.

The full statements, their acceptance criteria and everything else live in the completion roadmap
and the question brief. Do not restate them here, and do not treat silence there as a decision.

## Hard rules

For NHL mesh/head conversion work, start with the saved
[shared-UV template transfer workflow](docs/MESH_UV_TEMPLATE_TRANSFER.md).
It is the preferred correspondence strategy for compatible templates across the
user's NHL10-24 workflow; the Mogilny-to-NHL19 fixture has user-reported in-game
success. Validate each template and native adapter rather than assuming universal
format or rig compatibility. Keep donor UVs/IDs; use UVs to map source coordinates.

**Headmesh conversion is an external workstream (direction update 2026-09-13).** Game-to-game
headmesh conversion is being completed in a separate repository and will be integrated natively
into Studio once complete. Until then: do **not** start new Studio-side conversion or
donor-fitting work; preserve the existing headmesh code, fixtures, scripts and receipts; keep
native integration *and its qualification* in Studio's completion scope; and do **not** inspect
unrelated repositories. **Updated 2026-09-16:** the user explicitly authorized review of
`E:\Projects\Repositories\nhl-head-mesh-converter`; conversion implementation belongs solely to
that separate stream until completion and reviewed Studio integration. Shared asset
infrastructure that does not depend on it continues. Headmesh-specific evidence gaps stay
explicit blockers; never fabricate identities or promote coverage to make a milestone look
complete. Details: completion roadmap, C5 direction update.

1. **Clean-room repository — no outside influence.** Do **not** read, open, search, clone or copy
   from any other tool repository on this machine — the prior `nhl-database-studio` app,
   `E:\Tools\rexglue-sdk`, or anything else — **unless the user explicitly points you at it for that
   specific instance**. The only prior-app material permitted here is what the user deliberately
   placed in this repo (e.g. `docs/rebuild-design.md`, `docs/MESH_IMPORT_EXCLUSIVE_PATH.md`).
   **Recorded exception:** the supplied `Nhl19BigStudio-Tester/` directory is explicitly authorized
   reference material for the C3a Ignite archive work (completion roadmap section 2). That
   authorization is specific to it and relaxes nothing else.
2. **EA data policy.** Vanilla EA data that *ships with the tool* — the `anchors/` databases and
   save fixtures — **is committed**, as plain git binaries (no LFS; all files are small). Outside
   `anchors/`, keep stray/real game files and private working data out of the repo; use synthetic
   fixtures for tests.
3. **Format facts must be verified, not guessed.** TDB has real per-year/per-platform nuances. Do
   not assert binary-layout specifics you have not confirmed against material in this repo or real
   (locally held, un-committed) data. Mark unverified specifics clearly.
4. **Version- and platform-aware by default.** Anything touching game data must account for the
   title and platform it targets. Never assume a year's layout matches its neighbour's.
5. **Two audiences.** Keep the common path simple; put depth behind progressive disclosure.
6. **Human-readable surfaces.** Anything shown to the end user — findings, errors, labels,
   summaries, tooltips — must use the **localized / resolved reference**, never the internal TDB
   convention. Resolve studio table/field ids to their metadata `name`, and numeric entity ids to
   real names (team, player, …). Keep raw ids only behind progressive disclosure, never as the
   primary text. Do the resolution at the layer that has the name data (studio-domain / API), not by
   string-parsing in the frontend.

## Evidence rules

- **Documentation never promotes support.** Scope and implementation status are separate things.
  Editing a document, a matrix cell or a capability CSV does not make a target supported.
- Use these levels precisely, and say which one you mean:
  - **implemented** — the code exists;
  - **automated-tested** — a named automated case actually executed and asserted it;
  - **game-verified** — a manual run on the exact title/platform/build with recorded evidence.
    **No cell anywhere is game-verified today.**
  - **blocked** — implementation or evidence is blocked, with the blocker recorded;
  - **missing-fixture** — no fixture is available. A test that returns early on an absent fixture is
    a skip, not a pass.
- Rust/`cargo test` totals describe baseline health. They are **not** target qualification and must
  never be substituted for it.
- Manual evidence is invalidated by changes to the relevant parser, schema/metadata, codec, edit
  path, validation, conversion recipe or package writer. Keep the old record, mark it stale, rerun.
- **Never** run the QA generators (`qa-inventory` and friends) or hand-edit the
  `docs/audits/data/**/capability.csv` tree as a side effect of other work. How generated and manual
  evidence merge deterministically is owned by milestone C0.4.

## Generated code — do not hand-edit

`app/src/bindings/*.ts` are **generated** from the Rust DTOs via `ts-rs`. Change the Rust DTO and
regenerate; never hand-edit the `.ts`. Ordinary `cargo test` **does not write** them — it only
checks them for drift. The authoritative export-root list, the comparison contract and the drift
check live in `crates/studio-api/src/bindings.rs`.

```sh
# Verify. Renders to a temp dir, compares, reports sorted missing/changed/obsolete, exits
# nonzero on drift. Repairs nothing.
cargo run -p studio-api --bin export-bindings -- --check

# Install. The only deliberate write path for app/src/bindings.
cargo run -p studio-api --bin export-bindings -- --write
```

Running the binary with no mode prints usage and exits nonzero; it never silently writes. `--dir
<path>` points either mode at another directory. The same check runs as the `studio-api` test
`bindings::tests::live_bindings_match_the_dto_contract`, which asserts the binding tree is
unchanged in **both** content and membership.

**Comparison and ownership rules.** Only CRLF is normalised to LF before comparing (the repository
has `core.autocrlf=true`, so a fresh checkout yields CRLF); nothing else is normalised, so real
whitespace or content differences are never hidden. A `.ts` file is **generator-owned** only if it
starts with the ts-rs banner; obsolete owned files are removed by `--write`, foreign files are
always preserved, and a rendered name occupied by a non-banner file makes `--write` refuse before
writing anything. The check validates the **working tree** against the current Rust DTOs; it never
compares against HEAD.

**Failure handling.** Render failure, ownership refusal and preflight rejection all happen before
any destination file is touched. An I/O failure part-way through `--write` triggers a best-effort
rollback covering both the operations that already succeeded **and** the failing one, which may
have partially mutated its target. If that rollback also fails, the recovery directory is retained
and the error names the uncertain files and where their prior bytes live.

**No crash/power-loss atomicity and no protection against a concurrent external writer are
claimed**, and **re-running `--write` does not necessarily repair a damaged tree**: a truncated
file can have lost its ts-rs banner, so a later `--write` correctly refuses it as unowned instead
of overwriting it. After a retained-recovery failure, copy the named files back from the recovery
directory by hand, then re-run `--write`. Exit codes: 0 ok, 2 refused before mutation, 3 failed
then rolled back, 4 failed and recovery also failed.

**CI.** The existing `cargo test --all` step in `.github/workflows/ci.yml` will execute the drift
test once this implementation is committed — no workflow change is needed for that. A separate CI
step invoking the CLI is **not** wired, and CI still runs on `ubuntu-latest` only (Windows CI is
C0.2 R6, open).

The gate covers content and membership, not history: as of this writing 10 binding paths under
`app/src/bindings/` are still modified or untracked relative to HEAD. See
[docs/validation/c0-dt1-dto-drift-gate.md](docs/validation/c0-dt1-dto-drift-gate.md).

## Current development track (direction update 2026-09-16)

- **Studio feature completion is the primary track**, under the
  [parallel completion plan](docs/plans/parallel-completion-plan-2026-09-16.md).
  One developer owns Studio code, one owns the external universal head converter, and a separate
  Markdown-only research partner works with the user on game knowledge. Codex triages and reviews.
  Consume established locations and findings; populate adapters alongside the workflows that need
  them. Broad asset discovery and completion of every registry entry are not blanket prerequisites
  for independent implementation. Send specific unresolved questions to research.
- **Evidence-infrastructure activation is parked** until the feature-complete initial RC QA
  transition (C9). The emitter integration and fixture-freshness verification are committed
  (`52a12fd`) and ordinary tests, fixture identities and development receipts continue; live
  migration, publication, consumer transition, vocabulary/variant freeze, storage/retention and the
  CI gate stay open tasks at that gate — do not mark them complete and do not start further
  evidence-hardening packages unless a new defect materially blocks development.
- **Asset registration describes source and destination behavior**: identity, storage/archive location, formats and constraints, references/dependencies, conversion, installation and validation. Populate what the active direct-editing workflow needs. Canonical Project creation from external mods is deferred with Projects/Builds; do not restart it as a prerequisite. Existing native import/export and provenance requirements remain.

## Working with the architect (work packages and receipts)

Follow the [coordinator/developer working agreement](docs/plans/coordinator-developer-working-agreement.md). Receipts begin with a short plain-language summary before technical detail. Every package inventories temporary files/builds/processes; obsolete package-owned scratch is cleaned before advancing beyond its related work, with durable evidence retained and cleanup verified. Codex's initial review report begins with the same-session/new-session recommendation, then the next Claude prompt, then a truncated roadmap status.

- **Codex is the architect** and owns architecture, scope and acceptance. This assistant works as
  **developer**.
- Work arrives as a numbered package with an objective, tasks, limits and a receipt budget. Do the
  package, stay inside its limits and do not broaden scope. If accepted documents genuinely conflict
  about a feature, report the specific conflict and continue the unrelated corrections.
- Every package ends with a short **receipt**: package and result, files changed, key decisions or
  conflicts resolved, checks run, unresolved questions, and the commit SHA or the blocker — then
  stop for review.
- **Codex explicitly authorizes each commit and each push.** Never commit or push anything that has
  not been reviewed and named for commit. Before committing: verify the index is empty, stage only
  the named paths, inspect `git diff --cached`, run `git diff --cached --check`, and use a message
  file (`git commit -F`) so shell quoting cannot mangle the message.
- Durable package records live in [docs/validation/](docs/validation/).

## Conventions

- Keep documentation in `docs/` current when scope or architecture changes.
- Conventional-style commit subjects; end commit messages with the required `Co-Authored-By` line.
- Branch off `main`; commit and push only when authorized (see above).
- **Private-fixture tests.** `tdb-rx2` cases that need a real EA asset are `#[ignore]`d and read
  their fixture from `NHL_STUDIO_FIXTURES`, a directory outside the repository. Nothing falls back
  to another repository or scans a mounted game tree, and a selected case with a missing, unreadable
  or empty fixture **fails** rather than returning early. Layout and per-test mapping:
  [docs/validation/c0-fx1-explicit-fixture-tests.md](docs/validation/c0-fx1-explicit-fixture-tests.md).

  ```sh
  cargo test -p tdb-rx2                                              # default: synthetic cases only
  NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-rx2 -- --ignored      # fixture-dependent cases
  NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-rx2 -- --include-ignored
  ```

- **Known baseline defects (C0.2, still open):** `cargo fmt --all -- --check` and
  `cargo clippy … -D warnings` fail repo-wide **at HEAD** — the C0-INT1 repairs that fix them live
  only in the working tree, and cannot be committed standalone (see C0-INT1 section 7.2); several
  tests still address fixtures by hardcoded absolute paths and read mounted private game data. These are recorded, not fixed — do not repair them
  opportunistically inside an unrelated package. See
  [docs/validation/c0-02-baseline-execution.md](docs/validation/c0-02-baseline-execution.md). The
  `tdb-rx2` share of the silent missing-fixture skips is closed by C0-FX1 above; the remaining
  mounted-game-tree skips are listed in that record's section 5.4.

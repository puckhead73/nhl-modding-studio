# C0.1 — Repository baseline inventory

> **Package:** C0.1 (roadmap [C0](../plans/project-completion-roadmap.md) item 1, part of items 2–3).
> **Revision:** C0.1-R1, 2026-09-07 — corrections to the build-input claims, the `cargo metadata`
> finding, the snapshot method, the check side-effect classification, and the blocker split.
> Corrections applied per the architect's review; the original inventory is unchanged in substance.
> **Type:** inspection and documentation only. No source, data, generated artifact, game file or
> QA status was changed by this package. The only intentional change is this file.
> **Inspection date:** 2026-09-07 (UTC).
> **Author role:** developer. Reviewer: Codex (architect).

Conventions used below: **Observed** = read directly from the repository at the stated commit.
**Assumption** = an inference not yet proven by evidence in-repo. **Proposed** = an action for a
later package. Nothing here promotes an implementation or QA status.

---

## 1. Baseline identity

| Item | Value |
|---|---|
| Repository | `E:\Repositories\nhl-modding-studio` |
| Branch | `feat/phase5-asset-tooling` |
| HEAD | `01c8444ed464c27a850a9198aab94826869d5717` |
| HEAD subject | `fix(assets): always texture the catalog preview via source fallback` |
| HEAD author / date | msuckow73, 2026-08-09 03:23:19 -0600 |
| Main branch | `main` (HEAD branch is ahead; not merged) |
| Working tree | **not clean** — 60 tracked modified files, 47 untracked entries |
| Tracked diff volume | +4,258 / −852 lines across 60 files |
| Tracked files at HEAD | 1,128 |
| `.git` size | 581 MB |
| Toolchain observed | cargo/rustc 1.96.0 (2026-05-25), node v24.16.0, npm 11.13.0 |

**The baseline under test is the current working tree**, not HEAD. HEAD is a commit that predates
the uncommitted feature work inventoried below; it is a provenance reference point, not the subject
of this baseline. Whether HEAD alone builds was **not tested** and is not claimed either way here.

**Changes attributable to this package:** `docs/validation/c0-01-baseline-inventory.md` (new file)
and the new `docs/validation/` directory. Everything else listed in section 2 pre-existed this
package and was left untouched. No `git add`, `commit`, `checkout`, `restore`, `clean`, `stash` or
`reset`; no regenerator, no dependency install, no game-file access. Nothing was staged.

**Read-only verification performed:** `cargo metadata --locked --offline --no-deps` (exit 0) — see
section 3.1 for exactly what that does and does not establish.

---

## 2. Working-tree inventory by workstream

### 2.1 Tracked modifications (60 files)

| Workstream | Paths | Volume | Classification |
|---|---|---|---|
| Workspace build wiring | `Cargo.toml`, `Cargo.lock` | +43 | **Build-critical.** `Cargo.toml` adds `crates/tdb-headmesh` and `crates/blender-job` to `[workspace] members`; both crate directories are **untracked**. |
| Rust service layer | `crates/studio-api/src/{commands,convert,dto,lib,registry}.rs` | +1,023 / −11 | Production source |
| Rust domain | `crates/studio-domain/src/{assets,attributes,discovery,equipment_piece,formula,kit_preview,lib,session,team_tabs,view,workspace}.rs` | +848 / −93 (incl. tests below) | Production source. `lib.rs` declares `pub mod legends;` and `pub mod player_manifest;` — both module files are **untracked**. |
| Rust domain tests | `crates/studio-domain/tests/{import_batch,import_history_cost,import_lower,import_preview,ratings_write_codec,surfaces}.rs` | (in the above) | Test source |
| Codecs | `crates/tdb-rx2/src/{bc45,gnf,mcd,pack,playermesh,texlib}.rs` | +299 / −49 | Production source |
| Edit engine | `crates/tdb-edit/src/txn.rs` | +156 | Production source |
| Historical store | `crates/tdb-fhm-store/src/lib.rs` | +2 | Declares `pub mod legends;` — module file **untracked**. |
| Frontend | 22 files under `app/src/` (`AppShell.tsx`, `routes.tsx`, `nav.ts`, `lib/{breadcrumbs,client}`, 7 components, 8 pages) | +1,834 / −665 | Production source. `AppShell.tsx` imports 6 SVGs from the **untracked** `app/src/assets/ui/tabs/`; `routes.tsx`/`nav.ts` reference **untracked** Legends pages. |
| Generated DTO | `app/src/bindings/PlayerAttributesDto.ts` | small | **Generated** by `studio-api`'s `bindings::export_typescript` test (section 4.3). |
| Planning/evidence docs | `docs/ROADMAP.md`, `docs/audits/README.md`, `docs/plans/{full-game-qa-editability,phase-6-asset-tools,phase-6.1-portraits-logos-spikes}.md` | +48 / −34 | Documentation. Adds completion-roadmap pointers and Q04 / Q27–Q28 scope notes. |
| Manual asset evidence | `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | +1 line | **Hand-maintained evidence** (a `heads` row). Not produced by `qa-inventory`. |

### 2.2 Untracked entries (47), classified

**A. Production Rust source — build-critical.**

| Path | Size | Note |
|---|---|---|
| `crates/tdb-headmesh/` | 41 files, ~10.5k LOC | Workspace member per the modified `Cargo.toml`. Library + `headmesh` bin + 12 examples + 3 tests. |
| `crates/blender-job/` | 5 files, ~533 LOC | Workspace member. Includes `scripts/wrap_head.py`. |
| `crates/studio-domain/src/legends.rs` | 441 LOC | Declared by the modified `lib.rs`. |
| `crates/studio-domain/src/player_manifest.rs` | 237 LOC | Declared by the modified `lib.rs`. |
| `crates/tdb-fhm-store/src/legends.rs` | 508 LOC | Declared by the modified `lib.rs`. |

**B. Rust test source.**

| Path | Note |
|---|---|
| `crates/studio-domain/tests/overall_formula.rs` (169 LOC) | Substantive measured regression test over anchors. |
| `crates/studio-domain/tests/zz_diag.rs` (78 LOC) | Header reads "Scratch diagnostic (temporary)". **Included in the snapshot** for baseline fidelity (architect decision, section 6). Its eventual commit disposition is a separate later question, not a C0.2 input. Not deleted, not moved. |

**C. Frontend production source — build-critical.**

| Path | Note |
|---|---|
| `app/src/assets/ui/tabs/` (7 SVGs) | 6 are imported by the modified `AppShell.tsx`. `builds.svg` has no importer located; it is source and is **included** in the snapshot regardless — an absent importer is not a reason to discard source. |
| `app/src/components/{HistoricalPlayerHeader,HistoricalTeamHeader,HistoricalTeamViews,LegendsPendingBanner,LegendsTeamHeader}.tsx` | Each has at least one importer in the tree. |
| `app/src/lib/legendsKey.ts`, `app/src/lib/sidebarCollapse.tsx` | 2 importers each. |
| `app/src/pages/{LegendsPicks,LegendsTeamDetail}.tsx` | Routed from the modified `routes.tsx`. |
| Untracked `app/src/**/*.ts(x)` total | 1,555 LOC |

**D. Generated DTO bindings — build-relevant, reproducible in principle.**

`app/src/bindings/{CloneJerseyResultDto, CloneKitTexturesSummaryDto, ClonedKitFileDto,
JerseyThumbnailDto, LegendsComputeSummaryDto, LegendsMemberDto, LegendsStatusDto,
LegendsTeamSummaryDto, PlayerManifestSummaryDto}.ts` — 9 files. Emitted by `studio-api`'s
`bindings::export_typescript` test (section 4.3) and imported by frontend source. Their **current
working-tree bytes are part of the baseline** and are copied as-is; regeneration is a separate,
isolated step (section 6).

**E. Definitions / resources — build- and test-critical.**

`defs/appearance/` — 11 untracked TOMLs (1.0–26.3 KB each): `bone-overrides`, `cc5-bone-map`,
`facial-hair-donors`, `facial-regions`, `head-ids`, `head-uv-classes`, `headmesh-matrix`,
`legacy-bone-names`, `morph-donors`, `nhl24-facemesh-overrides`, `rig-audit-scoring`. Only
`defs/appearance/orphan-heads.toml` is tracked. Code in `tdb-headmesh` (`cc5`, `convert`, `facial`,
`matrix`, `morph`, `palette`, `qa`, `remap`, `uvfp`, the `headmesh` bin) and its three tests read
these by path at runtime/test time.

**F. Planning and evidence documents.**

`docs/STATE-OF-THE-REPO.md`, `docs/plans/project-completion-roadmap.md`,
`docs/plans/completion-design-questions.md`, `docs/audits/{headmesh-uv-compat,
legacy-head-donor-census, nhl21-ps4-head-asset-binding, nhl24-legacy-patha-roundtrip}.md`.
These are the canonical current planning inputs; three of the audits are the completion roadmap's
own cited evidence. **Included** in the snapshot.

**G. Authorized external reference material — excluded from the snapshot, preserved in place.**

`Nhl19BigStudio-Tester/` (498 MB, self-contained .NET bundle: `Nhl19BigStudio.Gui.exe` + native
DLLs + `README.txt` + `references/PS4 GNF Studio`, 364 MB) and `Nhl19BigStudio-Tester.zip` (216 MB).
Reviewed and hashed in the completion roadmap (exe SHA-256 `0108399F…04717E`). Third-party
binaries; no build input depends on them. Excluded from the snapshot; **left untouched on disk**.
Whether to retain both copies long-term is a later question, not a C0.2 input.

**H. Private / user working data — excluded from the snapshot, preserved in place.**

`corpus/` (untracked as a directory; its `*.db` members are additionally `.gitignore`d):
`historical.db` 902 MB, `legends_cache.db` 1.3 GB, `ea_import_ledger.db` 24 KB, plus
`ea_attribute_maxima.json`, `ea_formulas.json`, `ea_formula_state.json`, `ea_formula_profiles/`.
The JSON files and profile directory are **user-authored working state**, not derived from anchors,
and are *not* covered by any `.gitignore` rule — they are simply untracked. Excluded from the
snapshot per the architect's decision (section 6); whether any of it is a shippable default is
deferred and does not gate C0.2.

**I. Ignored build artifacts / caches (not part of the 47; listed for completeness).**

`target/`, `src-tauri/gen/`, `node_modules/`, `app/node_modules/`, `app/dist/`, `Staging/`.
`Staging/` additionally holds unverified `defs` material and `nhlviewng-validation-audit.md`.

### 2.3 Uncertain classifications (flagged; none block C0.2)

- `app/src/assets/ui/tabs/builds.svg` — no importer located; purpose unconfirmed. Included anyway.
- `crates/studio-domain/tests/zz_diag.rs` — self-labelled temporary; commit intent unknown.
  Included anyway.
- `corpus/ea_formula_profiles/` and `corpus/ea_*.json` — working state versus shippable defaults is
  undetermined. Excluded from the snapshot; left alone on disk.
- `Nhl19BigStudio-Tester.zip` versus the extracted directory — retention question, deferred.

---

## 3. Build wiring and build-input gaps

### 3.1 Workspace resolution — what was and was not established

**Observed:** `cargo metadata --locked --offline --no-deps` exited 0 and enumerated **21 workspace
members** — the 19 previously documented crates plus `tdb-headmesh` and `blender-job`, with
`src-tauri` (package `nhl-studio`). This confirms the workspace manifests parse and that the two new
member paths resolve to real manifests in the current working tree.

**Not established by that command:**

- complete dependency-graph resolution (`--no-deps` skips it);
- that the local registry cache holds every dependency needed to compile;
- that `Cargo.lock` is sufficient for a build, only that it satisfied `--locked` for this
  metadata query;
- that any offline build or test run would succeed.

Whether the workspace compiles offline, or at all, is a C0.2 execution question.

**On `Cargo.lock` state:** `git status` still reported ` M` for `Cargo.lock` after the command. That
is evidence the file remained modified relative to HEAD; it is **not** proof the file's bytes are
unchanged from before the command. C0.2's before/after hashing (section 5) is what establishes byte
stability, and this report makes no byte-level claim about `Cargo.lock`.

### 3.2 Build-input gaps

The demonstrated problem is **partial copying**: taking the modified tracked manifests, module
declarations and frontend files without the untracked files they now require. Each row below is a
dependency created by the current uncommitted work; none of them is a statement about whether HEAD
builds on its own, which was not tested.

| # | Gap | Effect on a partial copy |
|---|---|---|
| G1 | The modified `Cargo.toml` lists two member crates whose directories are **untracked** | Copying that `Cargo.toml` without `crates/tdb-headmesh/` and `crates/blender-job/` fails at manifest resolution. |
| G2 | The modified `studio-domain/src/lib.rs` and `tdb-fhm-store/src/lib.rs` declare three **untracked** modules | Copying those files without `legends.rs`, `player_manifest.rs` and `tdb-fhm-store/src/legends.rs` fails on missing module files. |
| G3 | The modified `app/src/AppShell.tsx` imports six **untracked** SVGs | Copying it without `app/src/assets/ui/tabs/` fails frontend resolution. |
| G4 | The modified `routes.tsx`/`nav.ts` and several components import **untracked** Legends/Historical modules | Same class of failure as G3. |
| G5 | Nine **untracked** generated DTO files under `app/src/bindings/` are imported by frontend source | Omitting them fails the frontend type-check unless they are regenerated first. |
| G6 | 11 **untracked** `defs/appearance/*.toml` are read by `tdb-headmesh` source and its 3 tests | Compilation is unaffected; `cargo test -p tdb-headmesh` fails (`cc5_defs_guard`, `matrix_guard`, `cc5_authored_pipeline` load them by path). |
| G7 | `corpus/historical.db` is excluded from the snapshot | Affects only `#[ignore]`d tests (`import_corpus_e2e`; `formula_real` via `FHM_CORPUS`). Not a default-run input. |
| G8 | Headless Blender is an **external runtime** | `blender-job::bundle` resolves a user-configured exe or a managed install of pinned series **4.5**; neither is in-repo, and that crate explicitly does not implement the download (milestone M3). Whether any default test requires it is unverified (*assumption:* none do). |
| G9 | Tauri prerequisites | CI installs `libwebkit2gtk-4.1-dev` and friends on Linux; a Windows run needs the equivalent WebView2/MSVC prerequisites. Not in-repo. |
| G10 | `anchors/` (662 MB, 552 files) is required by most Rust matrix tests | **Included** in the snapshot (section 5), copied as bytes like everything else. |
| G11 | `Nhl19BigStudio-Tester*` totals 714 MB | Excluded; no build input depends on it. |

Per the architect's decision (section 6), any missing dependency or prerequisite discovered during
C0.2 is recorded as an **explicit baseline blocker**, not worked around silently.

**Not a gap:** `app/package-lock.json` and the root `package-lock.json` are both tracked, so
`npm ci` is reproducible given network access.

### 3.3 Line endings — recorded, not normalized

**Observed:** `core.autocrlf=true`, and `.gitattributes` sets only `-text` on anchor binaries; there
is no `* text=auto`. This means git's blob content and the working-tree content of tracked text
files can differ, and content produced by `git archive`/`git checkout` is not guaranteed to match
the bytes currently on disk.

This report makes **no claim** that all working-tree text files are CRLF, nor that a git worktree
guarantees LF. The consequence for C0.2 is simply that the snapshot must take **the exact
working-tree bytes** and perform no newline conversion in either direction (section 5). The
working-tree bytes are this baseline's representation; git-derived content is not a substitute for
them.

---

## 4. Check commands — located, inspected, side effects

All Rust commands run from the repo root; frontend commands run from the `app` subdirectory. In the
snapshot these become the snapshot root and its `app` subdirectory. **None were executed in this
package, and none may be executed against the live source tree** (section 6).

### 4.1 Side-effect classification — verified versus assumed

Side effects were classified by reading the commands and the code they invoke. The distinction below
is deliberate: "no repository writes" is a claim that requires evidence, and for several commands
that evidence does not exist yet.

**Verified writes outside `target/` and `node_modules/`:**

- `src-tauri/build.rs` calls `tauri_build::build()`, which emits into **`src-tauri/gen/schemas/`** —
  inside the source tree, not `target/`. That directory exists and is `.gitignore`d. So the claim
  "cargo only writes into `target/`" is false for this workspace as a general statement.
- `studio-api`'s `bindings::export_typescript` test writes into **`app/src/bindings/`** — tracked
  source (section 4.3).
- The `docs/audits/data/*-matrix.csv` tests write only when `QA_WRITE` is set; otherwise they read
  and assert.

**Assumed, not verified:**

- that no other build script, proc macro, test or dependency writes into the source tree;
- that `vite build` writes only `app/dist` (the config sets `outDir: "dist"`, but plugins and
  `tsc` incremental state were not audited);
- that `npm ci`/`npm test` trigger no lifecycle scripts with side effects — the root and `app`
  `package.json` files declare no `pre*`/`post*`/`prepare` scripts (**observed**), but installed
  dependencies may run their own install scripts (**not audited**).

**Required of C0.2:** inspect the relevant hooks (cargo build scripts across the dependency graph as
far as practical, npm lifecycle and dependency install scripts, any git hooks — `.git/hooks` holds
only samples and there is no `.husky/`, both **observed**), and capture a **before/after file
manifest of the entire snapshot directory** around every command run. Unexpected writes are findings
to record, not noise to filter.

### 4.2 The command set

| Purpose | Command | Working dir | Notes |
|---|---|---|---|
| Rust formatting | `cargo fmt --all -- --check` | root | Read-only with `--check`. CI-equivalent. Needs the untracked crates present (G1). |
| Rust lint | `cargo clippy --all-targets -- -D warnings` | root | Compiles; triggers `src-tauri/build.rs` and therefore `src-tauri/gen/`. |
| Frontend deps | `npm ci` | `app` | **Needs network.** Writes `app/node_modules`; dependency install scripts unaudited. |
| Frontend type-check + build | `npm run build` (`tsc && vite build`) | `app` | Writes `app/dist`; other writes unverified. Needs G3–G5 satisfied. |
| Frontend tests | `npm test` (`vitest run`) | `app` | 4 test files under `app/src/lib/`. **Not run by CI today.** |
| QA drift, targeted | `cargo test -p studio-domain --test mutation_matrix` / `--test operation_matrix` / `--test save_matrix` / `--test surface_matrix` | root | Each asserts its CSV under `docs/audits/data/` matches regenerated content; writes only when `QA_WRITE` is set. |
| Headmesh evidence drift | `cargo test -p tdb-headmesh --test matrix_guard` | root | Cross-checks `defs/appearance/headmesh-matrix.toml` against `docs/audits/data/assets/<title>/<platform>/capability.csv`, including `game_verified` in one ⇔ `game_verified == pass` in the other. Needs G6. |
| Full Rust suite | `cargo test --all` | root | CI-equivalent, and **also the DTO generator** (section 4.3). Run in isolation only. |
| Workspace resolution | `cargo metadata --locked --offline --no-deps` | root | The one command run for this report; scope limits in section 3.1. |

### 4.3 Explicit regenerators — not part of a baseline run

| Purpose | Command | Writes |
|---|---|---|
| QA inventory | `cargo run -p tdb-qa --bin qa-inventory` | Rewrites `docs/audits/data/anchors-manifest.csv`, `schema-diff.csv`, `game-qa-matrix.csv`, and **deletes and rebuilds** `docs/audits/data/schema/`. |
| QA matrix refresh | `QA_WRITE=1 cargo test -p studio-domain --test <matrix>` | Rewrites the corresponding `docs/audits/data/*-matrix.csv`. |
| Headless edit QA | `cargo run -p studio-api --bin edit-qa` | Deposits edited artifacts and markdown reports (*assumption:* into an output directory — the exact target was not inspected). |
| Corpus build | `cargo run -p tdb-fhm-build -- <export-root> <out.db>` | Creates the corpus DB. Requires the external, non-repo FHM CSV season-export tree. |
| Corpus convert | `cargo run -p tdb-fhm-build --bin tdb-fhm-convert -- <prior historical.db> <out.db>` | Requires a prior-app DB — private input. |

Per section 6, these run only in a **separate disposable copy** if and when authorized, with outputs
captured; they never overwrite live evidence.

### 4.4 The DTO generator has no drift check

DTO generation is **not** a separate command. It is a `#[cfg(test)]` test inside `studio-api`:
`crates/studio-api/src/lib.rs:152` (`mod bindings`) / `:156-161` (`fn export_typescript`) sets
`TS_RS_EXPORT_DIR` to `app/src/bindings` and calls `export_all()` on roughly 60 DTO roots.

**Observed consequences:**

1. `cargo test --all` — the CI command — writes into `app/src/bindings/` unconditionally. It is
   simultaneously the generator and part of the test suite. There is no `--check` mode and no
   assertion that emitted files match what is committed.
2. Running the full Rust suite therefore mutates tracked frontend source in whatever tree it runs in.
3. Because generation is unconditional and unchecked, CI cannot fail on DTO drift; it silently
   regenerates inside its own workspace.

*Assumption:* this is a plausible origin for the modified `PlayerAttributesDto.ts` and the nine
untracked DTO files, but that was not proven.

This is a real design defect. It is **recorded, not fixed**, per the package brief. The C0.2
handling is fixed by the architect's decisions in section 6: frontend checks run first against the
original bindings; the DTO export and full Rust suite run only in isolation; binding deltas are
recorded; `studio-api` is never silently dropped from coverage.

### 4.5 What CI does and does not cover

`.github/workflows/ci.yml` — job `rust` (ubuntu-latest: `cargo fmt --all -- --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --all`) and job `frontend`
(`npm ci && npm run build`, working-directory `app`).

Not covered by CI: frontend `vitest`; any DTO drift gate (section 4.4); `qa-inventory` determinism;
and any Windows job — despite **Q05 making Windows the required initial platform**. CI is
Linux-only. Adding a Windows job is a **required C0 follow-up** under the existing Q05 decision and
the roadmap's "C10 CI enforcement starts at C0" rule; it is not a new scope decision and does not
gate C0.2 execution.

---

## 5. Proposed C0.2 snapshot plan (not executed)

**Goal.** A build directory holding the **exact current bytes** of the intended source — every
tracked path plus an explicitly approved set of untracked paths — with provable provenance, no
newline conversion, no dependency on accidental local files, and no ability to reach back into or
disturb the live tree.

The snapshot is built from the **working tree**, not from git object content. Git is used only to
enumerate membership and to record the HEAD reference.

### 5.1 Membership selection

1. **Enumerate tracked paths:** `git ls-files -z` at the repo root. All of them are included,
   including `anchors/` (552 files, 662 MB) and `docs/audits/data/`.
2. **Enumerate the approved untracked paths** — the explicit allowlist, no globbing beyond it:
   - `crates/tdb-headmesh/` and `crates/blender-job/` (both new crates, in full);
   - `crates/studio-domain/src/legends.rs`, `crates/studio-domain/src/player_manifest.rs`,
     `crates/tdb-fhm-store/src/legends.rs`;
   - `crates/studio-domain/tests/overall_formula.rs` and
     `crates/studio-domain/tests/zz_diag.rs`;
   - all untracked files under `app/src/` — components, pages, `lib/`, and all 7 SVGs in
     `app/src/assets/ui/tabs/` **including `builds.svg`**;
   - the 9 untracked DTO files under `app/src/bindings/`, at their current bytes;
   - all 11 untracked `defs/appearance/*.toml`;
   - the untracked planning/evidence documents: `docs/STATE-OF-THE-REPO.md`,
     `docs/plans/project-completion-roadmap.md`,
     `docs/plans/completion-design-questions.md`, and the four new `docs/audits/*.md`.
3. **Exclusions** (never copied; **all left untouched in place**): `.git/`, `target/`,
   `**/target/`, `src-tauri/gen/`, `node_modules/`, `app/node_modules/`, `app/dist/`, `Staging/`,
   all of `corpus/`, `Nhl19BigStudio-Tester/`, `Nhl19BigStudio-Tester.zip`.
4. **No-accidental-files rule:** any untracked path not on the allowlist is excluded, and the
   manifest records every exclusion that matched a real path, so a reviewer sees what was left out
   rather than inferring it.

### 5.2 Copy rules

- Copy **raw bytes**. No newline conversion, no text/binary heuristic, no filtering, no
  re-encoding, in either direction.
- **No hardlinks, junctions, symlinks or reparse points** back to live inputs. The snapshot must be
  a fully independent set of bytes so that later execution cannot mutate live source.
- **Detect links before copying.** Scan every selected path for symlinks, junctions and other
  reparse points. Report each as an exception in the manifest; do not silently dereference or
  silently skip. Resolve the handling of any exception explicitly before proceeding.
- Copy into a directory outside the repository so no snapshot content can be mistaken for live
  source or picked up by a live-tree tool.

### 5.3 Integrity and concurrency

1. **Pre-hash:** compute SHA-256 for every selected file from the live tree, together with size and
   mtime, before copying.
2. **Copy.**
3. **Post-hash the copies:** compute SHA-256 of each file **in the snapshot** and verify it equals
   its recorded source hash. Any mismatch fails the snapshot.
4. **Re-check the live tree:** re-enumerate membership (`git ls-files` plus the allowlist) and
   re-hash the selected live files. Any change in membership or in a source hash means the tree
   moved under the copy; the snapshot is discarded and retried, never patched.
5. Assert no `.git/index.lock` and no in-progress rebase/merge before starting and after finishing.
6. All operations against the live tree are read-only. No `stash`, `checkout`, `clean`, `restore` or
   `reset` at any point.

### 5.4 Manifest and identity

- The manifest records, per file: path, size, source SHA-256, snapshot SHA-256, and link-exception
  status; plus the full allowlist and exclusion rules, the HEAD SHA, branch, timestamp, and the
  matched-but-excluded path list.
- **Snapshot identity = SHA-256 of the manifest, together with the HEAD reference.** HEAD alone does
  not identify this snapshot, because the snapshot deliberately contains uncommitted bytes that HEAD
  does not describe. All later evidence cites the manifest hash.
- **Generated manifests and logs live outside the snapshot's input set.** They are written to a
  sibling directory, never into the snapshot tree, so they cannot alter what a later run hashes or
  re-verifies.

### 5.5 Anchors verification

`anchors/` is included as ordinary copied bytes. **After every execution run inside the snapshot,
re-hash every included anchor file** and compare against the manifest — a complete verification, not
a spot check. Any anchor mutation is a critical finding.

### 5.6 What the snapshot does and does not prove

Constructing and verifying the snapshot proves **isolation and provenance**: that a known, hashed,
byte-exact set of source was assembled independently of the live tree. It proves nothing about
buildability. Claims that the baseline builds, lints, type-checks or passes tests require the
corresponding checks to have actually run inside the snapshot and to have succeeded, each reported
with its command, working directory, exit status and before/after file manifest.

---

## 6. Architect's C0.2 decisions (recorded)

These are accepted decisions for C0.2, not open questions.

1. **`zz_diag.rs` is included** in the snapshot for baseline fidelity. No commit disposition is
   decided; that is a later question.
2. **Private corpus working state is excluded** from the snapshot. Whether any of it ships is
   deferred and does not gate C0.2.
3. **Frontend checks run before DTO generation**, against the bindings as they currently exist in
   the working tree.
4. **DTO export and the full Rust test suite run only in isolation**, with binding deltas recorded
   (before/after hashes of every file under `app/src/bindings/`).
5. **If regenerated bindings change frontend results**, report the original-state and
   regenerated-state results **separately**. Do not collapse them into one verdict.
6. **`studio-api` is never silently omitted** from coverage. If it is separated for sequencing, that
   separation and its coverage are stated explicitly.
7. **QA generators, if later authorized, run only in a separate disposable copy**, with outputs
   captured. They never overwrite live evidence, and never run in the verified snapshot.
8. **C0.4 owns the evidence-merge design** (the generated-versus-manual ledger problem, D9 below).
   It is not a C0.2 prerequisite.
9. **Windows CI is a required C0 follow-up** under the existing Q05 decision, not a new scope
   decision, and does not gate C0.2.
10. **Missing dependencies or prerequisites are explicit baseline blockers** — recorded as such,
    never silently substituted or worked around.
11. **Snapshot construction alone proves isolation and provenance.** Build/test success may be
    claimed only after the checks actually run and succeed inside the snapshot.
12. **All included anchor hashes are verified after execution**, in full.

---

## 7. Documentation and instruction conflicts

Recorded, not repaired. No canonical document was rewritten by this package.

| # | Conflict | Location | Resolving evidence / accepted decision |
|---|---|---|---|
| D1 | "The application has **not been scaffolded yet** — only documentation and the repo skeleton exist." | `CLAUDE.md:29` | **Stale.** 21 workspace crates, ~82k LOC Rust, ~24k LOC TS, 115 Tauri commands, 139 commits on this branch. Superseded by `docs/STATE-OF-THE-REPO.md` sections 2–3 and by this package's brief. |
| D2 | "Pre-release scaffold … No application code has been committed yet." | `README.md:10-12` | Same as D1. Already flagged in `STATE-OF-THE-REPO.md` section 6. |
| D3 | Title scope "NHL 10 → NHL 21" | `CLAUDE.md:7,34,64`; `README.md:20,30`; `docs/DESIGN.md:30,44,119,135,233,272` | **Superseded.** Current scope is 17 catalog entries spanning NHL 07–21 plus Legacy and Legacy Recompiled (completion roadmap section 1; `full-game-qa-editability.md` scope line; 27 anchors). |
| D4 | Platforms listed as "Xbox (360/One family)" | `README.md:20` | **Superseded.** Completion roadmap section 1: "Do not imply Xbox One support from the old README's wording." No Xbox One target exists in the catalog. |
| D5 | ROADMAP Phase 0 describes scaffolding; Phases 9/10 (PS4, title expansion) described as unstarted | `docs/ROADMAP.md` | **Acknowledged in-tree:** the uncommitted `ROADMAP.md` edit adds a 2026-09-05 banner marking the original phases historical and pointing at the completion roadmap. |
| D6 | Compatibility matrix shows every cell partial, described as inherited reference-app confidence | `docs/01-scope-and-compatibility.md:39` and its matrix | **Unresolved.** The QA ledger now carries 80 `edit-roundtrip-pass` and 27 `read-pass` rows. Reconciling the matrix must not promote anything to "verified": no cell anywhere is `game-verified`. |
| D7 | Legends is a shipped navigation section with no ROADMAP slot | `docs/ROADMAP.md` versus `app/src/pages/Legends*`, `crates/studio-domain/src/legends.rs` | **Unresolved in ROADMAP.** The completion roadmap section 2 does carry a Legends workstream row. |
| D8 | Arena assets in QA scope versus the Q27–Q28 stretch decision | `docs/plans/full-game-qa-editability.md` asset-family list | **Resolved by the uncommitted edit** to that file, which removes arenas from the asset families and records the stretch disposition. |
| D9 | `qa-inventory` regenerates `game-qa-matrix.csv` wholesale and can never assign `game-verified`, while manual asset evidence lives in a *different*, hand-maintained tree (`docs/audits/data/assets/**/capability.csv`) | `crates/tdb-qa/src/bin/qa_inventory.rs`; `crates/tdb-qa/src/matrix.rs:1-23` | **Structural; owned by C0.4** per section 6. Manual sign-off currently has nowhere to live in the generated ledger; the only manual channel is the asset CSV tree, and `tdb-headmesh`'s `matrix_guard` is the only thing binding the two. |
| D10 | `docs/audits/qa-ledger.md:10` — "no numbers are hand-maintained" | versus the hand-maintained `capability.csv` tree in D9 | **Narrow-scope statement.** True of the four `qa-inventory` artifacts; misleading as a repo-wide claim. Correct it as part of C0.4. |
| D11 | Q05 requires Windows as the initial platform; CI is Linux-only | `docs/plans/completion-design-questions.md` Q05 versus `.github/workflows/ci.yml` | **Required C0 follow-up** under the existing decision (section 6, item 9). Not a new scope question. |
| D12 | CLAUDE.md clean-room rule versus the authorized tester directory | `CLAUDE.md` hard rule 1 versus `Nhl19BigStudio-Tester/` and completion roadmap section 2 | **Resolved by explicit user authorization**, recorded in the roadmap's tester-review section. Not a general relaxation of rule 1. |

---

## 8. Blockers

### 8.1 C0.2 blockers (must be resolved to execute the baseline)

None outstanding from this package. Every disposition C0.2 needs — snapshot membership, byte-exact
copying, corpus exclusion, `zz_diag.rs` inclusion, check sequencing, DTO handling, generator
handling, anchor verification — is settled by section 6.

Blockers may still be **discovered** during C0.2, and are recorded as such: missing prerequisites
(Windows WebView2/MSVC toolchain, network for `npm ci` and any cold crates.io fetch, absent Blender
4.5 for G8), and any check that cannot run in the snapshot.

### 8.2 Later questions (explicitly not C0.2 prerequisites)

- Commit disposition of `zz_diag.rs` and of the untracked work generally.
- Whether to retain both the extracted tester directory and its zip.
- Whether any `corpus/` JSON or profile content is a shippable default.
- The evidence-merge design for the generated-versus-manual QA ledger (D9) — **owned by C0.4**.
- Reconciling the compatibility matrix (D6) and the ROADMAP Legends gap (D7).
- Correcting the stale scope statements in `README.md`, `CLAUDE.md` and `docs/DESIGN.md` (D1–D4).

None of these requires a user decision before the baseline runs.

---

## 9. What was not done in this package

No commit, push, staging, reset, cleanup, source repair, dependency install, generated-file refresh,
game-file access or QA-status promotion. No snapshot was created. No baseline check was executed. No
defect found here was fixed. No canonical document was rewritten. Section 4 lists commands that were
*located and inspected*; the only command executed against this repository was
`cargo metadata --locked --offline --no-deps`, plus read-only inspection of manifests, build
scripts, lifecycle scripts and hook directories.

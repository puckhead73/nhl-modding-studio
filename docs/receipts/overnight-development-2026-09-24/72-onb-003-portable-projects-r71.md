# 72 — ONB-003: flexible inputs, durable portable projects, Open existing project, mod packages and the built-in installer (candidate r71)

**Summary.** A project no longer needs a game installation to exist. It can start from a roster save alone,
a main database alone, a folder of extracted assets alone, any combination, or the bundled vanilla files,
and the game and save destinations may stay empty until a deployment needs them (they are linked later
from Settings). Everything a project knows is saved in its own folder in a versioned, documented form:
which inputs it was made from, which files were edited and what the edits mean, the editable source
images behind texture and nameplate outputs, which view was open last. **Open existing project** is a
separate action on the first screen (with a recent list): it reopens the project with its destinations,
edited files, pending changes and last view, and says plainly when the project was moved, when a
destination is missing, when a previous save was interrupted, when it was saved by an older Studio and
has been upgraded, and refuses when the same project is already open elsewhere or was written by a newer
Studio. **Build mod package** turns a saved project into one `.nhlmod` file that carries only the edits
(cells by table / entity / field with before and after, dictionary texts by key, replaced textures with
their source image), a manifest, hashes and a declared compatibility list; never a vanilla dump, never a
private path. **Install package** (Settings → Tools, and `/install`) reads and validates a package first,
offers only the consoles the package declares supported, shows the exact planned changes and every
conflict with the recipient's own edits, stages everything in its own folder, writes only on an explicit
Install, backs up every replaced file and can restore it. Cross-platform installation of the same title is
offered only where the bundled baselines prove the databases are byte-identical; NHL Legacy and NHL
Legacy Recompiled are treated as one title, and the three Legacy runtimes were qualified independently
against owned synthetic bases. Cross-game installation is refused.

Internal candidate **r71**. r67, r68, r69 and r70 are byte-for-byte unchanged (zips and extracted trees,
§5). Nothing was committed, pushed, published, uploaded, launched or installed into a real game; no
original installation or save was opened for writing; every test wrote into a temp tree it owns, and the
anchors tree hashed the same before and after. Dispatch:
[ONB-003](../../plans/onboarding-20261001-resume-projects.md). Compatibility matrix:
[onb-003-compatibility-matrix.md](../../validation/onb-003-compatibility-matrix.md). No user check is
marked passed; nothing is game-verified.

One build defect was caught and corrected before delivery: the first assembly of r71 carried three
hashes in `BUILD-RECORD.json` with a stray leading `\` (a `sha256sum` quirk on mixed path separators);
`verify.py check` refused that archive ("names another helper"), the archive and its extraction were
discarded, the record script was corrected and r71 was assembled again. Only the second archive exists.

## 1. What changed

### 1.1 Inputs (§1)

| Item | Where |
|---|---|
| Every input optional; any one of main database, roster save or extracted assets folder is enough; the installation root may stay empty; roster-only projects take their schema metadata from the bundled baseline; the import review names what each input is and what stays gated | `crates/studio-api/src/registry/project.rs` (`project_import_review`, `project_create_import`, `project_create_fresh`, `resolve_destinations`) |
| Assets-only projects register without an edit session (`register_asset_only`); every database route answers with a named reason ("this project has no main database or roster save, so database editing is unavailable…"); the summary carries `role: "assets"` | `crates/studio-api/src/registry.rs`, `convert.rs` (`asset_only_summary_dto`) |
| Destinations linked later (`project_relink`); a deployment into an unreachable linked destination is `unresolved` ("no reachable destination is known"), never created | `registry/project.rs`, `commands.rs` |
| Onboarding: four actions (Start fresh, Use my own files, Open existing project, Install a package); optional inputs including the assets folder; the installation root optional | `app/src/pages/OpenProject.tsx`; `ProjectInputsCard.tsx` (Settings) |

### 1.2 Durable project format (§2)

| Item | Where |
|---|---|
| Manifest schema `nhlms-project/2` (v1 read and migrated losslessly; `migrated_from` recorded; other schemas refused by name; a manifest with no input refused); `inputs` (role, original path, hash, kind), `sources/` (content-addressed editable sources: texture / nameplate source images, archive-member sources), `member_edits` (archive, member, scope, source hash), `last_view`, optional `working.main_db` / `meta`, `destinations.assets_dir`, `StagedFile.source_sha256` | `crates/studio-domain/src/project_store.rs` |
| Exclusive-share lock file `nhl-studio-project.lock` for the lifetime of the binding ("open in another Studio window or process") | `project_store.rs` (`ProjectLock`) |
| Every edit route writes into the project: texture and nameplate imports keep the source bytes (`record_staged_output`), member edits record semantic `MemberEditRecord`s at save (`capture_member_source`) | `registry/texture_review.rs`, `registry/project.rs`, `registry.rs` |

### 1.3 Open existing project (§3)

| Item | Where |
|---|---|
| `project_open`: lock; migration written back; warnings for a moved project, missing destinations (deployment gated, not failed), an interrupted save (`.tdb-txn` journal / `ACTIVE` marker reported, never resumed silently), undo history not persisted; `last_view` returned and restored only for `/app/…` routes | `registry/project.rs`, `commands.rs` (`project_open`, `project_set_view`, `project_relink`) |
| Recent projects remembered in settings (`recentProjects`); the resume step lists every warning before the project is entered | `app/src/lib/settings.tsx`, `OpenProject.tsx`, `AppShell.tsx` (debounced last-view recording) |

### 1.4 Mod package (§4)

| Item | Where |
|---|---|
| `.nhlmod` container: own deterministic stored ZIP writer / reader with CRC-32; `manifest.json` (`nhlms-modpkg/1`), `edits.json` (`nhlms-modpkg-edits/1`), `payloads/<sha256>.<ext>`; `package_id` = SHA-256 of the manifest without its id; hostile-input validation (stored entries only; no zip64, no encryption; entry-name rules; case-insensitive collisions; size and count limits; manifest ↔ entries consistency; CRC) | `crates/studio-domain/src/mod_package.rs` |
| Inventory and build: categories `cells` / `dictionary` / `textures` (portable) and `members` / `native` / `structural` (blockers); cells from `pending_changes(baseline, working)`, dictionary texts from a positional row diff, textures from the staged output plus kept source; a build refuses a dirty project, a non-empty category neither included nor excluded, an included blocked category, an unsupported declared platform, an existing output path; temp file then rename | `crates/studio-api/src/registry/package.rs` (`package_inventory`, `package_build`, `package_inspect`) |
| Per-platform support: cells on another platform of the same title only when the bundled baseline main database is byte-identical (`baselines/INVENTORY.json`); dictionary when that platform has a bundled dictionary; textures author-platform only; `same_title(nhllegacy, legacyrecomp)` | `package.rs` (`target_support`) |
| Build panel on Pending Edits (name, version, description, include / exclude per category, platforms, output) | `app/src/components/BuildPackagePanel.tsx`, `PendingEdits.tsx` |

### 1.5 Installer (§5–6)

| Item | Where |
|---|---|
| `package_preflight`: validate the package, resolve the destination (`detect_install` on the chosen root / save path), stage recipient copies in an installer-owned folder, replay cells through `Workspace::set_field` with before-value checks (differing recipient value = conflict, blocked unless explicitly allowed; already applied = skipped), dictionary through `apply_retext` after checking the current text, textures through `plan_texture_replace` / `replace_texture_in_file`; plan with requirements, items, byte counts, conflicts, reasons | `registry/package.rs` |
| `package_apply`: backups under `<app data>/installs/<seq>/backups`, publish through the shared `in_place_save` two-phase publisher, `nhlms-install/1` record; `package_installs`, `package_restore` (refuses when the destination changed since the install) | `package.rs`, `commands.rs` (`resolve_install_records`) |
| Installer page: read → choose console (unsupported consoles disabled) → destination → read-only review → Install → result; conflicts need an explicit allowance and a fresh review | `app/src/pages/InstallPackage.tsx`, `routes.tsx` (`/install`), `Settings.tsx` |

### 1.6 Commands, bindings, docs

New commands `project_set_view`, `project_relink`, `package_inventory`, `package_build`, `package_inspect`,
`package_preflight`, `package_apply`, `package_installs`, `package_restore` (all off the main thread,
asserted by `command_execution_context`). `project_import_review` / `project_create_import` take the
baseline store. DTOs: `ImportSourcesDto` (all optional, `assets_dir`), `ProjectInputDto`,
`ProjectRelinkDto`, `ProjectInfoDto` (`mode`, `inputs`, `last_view`, `migrated_from`, `schema`,
`sources`, `member_edits`), `ProjectOpenDto` (`warnings`, `last_view`), and the package / installer DTOs
(`PackageInventoryDto`, `PackageBuildRequestDto`, `PackageBuildDto`, `PackageInspectDto`,
`InstallPlanDto`, `InstallApplyDto`, `InstallRecordDto` and their parts). Export roots 186 → 194; 348
binding files, in sync. Ledger row added (`LEDGER.md`); compatibility matrix written.

## 2. Checks

| Suite | Result |
|---|---|
| `studio-domain --lib` (incl. `mod_package` 3 new, `project_store` extended) | 257 / 257 (2 ignored, pre-existing) |
| `studio-api --test project_inputs` (new: roster save alone; main database alone without an installation; extracted assets alone — stage, deploy, restore, database routes gated by name; combination + fresh with no installation, relink, unreachable destination blocked) | 4 / 4 |
| `studio-api --test project_resume` (new: save → close → reopen restores destinations, edited files, pending and last view; moved project; missing destination warning and gated deployment; interrupted save reported; v1 manifest migrated, newer schema refused, concurrent open refused by the lock) | 5 / 5 |
| `studio-api --test mod_package` (new: NHL 21 PS4 database-only project → package → owned destination round trip with the project moved and then deleted; Legacy Xbox 360 author → PS3 / Recompiled / Xbox 360 owned bases incl. dictionary and roster mirror, restore identical; tampered / truncated / renamed-entry / CRC / unsafe-path packages refused; NHL 15 PS3 vs PS4 unsupported, cross-game refused) | 4 / 4 |
| `studio-api --test project_workspace` (updated signatures) | 7 / 7 |
| `studio-api --lib` (bindings drift gate, 194 roots) | 42 / 42 |
| `studio-api` all tests (`--no-fail-fast`) | 59 binaries, 180 passed, 0 failed, 53 ignored (fixture-gated, pre-existing) |
| frontend `tsc --noEmit` | clean |
| frontend vitest (all; new `InstallPackage.test.tsx` 2, `BuildPackagePanel.test.tsx` 2, `OpenProject.test.tsx` +2: Open existing with warnings and last view, optional inputs) | 291 / 291 (49 files) |
| bindings `--write` · `--check` | 348 rendered · in sync |
| `baseline-inventory --check` | in sync |
| r71 `verify.py check` · `selftest` · release-root tests from `E:\tmp` | ok (772 files, extracted root verified against `MANIFEST.sha256`, helper / FFmpeg source / executable identities pinned against `BUILD-RECORD.json`) · 21 / 21 broken copies refused · 3 / 3 (`release_archive_layout` defs resolve beside the executable; `about_notices` every file the notices name is present; `ea_xma_real_vectors` through the bundled helper, no failures) |

## 3. Candidate r71

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20261001-r71\NHL Modding Studio rc-20261001-r71.zip`, SHA-256 `a51b907ac3311a833bef58c07bdf104b70edb4097b5f578914fd3528a8902309`, 772 files, 106 574 597 bytes |
| Executable | `E:\nhlms-review-build-r71-2026-10-01\NHL Modding Studio rc-20261001-r71\nhl-studio.exe`, SHA-256 `1359e186dff2584a304e5bd074f3ecb876c01adadf05be64988db38dd03ddee8` (53 839 360 bytes) |
| Inputs | HEAD `8d60a6d` + 1041 dirty inputs, manifest `2e482c8c…baa2d`, identical after the build; built 20:17:37 → 20:21:08 MDT, `CARGO_INCREMENTAL=0 npm run build -- --no-bundle` |
| defs / frontend / helpers / anchors | `20c3b183…cf8b` (unchanged) / `9738f784…48e1` / `1caa5f4d…f5c4e` (unchanged, r70 helper bundle `dc3533ed…8c21`, FFmpeg source `7138d28c…78a3`) / `170fcc9a…1fb9` (unchanged) |
| Baselines | `INVENTORY.json` `a78c50cc…2aa7` (unchanged from r70) |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder pending; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r71/` (inputs manifests before / after, exe hash, tree manifests, assemble / verify / release-root logs, build record input, r67–r70 preservation manifests) |

## 4. Changed dependencies and focused reruns (on r71)

Changed: onboarding (four actions, optional inputs), project persistence (schema v2, lock, sources,
member records), opening (new route), Pending Edits (build panel; assets-only staged list), Settings
(inputs card, Install button), and the new package / installer routes. Unchanged: database editing
surfaces, codecs, discovery rules, deployment transaction, extraction service, ISO and copied-target
advanced routes.

1. **First pass — NHL 21 PS4**: Start fresh into a project folder with the installation root left empty;
   two or three cell edits; Save project; Pending Edits → Build mod package (include cells; the
   dictionary / textures categories empty); Install the package into an **owned copy** of a NHL 21
   install from Settings → Tools → Install a package; review the plan (the two or three cells, nothing
   else); Install; confirm the copy's `nhl_game.db` differs by exactly those cells; Restore from the
   install record; confirm identical bytes.
2. **Open existing project**: close Studio, reopen via Open existing project (recent list and picker);
   the edits, pending list and last view return; move the project folder and reopen (warning, then
   normal); remove the linked destination and reopen (warning; Deploy says what is missing; relink from
   Settings → Project inputs).
3. **Roster save alone / assets alone**: create a project from a copied roster save only, edit, save,
   reopen; create a project from an extracted assets folder only, replace one texture, confirm the
   database areas say why they are unavailable, deploy to an owned copy of the folder, restore.
4. **Legacy multi-platform cards**: author on Xbox 360 (one cell + one dictionary text), build with
   PS3 and Recompiled declared; install into owned copies of a PS3 (RPCS3-shaped) tree and a Recompiled
   folder; the dictionary text must land in each recipient's own dictionary; restore both. The matrix
   lists what the automated case already covers; the user card is the same shape on real files.
5. **Conflict**: edit the same cell differently in the recipient copy, review → the conflict is named and
   Install is disabled until "Allow replacing values" is ticked and the plan is reviewed again.
6. **Refusals**: a truncated or edited `.nhlmod` is refused at Read package; a package for another title
   is refused at review.
7. **Stale r70 reruns still due** (first screen, deploy, extraction) remain valid on r71 except where the
   first screen changed (now four actions; the inputs are optional).

**Invalidated by a changed dependency:** every r70 result for the first screen and for reopening
(reopen was "create a second project" in r70). **Not invalidated:** the deployment, extraction and
database editing cards, PS4 discovery and roster selection rules (unchanged code, kept suites).

## 5. Preservation

r67 zip `6297cda9…0b4a`, r68 zip `67de2eb2…c5fd`, r69 zip `322b9b9a…5627`, r70 zip `406a864e…b075`, and
the historical-corpus zip: hashes identical before and after this package (`preserve/zips-before.txt` vs
`zips-after.txt`). Extracted trees re-hashed after the build: r67 620 / 620, r68 620 / 620, r69 620 / 620,
r70 772 / 772 identical (`r6x-before.tsv` / `-after.tsv` in the r71 evidence folder). r71 went to new
folders only. The anchors tree hashed `170fcc9a…1fb9` before and after.

## 6. Cleanup

Removed: the first (defective) r71 archive, its extraction and its assembler stage; the selftest scratch
(removed by the selftest); every test's owned temp tree (guards). Kept: r67–r70 bundles and zips, the r71
zip + extracted bundle + assembler `stage` (386 MB), the shared `target\` cache (6.4 GiB), fixtures,
`E:\tmp`. Session scratch (patch scripts, logs, preservation manifests; 785 KB) is under the session's
scratchpad and is not part of the repository. Free space at delivery: E: 125 GiB, H: 126 GiB. No process
of this package is running.

## 7. Remaining gaps and limits

- **Installer texture path**: implemented, no automated case (needs an owned install with a real loose
  texture container; the synthetic case covers the project side only).
- **Not portable in a package** (listed and blocked, never omitted): archive-member replacements,
  native-only outputs (kit clones, outputs without a retained source), structural changes (rows added /
  removed, new dictionary keys), settings-CSV defaults, schedule edits, dictionary growth. These stay
  saved in the project and deploy from it.
- **Package-level fault injection**: `package_apply` / `package_restore` rely on the shared two-phase
  publisher's own fault tests; no mid-write case at the package level.
- **Cross-platform cells** are offered only where the bundled main databases are byte-identical; NHL 15
  PS4 and every PS4-only title are author-platform only. Cross-game installation is refused by design.
- **Undo history** is not persisted across a reopen (the saved files are; the resume warning says so).
- **Dictionary diff** is positional; a re-keyed row is reported as structural rather than as a text edit.
- **Nothing is game-verified**; the desktop app was not run in this package.

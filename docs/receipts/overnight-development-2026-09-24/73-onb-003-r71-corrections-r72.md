# 73 — ONB-003 r71 coordinator corrections: row identity, file roles, coordinated recovery, assets-only Save, asset roots, Legacy qualification (candidate r72)

**Summary.** The r71 review found five defects in the mod package installer and one in project Save;
all six of the coordinator's probes reproduced here before anything was changed and all six pass now
at the coordinator's own assertions. A package edit no longer says "row 0 of the players table": it
names the **player's key values** (and a fingerprint of the row the author started from), and the
installer finds that player in the recipient's own files — a reordered database edits the right
player or refuses, never a stranger. Edits go **only** to the file they were made in: a database-only
package never asks for, touches or lists a roster save (PS4 included); a package that edits both files
needs both and checks each on its own; when several roster saves exist the choice is explicit on every
console and a stale choice is refused. Installing into a save layout that needs an index header shows
that header **in the reviewed plan** and removes it again **inside** the restore transaction, so a
removal that fails rolls the rest back and the record stays restorable; a split or interrupted
installation (or restore) is recorded per file and finished by **Recover**, also from a fresh Studio
after a reopen; a file changed by someone else refuses the whole restore and is left alone. The
project deployment route shares that behaviour. **Save** is valid on an assets-only project (it
reports, accurately, that nothing is left to write) and the staged outputs, their sources and the last
view survive close, reopen and relocation into a package built without any database. Loose textures
are placed from the **platform's asset root** (the PS4 game folder, never the database folder; the
Recompiled `_compiled/` folder), and a wrong folder, a missing extraction, a file that is not a texture
and a changed file each refuse with the reason. Legacy loose textures install across Xbox 360 ↔
Recompiled by the registered profile inheritance and are **refused for PS3** with the exact missing
profile named.

Internal candidate **r72**. r67–r71 are byte-for-byte unchanged (zips and extracted trees, §5).
Nothing was committed, pushed, published, uploaded, launched or installed into a real game; no
original installation or save was opened for writing; every test wrote into a temp tree it owns.
Dispatch: [CORRECTION-HANDOFF.md](../../validation/user-beta-pass-20261001/r71-review/CORRECTION-HANDOFF.md),
review: [REVIEW.md](../../validation/user-beta-pass-20261001/r71-review/REVIEW.md). Corrected matrix:
[onb-003-compatibility-matrix.md](../../validation/onb-003-compatibility-matrix.md). No user check is
marked passed; nothing is game-verified. The user pass records were not touched.

## 1. Findings → behaviour, files, tests

| Finding | Behaviour now | Where | Automated evidence |
|---|---|---|---|
| **R1** cells bound to row numbers | Each `CellOp` carries `keys[]` (the table's metadata key fields with the author's baseline values) and `row_sha256` (the baseline row bytes); edits schema `nhlms-modpkg-edits/2`. The installer resolves the recipient's row by the keys — exactly one match; none ("the player the author edited is not there") or several ("ambiguous") refuse the plan with nothing written. A keyless table falls back to the position only when the recipient's row fingerprint equals the author's. A schema-1 (r71) package inspects but the installer refuses its cells: "bound … to row positions … Rebuild it from its project". Before-value equality never substitutes for identity; allowing conflicts never authorises an identity guess. | `crates/studio-domain/src/mod_package.rs` (`KeyValue`, `CellOp.keys/row_sha256`, `EDITS_SCHEMA` 2 + `EDITS_SCHEMA_V1`, validation), `crates/studio-api/src/registry/package.rs` (`cells_between`, `key_fields`, `row_fingerprint`, `locate_row`, `same_edit`) | probe `coordinator_reordered_player_is_not_edited_by_old_row_number`; `onb003_corrections::r1_missing_or_duplicate_keys_refuse_without_writing`, `r1_a_position_bound_package_from_r71_is_refused_by_the_installer`; `mod_package` 4 / 4 unchanged |
| **R2** roles not enforced | Dependencies come from each operation's declared file: `main_db` → database only, `roster` → roster only, `both` → both required, replayed in each file independently with its own row resolution and before-value check (a conflict names the file; the op applies to neither file unless both pass). No workspace mirroring in the installer. Roster choice on every platform through one rule: the explicit choice must be among the saves found now (stale → blocked), one found → used, several → "choose which one" blocks; a package without roster edits never calls it, lists the roster as *not needed* and ignores a named save (PS4 included). The builder merges a main and a roster measurement into one `both` op only when row identity, field, before and after all agree. | `registry/package.rs` (`cell_targets`, `chosen_roster`, `StagedTarget`, preflight), `app/src/pages/InstallPackage.tsx` (roster choice on every console, "not needed" requirement) | probes `coordinator_main_db_only_does_not_change_recipient_roster`, `coordinator_both_roles_requires_roster`; `r2_a_database_only_package_never_needs_or_touches_a_roster_even_on_ps4_with_one_named`, `r2_both_edits_check_and_apply_each_file_on_its_own_and_roster_choices_are_explicit`; UI `InstallPackage.test.tsx` (not-needed, explicit PS3 choice sent exactly) |
| **R3** restore strands a mixed result | The in-place publisher gained coordinated **removals** (`PlannedRemoval`, `publish_all`, events `BeforeRemove` / `AfterRemoved`): a removal is checked before it enters the recovery set, rolled back by rewriting the prior bytes, classified like a write. The save-index header is an item of the **reviewed plan** (state `create`, the sibling it was cloned from bound by identity and re-checked at Apply), never added at Apply. Restore and recovery share `registry/recovery.rs`: every destination is classified by its **current identity** (installed → put back / remove; prior or absent-as-before → already done; anything else → foreign, whole restore refused, nothing touched), the puts-back and removals run as one step set, and record / item states are read from the destinations afterwards (`committed` / `restore-incomplete` / `split` / `publishing` are recoverable; `rolled-back` / `restored` are final). Hooks variants (`package_apply_with`, `package_restore_with`, `project_deploy_restore_with`) exist for fault injection. The project deployment route uses the same plan item, step set and classification. | `crates/studio-domain/src/in_place_save.rs`, `crates/studio-api/src/registry/recovery.rs` (new), `registry/package.rs`, `registry/project.rs` (`PlannedItem.inline/template`, header item, apply, restore), `InstallRecordDto.recovery`, `DeploymentRecordDto.recovery`, `InstallPackage.tsx` (Recover) | probe `coordinator_restore_can_recover_after_sidecar_removal_failure`; `r3_a_fault_while_applying_rolls_back_and_a_split_apply_is_recovered_by_restore` (fault at the roster write → rolled back; fault after the roster rename with the database's rollback broken → split, recovered by a fresh registry), `r3_an_interrupted_restore_is_retried_and_a_split_restore_resumes_after_reopen` (removal fault → committed, retried; split restore → `restore-incomplete` per item, foreign change refused, resumed after reopen), `r3_the_project_deployment_route_plans_the_header_and_removes_it_inside_the_restore`; domain `in_place_save::tests::removals_roll_back_with_the_writes_and_are_classified`; UI Recover button → `package_restore` |
| **R4** assets-only Save fails | `project_save` on a project without an edit session returns `nothing_to_save` (the staged outputs, kept sources and intent are written when each route runs); `summary` returns the synthetic assets-only summary instead of a workspace error; Pending Edits offers **Save project** for assets-only projects and shows the accurate result. | `registry/project.rs` (`project_save`, `asset_only_summary`), `registry.rs` (`summary`), `app/src/pages/PendingEdits.tsx` | probe `coordinator_ps4_texture_package_uses_game_asset_root` (passes Save, builds, installs); `r4_assets_only_save_reopen_relocate_and_build_without_a_database` (save → close → relocate → reopen in a fresh registry → staged file, kept source, last view → package payload = the authored bytes); UI `PendingEdits.test.tsx` (2) |
| **R5** PS4 textures resolved under the database folder | `package::asset_roots(report, platform)`: PS4 → the matched game folder; Xbox 360 / PS3 → the install folder; Recompiled → `_compiled/` when present, then the install folder; nothing recognised → blocked ("wrong folder?"). The recipient's own file at the same relative path is required (missing → "extract … or choose the right game folder"), must be a texture container the writer accepts, and must be the file the author replaced (else a conflict). The project deployment route addresses its overlay outputs from the same root. | `registry/package.rs` (`asset_roots`, textures loop), `registry/project.rs` (deploy plan root) | probes `coordinator_ps4_texture_package_uses_game_asset_root`, `coordinator_ps4_texture_with_database_uses_game_asset_root`; `r5_a_native_ps3_texture_installs_into_the_asset_root_and_bad_destinations_refuse` (synthetic RW4ps3 logo in an X-C BIGF package: wrong root, missing extraction, non-texture file, changed target, install + decoded pixels + exact restore) |
| **R6** qualification | Textures across platforms only where the title's loose-asset layout is registered as shared: NHL Legacy Xbox 360 ↔ Recompiled (`assets-logos.toml` Recompiled profile inherits Xbox 360 by reference, Q47 / C0-AS5). NHL Legacy PS3: unsupported — "no registered loose-asset profile … path families are unverified"; every other pair unsupported — "registered per platform and not verified identical". The build refuses a declared platform with that reason; every Recompiled placement is still verified against the recipient's own file. | `registry/package.rs` (`shared_loose_layout`, `target_support`) | `r6_an_xbox_texture_installs_on_a_recompiled_tree_and_is_refused_for_ps3_with_the_reason` (synthetic RW4xb2 texlib; installed under `_compiled/rendering/logo/…`, restored exactly; PS3 build refused) |

### 1.1 Reproduction before the change

The coordinator's file was installed verbatim as `crates/studio-api/tests/onb003_coordinator_probe.rs`
and run first: **0 passed, 6 failed**, each at the reported assertion / error
(`scratchpad/probe-repro-before.log`). It is kept as a permanent test; after the corrections it
runs **10 / 10** (the six probes plus the four original package cases it carries).

### 1.2 Data versioning

- Package edits document: `nhlms-modpkg-edits/1` → `/2`. Version 1 packages still inspect
  (`PackageInspectDto.edits_schema`); the installer refuses their cells explicitly. No silent
  reinterpretation.
- Project manifest: unchanged (`nhlms-project/2`); edits are measured from the project's baseline and
  working files at build time, so no project migration is needed.
- Installation / deployment records: schema unchanged; the state vocabulary gained
  `restore-incomplete`; `restorable` now covers every recoverable state and `recovery` flags an
  interrupted one.

## 2. Checks

| Suite | Result |
|---|---|
| `onb003_coordinator_probe` (the coordinator's six probes, verbatim) | **6 / 6** (10 / 10 with the four original cases in the file) |
| `onb003_corrections` (new, 10 cases: R1 ×2, R2 ×2, R3 ×3, R4, R5, R6) | 10 / 10 |
| `mod_package` 4 · `project_inputs` 4 · `project_resume` 5 · `project_workspace` 7 | all green (one wording expectation updated: a restored deployment now says "nothing to restore or recover") |
| `studio-domain --lib` (incl. the new publisher removals case) | 258 / 258 (2 ignored, pre-existing) |
| `studio-api` all tests (`--no-fail-fast`) | 61 binaries, 200 passed, 0 failed, 53 ignored (fixture-gated, pre-existing) |
| frontend `tsc --noEmit` · vitest (all; `InstallPackage.test.tsx` +3, `PendingEdits.test.tsx` new 2) | clean · 296 / 296 (50 files) |
| bindings `--write` · `--check` | 4 written, 344 unchanged · in sync (194 roots, 348 files) |
| `baseline-inventory --check` | in sync |
| r72 `verify.py check` · `selftest` · release-root tests from `E:\tmp` | ok (772 files, extracted root verified against `MANIFEST.sha256`, helper / FFmpeg source / executable identities pinned against `BUILD-RECORD.json`) · 21 / 21 broken copies refused · 3 / 3 (`release_archive_layout`, `about_notices`, `ea_xma_real_vectors` through the bundled helper) |

## 3. Candidate r72

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20261001-r72\NHL Modding Studio rc-20261001-r72.zip`, SHA-256 `e56f6a0f58d690f159a94e18ff234a8e30ff3ca6b00cff2d229a7f7f05421f0b`, 772 files, 106 608 430 bytes |
| Executable | `E:\nhlms-review-build-r72-2026-10-01\NHL Modding Studio rc-20261001-r72\nhl-studio.exe`, SHA-256 `13c5786b937d1eb1d8387dc5ff997984ff2f83577e28dbf7c1a3cecb7f0200c4` (53 929 472 bytes) |
| Inputs | HEAD `8d60a6d` + 1045 dirty inputs, manifest `8af16c14…aa40`, identical after the build; built 23:33:23 → 23:36:28 MDT, `CARGO_INCREMENTAL=0 npm run build -- --no-bundle` |
| defs / frontend / helpers / anchors | `20c3b183…cf8b` (unchanged) / `5092dde3…6846` / `1caa5f4d…f5c4e` (unchanged, r71 helper bundle `dc3533ed…8c21`, FFmpeg source `7138d28c…78a3`) / `170fcc9a…1fb9` (unchanged) |
| Baselines | `INVENTORY.json` `a78c50cc…2aa7` (unchanged) |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder pending; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r72/` |

## 4. Changed dependencies and focused reruns (on r72)

Changed: the package format (edits schema 2), the installer (identity, roles, roots, recovery), the
installer and Pending Edits pages, project Save for assets-only projects, deployment plan / apply /
restore (header item, recovery), the in-place publisher (removals — additive; every existing
publisher case unchanged). Unchanged: database editing surfaces, codecs, discovery rules, the
extraction service, the ISO and copied-target routes.

**Invalidated:** every r71 package that carries database cell edits (edits schema 1; the installer
refuses those and asks for a rebuild from the project — a schema-1 package with dictionary or texture
edits only still installs); the r71 user reruns 1, 4, 5 and 6 of receipt 72 (package → install,
Legacy cards, conflict, refusals); the **deployment cards** of receipts 71 / 72 (deployment planning now
lists the save-index header, apply reads it from the plan and restore runs through the shared
recovery) — rerun them. **Not invalidated:** reruns 2 and 3 (Open existing; roster-only / assets-only
creation), the extraction cards, every non-onboarding database card. *(Wording corrected on the r72
coordinator review, 2026-10-02; no historical user result was changed.)*

1. **NHL 21 PS4 first pass** (receipt 72 §4.1) on r72: build from a saved project, install into an
   owned copy; the plan lists the database only and says the roster is *not needed*; Restore.
2. **Reordered recipient**: install the same package into an owned copy whose player rows were
   reordered (or where the edited player was removed) — the right player changes, or the plan
   names the player it cannot find; nothing is written.
3. **Legacy cards** (Xbox 360 author → owned PS3 tree and owned Recompiled content tree): the PS3
   plan lists `nhlng.db`, `SYS-DATA` and the dictionary; the Recompiled plan also lists the
   *Save index header* as **created**; Restore removes it again; a second roster save in the PS3
   tree makes the choice explicit.
4. **Recover**: interrupt an installation or a restore (an open handle on the header is the
   reproduced case) — the record shows *restore-incomplete* / *committed* and **Recover** finishes it
   once the handle is released; a file edited by hand in between is refused and left alone.
5. **Assets-only Save**: stage a texture in an assets-only project, Save (says nothing left to
   save), close, move the folder, Open existing project, Build mod package.
6. **Textures**: PS3 Legacy logo package into an owned extracted tree (wrong folder, not-extracted
   and changed-file refusals), Xbox 360 → Recompiled logo under `_compiled/`; a PS3 declaration on an
   Xbox 360 texture package is refused with the missing-profile reason.

## 5. Preservation

r67 zip `6297cda9…0b4a`, r68 `67de2eb2…c5fd`, r69 `322b9b9a…5627`, r70 `406a864e…b075`,
r71 `a51b907a…2309`, historical-corpus zip `716c69cb…c570`: hashes identical before and after this package (`preserve/zips-before-r72.txt` vs `zips-after-r72.txt`). Extracted trees
re-hashed after the build: r67 620 / 620, r68 620 / 620, r69 620 / 620, r70 772 / 772, r71 772 / 772 identical (`r6x-before.tsv` / `-after.tsv`, `r71-before.tsv` / `-after.tsv` in the r72 evidence folder). r72 went to new folders only.

## 6. Cleanup

Removed: the selftest scratch (by the selftest), every test's owned temp tree (guards), the
installer's staging folders (removed at the end of every preflight). Kept: r67–r71 bundles and zips,
the r72 zip + extracted bundle + assembler `stage`, the shared `target\` cache, fixtures, `E:\tmp`.
Session scratch (patch scripts, logs, preservation manifests) is under the session's scratchpad and
is not part of the repository. The coordinator's probe file is now a repository test. No process of
this package is running.

## 7. Remaining blockers and skips (explicit)

- **Loose textures on NHL Legacy PS3**: no registered loose-asset profile — unsupported by rule,
  never attempted. Closing it needs the PS3 Legacy logo / jersey path families registered with
  provenance (research), then the same verified placement this build does for Recompiled.
- **Cross-platform textures for every other title pair**: path families registered per platform,
  not verified identical — unsupported by rule.
- **Archive-member replacements** (PS3 board pairs, `.big` members), native-only outputs,
  structural changes, settings-CSV, schedule and dictionary-growth edits: not portable; listed and
  blocked, never omitted. The Legacy "roster plus logos / uniforms" scenario is complete at the
  automated level for Xbox 360 ↔ Recompiled loose files and blocked for PS3 textures and for every
  archive-member route.
- **Crash between two renames**: recovery classifies from the destinations' identities when the
  record is next opened; no process crash is simulated (faults are injected at the publisher's
  events).
- **Cross-year conversion** and automatic omission: deferred, untouched.
- Nothing is game-verified; the desktop app was not run in this package.

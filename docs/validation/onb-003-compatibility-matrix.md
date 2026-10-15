# ONB-003 — operation / target compatibility matrix (candidate r73, 2026-10-02)

What the mod package builder can carry and what the built-in installer can apply, per operation
category and per platform, **as measured in this build**. Evidence levels follow CLAUDE.md:
*implemented* (code exists), *automated-tested* (a named test executed and asserted it),
*game-verified* (none — no cell anywhere is game-verified), *blocked*, *missing-fixture*.
Nothing here is runtime acceptance; every test wrote only into a temp tree it owned.

This revision corrects the r71 matrix after the coordinator review
(`docs/validation/user-beta-pass-20261001/r71-review/`): r71 bound database edits to row
positions, spread edits across file roles, left a created sidecar outside the restore
transaction, refused Save on assets-only projects and searched the PS4 database folder for
assets. Each correction is named with its automated case. The r72 review (`r72-review/`) added one
dependency correction: a roster-only package installs from the save location alone, with the title's
schema metadata from the shipped baseline store (R72-R1).

## 1. Portable edit categories

| Category | In a project | In a package | Installer | Evidence |
|---|---|---|---|---|
| Database cells (main database / roster save) | measured baseline ↔ saved working copies (`pending::pending_changes`), named by table / entity / field with before + after; **bound to the row's key values** (metadata key fields, the author's baseline values) and a fingerprint of the baseline row; `main_db`, `roster`, or `both` only when the same row identity, field and values were measured in both files | `edits.json` schema `nhlms-modpkg-edits/2` `cells[]` with `keys[]` + `row_sha256`; a schema-1 (r71) package still inspects; the installer refuses it **when it carries cell edits** ("rebuild from the project") and installs a dictionary- or texture-only one | each file role is staged and replayed **on its own** (no mirroring): the row is resolved by its key values in that file (exactly one match; none or several → refused), the field's current value must equal the package's before (else a conflict naming the file; blocked unless allowed; already-after = skipped); a `both` edit applies to neither file unless both resolve and check | automated-tested: `onb003_corrections` R1 (`r1_missing_or_duplicate_keys_refuse_without_writing`, `r1_a_position_bound_package_from_r71_is_refused_by_the_installer`), R2 (`r2_a_database_only_package_never_needs_or_touches_a_roster_even_on_ps4_with_one_named`, `r2_both_edits_check_and_apply_each_file_on_its_own_and_roster_choices_are_explicit`); coordinator probes `coordinator_reordered_player_is_not_edited_by_old_row_number`, `coordinator_main_db_only_does_not_change_recipient_roster`, `coordinator_both_roles_requires_roster`; `mod_package` (NHL 21 PS4; Legacy X360 → PS3 / Recompiled / X360; NHL 15) |
| Dictionary texts (key-based) | positional row diff of baseline ↔ saved dictionary; re-keyed / added rows are *structural* | `dictionary[]` (key, before, after) | `apply_retext` on the recipient's own dictionary after checking the current text; missing key = blocked | automated-tested: `mod_package` Legacy `TXT_LEAGUE_NORWAY` X360 author → PS3 (different dictionary file) and Recompiled |
| Loose texture replacements | staged overlay output **plus the source image kept under `sources/`** (texture replace, nameplate) | `textures[]` + `payloads/<sha>.png` | the recipient's own file at the same relative path under its **platform asset root** (§2b) is re-encoded from the payload through `plan_texture_replace` / `replace_texture_in_file`; the author's prior identity checked (conflict); a missing file, an unrecognised installation, a non-texture file or a writer refusal block with the reason | automated-tested: R5 `r5_a_native_ps3_texture_installs_into_the_asset_root_and_bad_destinations_refuse` (synthetic RW4ps3 logo package in an X-C BIGF: wrong root, missing extraction, incompatible file, changed target, install + decode + restore), R6 `r6_an_xbox_texture_installs_on_a_recompiled_tree_and_is_refused_for_ps3_with_the_reason` (synthetic RW4xb2 texlib); coordinator probes `coordinator_ps4_texture_package_uses_game_asset_root`, `coordinator_ps4_texture_with_database_uses_game_asset_root` (PNG); project side `project_inputs` assets-only |
| Save-index sidecar (content-indexed save layouts) | — | — | part of the **reviewed plan** when the roster is written into a content tree whose index needs a header (state `create`, the sibling it is cloned from bound by identity and re-checked at Apply); removed again **inside** the restore step set | automated-tested: R3 `r3_a_fault_while_applying_rolls_back_and_a_split_apply_is_recovered_by_restore`, `r3_an_interrupted_restore_is_retried_and_a_split_restore_resumes_after_reopen`, `r3_the_project_deployment_route_plans_the_header_and_removes_it_inside_the_restore`; coordinator probe `coordinator_restore_can_recover_after_sidecar_removal_failure` |
| Archive-member replacements (PS4 / Xbox `.big`, PS3 board pairs) | recorded as semantic `member_edits` (archive, member, scope, source) at save | **not packageable** — reported as a blocker, build refuses while included | no installer adapter in this build | blocked (explicit) |
| Native-only outputs (kit clones, outputs without a retained source) | staged overlay output | not packageable (exact-target only; deploy from the project) | — | blocked (explicit) |
| Structural changes (rows added / removed, new dictionary keys) | measured and listed | not packageable | — | blocked (explicit) |
| Settings-CSV defaults, schedule edits, dictionary growth | saved in the project (unchanged ONB-002 routes) | not inventoried as portable ops in this build | — | not packaged (saved and deployable from the project only) |

A build refuses — it never omits silently — when a non-empty category is neither included nor
explicitly excluded, when an included category is blocked, or when a declared platform is
unsupported for an included category.

## 2. Per-platform support rule (same title only)

| Category | Author's platform | Other platform of the same title |
|---|---|---|
| cells | supported (rows by key values) | supported **only** when the bundled baseline main database of that platform is byte-identical to the author's baseline (measured from `baselines/INVENTORY.json`: same tables, key fields and codecs); otherwise unsupported with both hashes named. Roster-save-only projects: author's platform only |
| dictionary | supported | supported when that platform has a bundled dictionary; verified key by key at install |
| textures | supported | supported only where the title's loose-asset layout is **registered as shared**: NHL Legacy Xbox 360 ↔ Recompiled (the Recompiled logo profile inherits the Xbox 360 profile by reference, Q47 / C0-AS5, `defs/adapters/asset-profiles/assets-logos.toml`); every placement is still verified against the recipient's own file at install. NHL Legacy PS3: unsupported — no registered loose-asset profile, path families unverified. Every other pair: unsupported — path families registered per platform, not verified identical |

Measured identities (`INVENTORY.json`, HEAD `8d60a6d`): every NHL 08–15 title's `nhlng.db` is
byte-identical between Xbox 360 and PS3; NHL Legacy's `nhlng.db` (`105d2186…`) is shared by
Xbox 360, PS3 and Recompiled; NHL 15 PS4 `nhl_game.db` (`ae9d1d56…`) differs from PS3/X360
(`028ae06a…`). NHL Legacy and NHL Legacy Recompiled are treated as one title on two runtimes
(`same_title`); every other title pair is distinct and cross-game installation is refused.

### 2a. File roles and dependencies (R2)

| Package edits | Needs the main database | Needs a roster save | Roster choice |
|---|---|---|---|
| `main_db` only | yes | **no** — no save is listed, chosen or touched (PS4 included, even when a roster is named) | — |
| `roster` only | **no** — no game installation folder is needed (R72-R1): the save-data location (or the roster save itself) is the destination; the title's schema metadata comes from the shipped baseline store, verified by hash (`baselines/INVENTORY.json`, `requirements[].role = "schema"`), never from a file the recipient must copy. PS3 keeps its saves inside the RPCS3 folder, so that folder stays the location to choose. No store / no bundled schema / a schema that does not verify → blocked with the reason, nothing written | yes | explicit when several writable roster saves are found (PS4: across profile / title folders of the save-data location); a stale choice (not among the saves found now) blocks; a missing roster blocks; a save that does not open as this title's roster with the schema refuses by name |
| `both` | yes | yes — a missing roster blocks the **whole** plan | as above |

### 2b. Asset roots (R5)

| Platform | Root the installer addresses loose textures from |
|---|---|
| PS4 | the matched game folder (`game_dir`), never the database folder |
| Xbox 360 / PS3 | the resolved install folder |
| Recompiled | `_compiled/` beside the executable when it exists, else the install folder |
| no installation recognised | blocked ("wrong folder?"); nothing located or written |

The project deployment route addresses its overlay outputs from the same roots.

## 3. The Legacy trio — qualified independently

| Recipient | Owned synthetic base | Cells + dictionary (automated, `legacy_xbox_package_installs_on_ps3_and_recompiled_owned_bases`) | Loose textures |
|---|---|---|---|
| Xbox 360 (author) | loose folder with the X360 Legacy anchors + explicit save folder | 1 cell applied to `nhlng.db`, mirrored into the roster container by the package's own `both` edit | author platform (synthetic RW4xb2 texlib, `r6_…`) |
| PS3 | RPCS3-shaped tree from the PS3 Legacy anchors (`BLUS31540`) | 1 cell in `nhlng.db` and in `SYS-DATA` (each resolved by key in its own file), dictionary key re-encoded in the **PS3** dictionary; everything else byte-identical; restore → identical | **unsupported** — no registered loose-asset profile for NHL Legacy PS3; a build declaring PS3 is refused with that reason (`r6_…`) |
| Recompiled | loose game folder from the X360 Legacy anchors + explicit saves, with a content-indexed save tree | same as PS3 on the Recompiled layout, the save-index header created inside the plan and removed inside the restore (`r3_…`, coordinator probe); restore → identical | supported by registered inheritance; installed under `_compiled/` and restored exactly (`r6_…`) |

These are synthetic / copied-fixture results. No game was launched; no original install or save
was opened for writing; nothing is game-verified. The requested Legacy roster-plus-logos /
uniforms multi-platform scenario is therefore **complete for Xbox 360 ↔ Recompiled** at the
automated level (cells, dictionary, loose textures) and **blocked for PS3 loose textures** on the
missing profile; the archive-member path (PS3 board pairs, `.big` members) remains blocked (§1).

## 4. Input modes (ONB-003 §1)

| Mode | Opens | Independent work | Gated | Evidence |
|---|---|---|---|---|
| roster save only (bundled schema metadata) | `project_create_import` with `roster_save` | roster edits, save into project, package (author platform); **install that package into another save location with no game installation** | main-database-only areas; deployment until a save destination is linked | `roster_save_alone_opens_edits_saves_and_reopens`; coordinator probe `coordinator_roster_only_package_needs_no_game_installation`; `r72_r1_a_roster_only_package_installs_from_the_save_location_alone_with_bundled_schema` (store outside the checkout), `r72_r1_schema_lookup_failures_and_bad_saves_refuse_precisely_without_writing`, `r72_r1_roster_only_multi_save_choice_stale_choice_and_changed_target`, `r72_r1_a_ps4_roster_only_package_installs_into_the_profile_and_title_save_folder`; UI `InstallPackage.test.tsx` (empty game root + explicit save submitted) |
| main database only, no installation | `main_db` only, root empty | database edits, save, package | deployment until relinked | `main_database_alone_without_an_installation` |
| extracted assets only | `assets_dir` only | loose texture routes (overlay staging, source kept), **Save (accurate no-op)**, close / reopen / relocate, package without a database, deploy to the assets folder, restore | every database route (named reason) | `extracted_assets_alone_stage_into_the_project_and_gate_database_routes`, `r4_assets_only_save_reopen_relocate_and_build_without_a_database`, UI `PendingEdits.test.tsx` |
| combination | any of the above together | all of the above | — | `a_combination_keeps_every_input_role_and_a_fresh_project_needs_no_installation` |
| fresh, no installation | bundled baseline, root empty, relink later | database + roster edits; deployment after `project_relink` | — | same case |

## 5. Recovery (R3)

| Record state | Meaning | Restore / Recover |
|---|---|---|
| `committed` | every destination holds the installed bytes | Restore: puts-back and sidecar removals as one step set; a failure rolls the step set back and the record stays `committed` (retry) |
| `rolled-back` | the apply failed and every destination verified back | nothing to do |
| `split` | the apply failed and a rollback also failed; per item `installed` / `prior` / `unknown` | Recover: every item still holding the installed bytes is put back |
| `restore-incomplete` | a restore's step set split; per item states | Recover: resumes exactly where it stopped |
| `publishing` | the process stopped between writing the record and the outcome | Recover: classified by each destination's current identity |
| `restored` | every destination holds its prior state | nothing to do |

A destination holding anything other than the installed bytes or its prior state is a foreign
change: the whole operation refuses and nothing is touched. The project deployment route uses the
same classification and step set.

## 6. Not covered in r73 (honest gaps)

- Loose textures on NHL Legacy PS3 (no registered profile) and across the Xbox 360 / PS3 pairs of
  the other titles (path families not verified identical): unsupported by rule, never attempted.
- Archive-member, settings-CSV, schedule, dictionary-growth and native-only edits are not
  portable in this build (listed and blocked, never omitted).
- Recovery after a crash **between two renames** is classified from identities when the record
  is next opened; no crash is simulated by a test (process faults are injected at the publisher's
  events).
- No game-verified cell. No run of the desktop app in this package.

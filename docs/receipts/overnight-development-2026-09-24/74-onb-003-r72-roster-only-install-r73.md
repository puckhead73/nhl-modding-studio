# 74 — ONB-003 r72 coordinator correction: roster-only package installation without a game installation (candidate r73)

**Summary.** The r72 review accepted the five r71 fixes and found one dependency still missing: a
package that edits only the roster save still demanded a game installation folder and took the
schema metadata from that installation. Now the installer works out what a package needs from the
package itself before any path is checked. A roster-only package installs from the **save-data
location alone** (or from the chosen roster save itself): no game folder, no main database, and
the title's schema metadata comes from Studio's **shipped baseline store**, verified by hash — the
recipient never copies a schema file by hand. When the store is missing, has no schema for that
title and console, or holds a file that does not verify, the plan refuses with that exact reason
and writes nothing; a save that is not this title's roster refuses by name. The explicit choice
among several roster saves and the refusal of a stale choice are unchanged, on every console
including PS4's profile / title save folders. A package that also edits the database, the
dictionary or textures still needs the game installation. The installer page lets the review be
submitted with an empty game folder and an explicit save for such a package and says the folder is
optional. Receipt 73 and the matrix were corrected as the review asked: the deployment cards need
reruns (deployment planning, header publication and restore changed), and the schema-1 refusal
applies to r71 packages that carry cell edits, not to every old package.

Internal candidate **r73**. r67–r72 are byte-for-byte unchanged (zips and extracted trees, §5).
Nothing was committed, pushed, published, uploaded, launched or installed into a real game; no
original installation or save was opened for writing; every test wrote into a temp tree it owns.
The user pass records were not touched. Dispatch:
[r72-review/CORRECTION-HANDOFF.md](../../validation/user-beta-pass-20261001/r72-review/CORRECTION-HANDOFF.md),
review: [REVIEW.md](../../validation/user-beta-pass-20261001/r72-review/REVIEW.md). Matrix:
[onb-003-compatibility-matrix.md](../../validation/onb-003-compatibility-matrix.md). No user
check is marked passed; nothing is game-verified.

## 1. R72-R1 → behaviour, files, tests

| Aspect | Behaviour now | Where |
|---|---|---|
| Dependencies first | `needs_main` / `needs_roster` / `needs_installation` are derived from the inspected package's operations before any path is validated. An empty game folder is accepted only for a roster-only package (every edit `roster`, no dictionary, no texture); otherwise "choose the destination game installation first". | `crates/studio-api/src/registry/package.rs` (`package_preflight_with`) |
| Save-only destination | With no game folder, the save-data location (or the folder of the named roster save; on PS4 the save-data root above `<profile>/<title>/<save>`) is what discovery scans; the roster choice goes through the same rule as before (explicit choice must be among the saves found now; one → used; several → "choose which one"). PS3 keeps its saves inside the RPCS3 folder, so a roster-only package there asks for that folder, never for a game. | `package.rs` (`save_path_owned`, `chosen_roster`) |
| Bundled schema | A roster-only package takes `nhlng-meta.xml` / `nhl_game-meta.xml` from the baseline store's `meta` component for the exact title and console, read through `read_verified` (length + SHA-256). The plan lists it as requirement `schema` ("bundled nhlng-meta.xml (verified-vanilla, 6c335a62…)"). Refusals name the cause: no store ("no bundled schema metadata is available in this installation of Studio"), no component for that title / console, a file that does not verify. A package that edits the database keeps using the installation's own metadata. | `package.rs`; `crates/studio-api/src/registry.rs` (`set_baselines`, `baselines()`: the shipped store set by the app, the checkout in development); `crates/studio-api/src/commands.rs` (`package_preflight` passes `resolve_baselines`) |
| Bad saves | The staged roster session names the save, the title, the console and the schema source when it does not open ("does not open as a NHL Legacy roster save with the xbox schema (bundled …)"); nothing is applied. | `package.rs` |
| UI | The game folder is optional for a roster-only package (hint + note); detection and **Review installation** run with an empty folder once the save-data location is given; the database / dictionary lines are replaced by "not needed by this package"; mixed packages keep requiring the folder. | `app/src/pages/InstallPackage.tsx` |

### 1.1 Reproduction before the change

The coordinator's probe was installed verbatim as `crates/studio-api/tests/r72_coordinator_probe.rs`
and run first: **0 passed, 1 failed** ("choose the destination game installation first"; the save
folder as root: "no nhlng-meta.xml found"). It is kept as a permanent test and passes now (both
forms installable, recipient untouched by preflight).

### 1.2 New automated cases (`onb003_corrections`, +4)

| Case | What it proves |
|---|---|
| `r72_r1_a_roster_only_package_installs_from_the_save_location_alone_with_bundled_schema` | Legacy Xbox 360 roster-only package → owned save folder, **no game anywhere**; the registry's store is a copy of the inventory plus the one schema file in a temp folder (**outside the checkout**); requirements `main_db` not-needed, `schema` found (bundled, hash named), `roster_save` found; the only item is the roster; preflight writes nothing; the roster save itself as the only path also works; exactly the two intended cells differ in the native save; no database is created; exact restore |
| `r72_r1_schema_lookup_failures_and_bad_saves_refuse_precisely_without_writing` | no store → "no bundled schema metadata is available"; inventory without the file → "does not verify"; wrong bytes at the inventoried path → "does not verify"; a malformed save → refused by name; an NHL 10 roster offered to a Legacy package → not installable, untouched; a mixed package with an empty game folder → "choose the destination game installation first"; the recipient roster's hash unchanged throughout |
| `r72_r1_roster_only_multi_save_choice_stale_choice_and_changed_target` | two saves → "choose which one"; a stale choice → "isn't one of the roster saves"; the explicit choice is the only save written; a save edited elsewhere → a conflict naming the roster; exact restore |
| `r72_r1_a_ps4_roster_only_package_installs_into_the_profile_and_title_save_folder` | NHL 17 PS4 roster-only package into an owned shadPS4-shaped tree `<savedata>/1/<CUSA…>/RosterCustom0/` with no game folder; only `DATA` changes (`INDEX`, `NAME`, `sce_sys/*` untouched); exactly two cells; exact restore |

UI: `InstallPackage.test.tsx` +2 — an actual submission with an empty game root, the save-data
location and an explicit save (`detect_install` with `root: ""`, `package_preflight` with
`root: ""` and `rosterSave`, Install sent); a package that also edits the database keeps the
Review button disabled without the game folder and never detects.

### 1.3 Documentation corrections (handoff item 4)

- Receipt 73 §4: the deployment cards of receipts 71 / 72 **are** invalidated (deployment planning,
  header publication and restore changed) and need reruns; the schema-1 refusal applies to r71
  packages that carry cell edits (a dictionary- or texture-only schema-1 package still installs).
  Historical user results untouched.
- Matrix: the same precision for schema-1 packages; the roster-only row (§2a) and input mode (§4)
  now describe the save-only installation and its evidence.

## 2. Checks

| Suite | Result |
|---|---|
| `r72_coordinator_probe` (the coordinator's probe, verbatim) | **1 / 1** (5 / 5 with the four package cases it carries) |
| `onb003_coordinator_probe` · `onb003_corrections` | 10 / 10 · 14 / 14 (10 retained + 4 new) |
| `mod_package` 4 · `project_inputs` 4 · `project_resume` 5 · `project_workspace` 7 | all green |
| `studio-domain --lib` | 258 / 258 (2 ignored, pre-existing) |
| `studio-api` all tests (`--no-fail-fast`) | 62 binaries, 209 passed, 0 failed, 53 ignored (fixture-gated, pre-existing) |
| frontend `tsc --noEmit` · vitest (all; `InstallPackage.test.tsx` +2) | clean · 298 / 298 (50 files) |
| bindings `--check` (no DTO change) · `baseline-inventory --check` | in sync · in sync |
| r73 `verify.py check` · `selftest` · release-root tests from `E:\tmp` | ok (772 files, extracted root verified against `MANIFEST.sha256`, helper / FFmpeg source / executable identities pinned against `BUILD-RECORD.json`) · 21 / 21 broken copies refused · 3 / 3 (`release_archive_layout`, `about_notices`, `ea_xma_real_vectors` through the bundled helper) |

## 3. Candidate r73

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20261002-r73\NHL Modding Studio rc-20261002-r73.zip`, SHA-256 `2d29e3bdf4bdac5dbe2a7327c273c4cd1fe9978fe1342b3292c92abe41d914e9`, 772 files, 106 616 660 bytes |
| Executable | `E:\nhlms-review-build-r73-2026-10-02\NHL Modding Studio rc-20261002-r73\nhl-studio.exe`, SHA-256 `e35a68c9cf268861f0f7f16835d909e630aad650fda6afca9e6a919108b9bc7a` (53 940 736 bytes) |
| Inputs | HEAD `8d60a6d` + 1046 dirty inputs, manifest `af14e64d…a342`, identical after the build; built 2026-10-02 01:33:59 → 01:36:52 MDT, `CARGO_INCREMENTAL=0 npm run build -- --no-bundle` |
| defs / frontend / helpers / anchors | `20c3b183…cf8b` (unchanged) / `de73fd88…6ce0` / `1caa5f4d…f5c4e` (unchanged, r72 helper bundle `dc3533ed…8c21`, FFmpeg source `7138d28c…78a3`) / `170fcc9a…1fb9` (unchanged) |
| Baselines | `INVENTORY.json` `a78c50cc…2aa7` (unchanged; it is what the installer now reads the schema from) |
| Record | `internal-review`, `release_ready: false`; blockers unchanged (MIT holder pending; 53 artwork files, rights unrecorded) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r73/` |

## 4. Changed dependencies and focused reruns (on r73)

Changed: the installer's dependency derivation and schema source for roster-only packages, the
installer page's destination step. Unchanged: everything else of r72 (identity binding, roles,
recovery, asset roots, deployment, extraction, database editing).

**Invalidated:** the r72 user reruns 1 and 3 of receipt 73 where a roster-only package is involved
(the destination step changed). **Not invalidated:** r72 reruns 2, 4, 5, 6 (reordered recipient,
Recover, assets-only Save, textures) and the deployment reruns receipt 73 now asks for.

1. **Roster-only, no game**: author from a roster save alone (Legacy Xbox 360), build, Install
   with the game folder left empty and the save-data location given; the plan lists *main db · not
   needed*, *schema · found (bundled …)*, the roster; Install; Restore.
2. **Several saves**: two roster saves in the location → the choice is required; the unchosen
   save's hash is unchanged after Install.
3. **PS4**: the same with a shadPS4 save-data folder (profile / title levels), no game folder.
4. **Mixed package**: a package with database edits keeps asking for the game folder.
5. **Deployment reruns** (receipt 73 correction): Deploy / Restore of a project into an owned
   Recompiled content tree — the plan lists the save-index header, Restore removes it.

## 5. Preservation

r67 zip `6297cda9…0b4a`, r68 `67de2eb2…c5fd`, r69 `322b9b9a…5627`, r70 `406a864e…b075`,
r71 `a51b907a…2309`, r72 `e56f6a0f…1f0b`, historical-corpus zip `716c69cb…c570`: hashes identical before and after this package (`preserve/zips-before-r73.txt` vs `zips-after-r73.txt`).
Extracted trees re-hashed after the build: r67 620 / 620, r68 620 / 620, r69 620 / 620, r70 772 / 772, r71 772 / 772, r72 772 / 772 identical (`r6x-before.tsv` / `-after.tsv`, `r7x-…` in the r73 evidence folder). r73 went to new folders only.

## 6. Cleanup

Removed: the selftest scratch (by the selftest), every test's owned temp tree (guards), the
installer's staging folders. Kept: r67–r72 bundles and zips, the r73 zip + extracted bundle +
assembler `stage`, the shared `target\` cache, fixtures, `E:\tmp`. Session scratch is under the
session's scratchpad and is not part of the repository. The coordinator's r72 probe file is now a
repository test. No process of this package is running.

## 7. Remaining blockers and skips (explicit, unchanged from receipt 73 except as noted)

- Loose textures on NHL Legacy PS3 (no registered profile) and across every other platform pair:
  unsupported by rule.
- Archive-member, native-only, structural, settings-CSV, schedule and dictionary-growth edits:
  not portable; blocked, never omitted.
- A roster-only package on **PS3** still needs the RPCS3 folder, because that is where PS3 saves
  live; it does not need the game's database inside it.
- No process crash is simulated; cross-game / cross-year conversion deferred; nothing is
  game-verified; the desktop app was not run in this package.
